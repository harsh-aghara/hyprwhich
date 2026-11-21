// gui.rs: Production-Grade Wayland Layer-Shell HUD & Action Palette for Hyprland

use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Command;
use std::rc::Rc;

use gdk::keys::constants as keys;
use gdk::ModifierType;
use gtk::prelude::*;
use gtk::{
    Align, Box, Button, Entry, Label, ListBox, ListBoxRow, Orientation, PolicyType,
    ScrolledWindow, SelectionMode, StyleContext, Window, WindowPosition, WindowType,
};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::{expand_hypr_cmd, Keybind};
use crate::theme::ThemePalette;

pub fn fuzzy_score(pattern: &str, text: &str) -> Option<i32> {
    let p_chars: Vec<char> = pattern.to_lowercase().chars().collect();
    let t_chars: Vec<char> = text.to_lowercase().chars().collect();
    if p_chars.is_empty() {
        return Some(0);
    }
    if t_chars.is_empty() || p_chars.len() > t_chars.len() {
        return None;
    }

    // Subsequence presence check
    let mut p_idx = 0;
    for &tc in &t_chars {
        if tc == p_chars[p_idx] {
            p_idx += 1;
            if p_idx == p_chars.len() {
                break;
            }
        }
    }
    if p_idx < p_chars.len() {
        return None;
    }

    // Exact substring bonus
    let lower_pattern = pattern.to_lowercase();
    let lower_text = text.to_lowercase();
    if let Some(pos) = lower_text.find(&lower_pattern) {
        let mut score = 2000;
        if pos == 0 {
            score += 500;
        } else {
            let prev = lower_text.as_bytes().get(pos - 1).copied().unwrap_or(0);
            if prev == b' ' || prev == b'(' || prev == b'[' || prev == b'-' || prev == b'_' {
                score += 300;
            }
        }
        score -= (text.len() as i32) * 2;
        return Some(score);
    }

    // Subsequence scoring with word-boundary and contiguous bonuses
    let mut score = 1000;
    let mut last_match_idx: Option<usize> = None;
    let mut p = 0;

    for (t_idx, &tc) in t_chars.iter().enumerate() {
        if p < p_chars.len() && tc == p_chars[p] {
            let is_word_start = if t_idx == 0 {
                true
            } else {
                let prev = t_chars[t_idx - 1];
                prev == ' ' || prev == '(' || prev == '[' || prev == '-' || prev == '_' || prev == '/'
            };

            if is_word_start {
                score += 50;
            }

            if let Some(last) = last_match_idx {
                if t_idx == last + 1 {
                    score += 35;
                } else {
                    score -= (t_idx - last - 1) as i32 * 4;
                }
            }

            last_match_idx = Some(t_idx);
            p += 1;
        }
    }

    score -= t_chars.len() as i32;
    Some(score)
}

pub fn match_query(query: &str, text: &str) -> Option<i32> {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return Some(0);
    }
    let mut total_score = 0;
    for tok in tokens {
        match fuzzy_score(tok, text) {
            Some(s) => total_score += s,
            None => return None,
        }
    }
    Some(total_score)
}

pub fn highlight_fuzzy(text: &str, query: &str) -> String {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return glib::markup_escape_text(text).to_string();
    }

    let t_chars: Vec<char> = text.chars().collect();
    let lower_t: Vec<char> = text.to_lowercase().chars().collect();
    let mut matched_indices = vec![false; t_chars.len()];

    for tok in tokens {
        let p_chars: Vec<char> = tok.to_lowercase().chars().collect();
        if p_chars.is_empty() {
            continue;
        }

        let tok_lower = tok.to_lowercase();
        let text_lower: String = lower_t.iter().collect();
        if let Some(byte_pos) = text_lower.find(&tok_lower) {
            let char_start = text_lower[..byte_pos].chars().count();
            let char_len = p_chars.len();
            for i in char_start..(char_start + char_len) {
                if i < matched_indices.len() {
                    matched_indices[i] = true;
                }
            }
            continue;
        }

        let mut p = 0;
        for (t_idx, &tc) in lower_t.iter().enumerate() {
            if p < p_chars.len() && tc == p_chars[p] {
                matched_indices[t_idx] = true;
                p += 1;
            }
        }
    }

    let mut out = String::new();
    let mut in_highlight = false;

    for (idx, &c) in t_chars.iter().enumerate() {
        let is_m = matched_indices[idx];
        if is_m && !in_highlight {
            out.push_str("<b><u>");
            in_highlight = true;
        } else if !is_m && in_highlight {
            out.push_str("</u></b>");
            in_highlight = false;
        }

        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&apos;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }

    if in_highlight {
        out.push_str("</u></b>");
    }

    out
}

struct HudState {
    keybinds: Vec<Keybind>,
    filtered: RefCell<Vec<Keybind>>,
    current_cat_idx: RefCell<usize>,
    categories: Vec<&'static str>,
    filter_buttons: Vec<(Button, Label, String)>,
    theme: ThemePalette,
    window: Window,
    search_entry: Entry,
    count_badge: Label,
    scrolled_window: ScrolledWindow,
    listbox: ListBox,
}

impl HudState {
    fn update_count_badge(&self, count: usize) {
        let total = self.keybinds.len();
        let text = if count == total {
            format!("{} Binds", total)
        } else {
            format!("{} of {}", count, total)
        };
        self.count_badge.set_markup(&format!(
            "<span color='{}' font_size='10pt' font_weight='bold'>{}</span>",
            self.theme.text_sub, text
        ));
    }

    fn populate_list(self: &Rc<Self>) {
        for child in self.listbox.children() {
            self.listbox.remove(&child);
        }

        let query = self.search_entry.text().to_lowercase().trim().to_string();
        let current_cat = *self.current_cat_idx.borrow();
        let cat_filter = self.categories[current_cat];

        let mut scored_list = Vec::new();
        for b in &self.keybinds {
            if cat_filter != "All" {
                if cat_filter == "Custom" && !b.is_custom {
                    continue;
                } else if cat_filter != "Custom"
                    && !b.category.to_lowercase().contains(&cat_filter.to_lowercase())
                {
                    continue;
                }
            }

            if !query.is_empty() {
                let searchable = format!(
                    "{} {} {} {} {}",
                    b.mods.join(" "),
                    b.key,
                    b.desc,
                    b.dispatcher,
                    b.arg
                );
                match match_query(&query, &searchable) {
                    Some(score) => scored_list.push((score, b.clone())),
                    None => continue,
                }
            } else {
                scored_list.push((0, b.clone()));
            }
        }

        if !query.is_empty() {
            scored_list.sort_by(|(s_a, b_a), (s_b, b_b)| {
                s_b.cmp(s_a)
                    .then_with(|| {
                        let prio_a = if b_a.is_custom { 0 } else { 1 };
                        let prio_b = if b_b.is_custom { 0 } else { 1 };
                        prio_a.cmp(&prio_b)
                    })
                    .then_with(|| b_a.desc.cmp(&b_b.desc))
            });
        }

        let filtered_list: Vec<Keybind> = scored_list.into_iter().map(|(_, b)| b).collect();

        self.update_count_badge(filtered_list.len());

        // Empty state handling
        if filtered_list.is_empty() {
            *self.filtered.borrow_mut() = filtered_list;
            let row = ListBoxRow::new();
            row.set_selectable(false);
            row.set_activatable(false);

            let empty_box = Box::new(Orientation::Vertical, 8);
            empty_box.set_halign(Align::Center);
            empty_box.set_margin_top(45);
            empty_box.set_margin_bottom(45);

            let icon = Label::new(None);
            icon.set_markup(&format!(
                "<span font_size='26pt' color='{}'>⌕</span>",
                self.theme.text_muted
            ));
            empty_box.pack_start(&icon, false, false, 0);

            let msg = Label::new(None);
            msg.set_markup(&format!(
                "<span color='{}' font_weight='bold' font_size='12pt'>No actions found for \"{}\"</span>",
                self.theme.text_title,
                glib::markup_escape_text(&query)
            ));
            empty_box.pack_start(&msg, false, false, 0);

            let sub = Label::new(None);
            sub.set_markup(&format!(
                "<span color='{}' font_size='10pt'>Press <tt>Esc</tt> to clear search or <tt>Tab</tt> to switch category</span>",
                self.theme.text_sub
            ));
            empty_box.pack_start(&sub, false, false, 0);

            row.add(&empty_box);
            self.listbox.add(&row);
            self.listbox.show_all();
            return;
        }

        let cat_colors: HashMap<&str, &str> = if self.theme.is_dark {
            [
                ("[Custom]", "#34d399"),
                ("[Apps]", "#60a5fa"),
                ("[Window]", "#c084fc"),
                ("[Workspace]", "#a78bfa"),
                ("[System]", "#fb923c"),
                ("[Media]", "#2dd4bf"),
                ("[Screenshot]", "#f472b6"),
                ("[Settings]", "#facc15"),
                ("[General]", "#9ca3af"),
            ]
            .into_iter()
            .collect()
        } else {
            [
                ("[Custom]", "#15803d"),
                ("[Apps]", "#1d4ed8"),
                ("[Window]", "#7e22ce"),
                ("[Workspace]", "#6d28d9"),
                ("[System]", "#c2410c"),
                ("[Media]", "#0f766e"),
                ("[Screenshot]", "#be185d"),
                ("[Settings]", "#b45309"),
                ("[General]", "#475569"),
            ]
            .into_iter()
            .collect()
        };

        for bind in &filtered_list {
            let row = ListBoxRow::new();
            let h_box = Box::new(Orientation::Horizontal, 14);
            h_box.set_margin_top(4);
            h_box.set_margin_bottom(4);
            h_box.set_margin_start(6);
            h_box.set_margin_end(6);

            // 1. Keycaps
            let caps_box = Box::new(Orientation::Horizontal, 3);
            caps_box.set_size_request(205, -1);
            caps_box.set_halign(Align::Start);
            for m in &bind.mods {
                let lbl = Label::new(Some(m));
                lbl.style_context().add_class("keycap");
                caps_box.pack_start(&lbl, false, false, 0);

                let plus = Label::new(Some("+"));
                plus.style_context().add_class("key-plus");
                caps_box.pack_start(&plus, false, false, 0);
            }

            let key_lbl = Label::new(Some(&bind.key));
            key_lbl.style_context().add_class("keycap");
            caps_box.pack_start(&key_lbl, false, false, 0);
            h_box.pack_start(&caps_box, false, false, 0);

            // 2. Action Title & Subtext
            let text_box = Box::new(Orientation::Vertical, 1);
            let title = Label::new(None);
            if !query.is_empty() {
                let highlighted = highlight_fuzzy(&bind.desc, &query);
                title.set_markup(&format!(
                    "<span color='{}' font_weight='bold'>{}</span>",
                    self.theme.text_title, highlighted
                ));
            } else {
                title.set_text(&bind.desc);
                title.style_context().add_class("desc-title");
            }
            title.set_halign(Align::Start);
            text_box.pack_start(&title, false, false, 0);

            let subtext = if !bind.arg.is_empty() {
                &bind.arg
            } else {
                &bind.dispatcher
            };
            if !subtext.is_empty() {
                let clean_sub = if subtext.chars().count() > 65 {
                    format!("{}...", subtext.chars().take(65).collect::<String>())
                } else {
                    subtext.to_string()
                };
                let sub = Label::new(Some(&clean_sub));
                sub.style_context().add_class("desc-sub");
                sub.set_halign(Align::Start);
                text_box.pack_start(&sub, false, false, 0);
            }
            h_box.pack_start(&text_box, true, true, 0);

            // 3. Badges: User Custom + Category
            let badge_box = Box::new(Orientation::Horizontal, 6);
            badge_box.set_halign(Align::End);

            if bind.is_custom {
                let custom_lbl = Label::new(Some("★ Custom"));
                custom_lbl.style_context().add_class("badge-custom");
                badge_box.pack_start(&custom_lbl, false, false, 0);
            }

            let cat_c = cat_colors
                .get(bind.category.as_str())
                .unwrap_or(&"#94a3b8");
            let cat_lbl = Label::new(None);
            cat_lbl.style_context().add_class("badge-cat");
            cat_lbl.set_markup(&format!(
                "<span color='{}' font_weight='bold'>{}</span>",
                cat_c,
                glib::markup_escape_text(&bind.category)
            ));
            badge_box.pack_start(&cat_lbl, false, false, 0);

            h_box.pack_end(&badge_box, false, false, 0);

            row.add(&h_box);
            self.listbox.add(&row);
        }

        *self.filtered.borrow_mut() = filtered_list;
        self.listbox.show_all();

        if let Some(first_row) = self.listbox.row_at_index(0) {
            self.listbox.select_row(Some(&first_row));
        }
    }

    fn set_active_category(self: &Rc<Self>, idx: usize) {
        let n = self.categories.len();
        let target = idx % n;
        *self.current_cat_idx.borrow_mut() = target;

        for (i, (btn, lbl, text)) in self.filter_buttons.iter().enumerate() {
            if i == target {
                btn.style_context().add_class("active");
                lbl.set_markup(&format!(
                    "<span color='{}' font_weight='heavy'>{}</span>",
                    self.theme.tab_active_fg, text
                ));
            } else {
                btn.style_context().remove_class("active");
                lbl.set_markup(&format!(
                    "<span color='{}' font_weight='bold'>{}</span>",
                    self.theme.tab_inactive_fg, text
                ));
            }
        }

        self.populate_list();
    }

    fn ensure_row_visible(&self, row: &ListBoxRow) {
        let adj = self.scrolled_window.vadjustment();
        let alloc = row.allocation();
        if alloc.height() > 0 {
            let top = alloc.y() as f64;
            let bottom = top + alloc.height() as f64;
            let cur_top = adj.value();
            let cur_bottom = cur_top + adj.page_size();
            if top < cur_top {
                adj.set_value(top);
            } else if bottom > cur_bottom {
                adj.set_value(bottom - adj.page_size());
            }
        }
    }

    fn execute_selected(self: &Rc<Self>) {
        let opt_row = self.listbox.selected_row().or_else(|| self.listbox.row_at_index(0));
        if let Some(row) = opt_row {
            if row.is_selectable() {
                let idx = row.index();
                let binds = self.filtered.borrow();
                if idx >= 0 && (idx as usize) < binds.len() {
                    self.execute_bind(&binds[idx as usize]);
                }
            }
        }
    }

    fn execute_bind(&self, bind: &Keybind) {
        self.window.close();
        let disp = &bind.dispatcher;
        let raw_arg = &bind.arg;
        let expanded_arg = expand_hypr_cmd(raw_arg);

        if disp.eq_ignore_ascii_case("exec") {
            let _ = Command::new("hyprctl")
                .args(["dispatch", "exec", &expanded_arg])
                .spawn();
            let _ = Command::new("sh")
                .args(["-c", &expanded_arg])
                .spawn();
        } else if !disp.is_empty() {
            let mut cmd = Command::new("hyprctl");
            cmd.arg("dispatch").arg(disp);
            if !expanded_arg.is_empty() {
                cmd.arg(&expanded_arg);
            }
            let _ = cmd.spawn();
        }
    }

    fn copy_to_clipboard(&self, text: &str) {
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        clipboard.set_text(text);
        clipboard.store();
        let _ = Command::new("notify-send")
            .args(["-a", "hyprwhich", "Copied Keybind", text])
            .spawn();
    }
}

pub fn run_gui(keybinds: Vec<Keybind>, force_light: bool, initial_query: Option<String>) {
    if gtk::init().is_err() {
        eprintln!("hyprwhich: Failed to initialize GTK 3 display connection.");
        std::process::exit(1);
    }

    let theme = ThemePalette::new(force_light);

    let window = Window::new(WindowType::Toplevel);
    window.set_title("hyprwhich Action Palette");
    window.set_default_size(780, 510);
    window.set_position(WindowPosition::Center);

    // Layer-Shell Setup for native Wayland Overlay
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_anchor(Edge::Top, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);

    // Apply Dynamic CSS
    let provider = gtk::CssProvider::new();
    let css = theme.generate_css();
    if provider.load_from_data(css.as_bytes()).is_ok() {
        if let Some(screen) = gdk::Screen::default() {
            StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
        }
    }

    let main_box = Box::new(Orientation::Vertical, 9);
    main_box.set_size_request(780, 490);
    main_box.set_margin_top(13);
    main_box.set_margin_bottom(12);
    main_box.set_margin_start(16);
    main_box.set_margin_end(16);
    window.add(&main_box);

    // 1. Header Box: Search Entry + Count Badge
    let header_box = Box::new(Orientation::Horizontal, 10);
    let search_entry = Entry::new();
    search_entry.set_widget_name("search");
    search_entry.set_placeholder_text(Some(
        "🔍  Search actions or keybinds... (e.g. presentation, volume, terminal, lock)",
    ));
    header_box.pack_start(&search_entry, true, true, 0);

    let count_badge = Label::new(None);
    header_box.pack_end(&count_badge, false, false, 2);
    main_box.pack_start(&header_box, false, false, 0);

    // 2. Category Counts & Filter Pills
    let categories = vec![
        "All",
        "Custom",
        "Apps",
        "Window",
        "Workspace",
        "System",
        "Media",
        "Screenshot",
        "Settings",
    ];

    let mut cat_counts = HashMap::new();
    cat_counts.insert("All".to_string(), keybinds.len());
    let mut custom_count = 0;
    for b in &keybinds {
        if b.is_custom {
            custom_count += 1;
        }
        let clean = b.category.trim_matches(|c| c == '[' || c == ']').to_string();
        *cat_counts.entry(clean).or_insert(0) += 1;
    }
    cat_counts.insert("Custom".to_string(), custom_count);

    let filter_box = Box::new(Orientation::Horizontal, 6);
    let mut filter_buttons = Vec::new();

    for (idx, cat) in categories.iter().enumerate() {
        let count = cat_counts.get(*cat).copied().unwrap_or(0);
        let btn_text = if count > 0 {
            format!("{} ({})", cat, count)
        } else {
            cat.to_string()
        };

        let btn = Button::new();
        btn.style_context().add_class("filter-btn");
        btn.set_can_focus(false);

        let lbl = Label::new(None);
        if idx == 0 {
            btn.style_context().add_class("active");
            lbl.set_markup(&format!(
                "<span color='{}' font_weight='heavy'>{}</span>",
                theme.tab_active_fg, btn_text
            ));
        } else {
            lbl.set_markup(&format!(
                "<span color='{}' font_weight='bold'>{}</span>",
                theme.tab_inactive_fg, btn_text
            ));
        }
        btn.add(&lbl);
        filter_box.pack_start(&btn, false, false, 0);
        filter_buttons.push((btn, lbl, btn_text));
    }
    main_box.pack_start(&filter_box, false, false, 2);

    // 3. Scrollable List Box
    let scrolled_window = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
    scrolled_window.set_min_content_height(355);
    scrolled_window.set_vexpand(true);

    let listbox = ListBox::new();
    listbox.set_selection_mode(SelectionMode::Single);
    scrolled_window.add(&listbox);
    main_box.pack_start(&scrolled_window, true, true, 0);

    // 4. Footer: Keyboard Navigation Legend
    let footer_box = Box::new(Orientation::Horizontal, 6);
    footer_box.set_halign(Align::Center);

    let legend_items = [
        ("↵ Enter", "Launch"),
        ("↑ / ↓", "Navigate"),
        ("Tab", "Cycle Filter"),
        ("Ctrl+C", "Copy Keys"),
        ("Esc", "Close"),
    ];

    for (idx, (k, desc)) in legend_items.iter().enumerate() {
        if idx > 0 {
            let sep = Label::new(Some("•"));
            sep.style_context().add_class("footer-sep");
            footer_box.pack_start(&sep, false, false, 3);
        }
        let k_lbl = Label::new(Some(k));
        k_lbl.style_context().add_class("footer-key");
        footer_box.pack_start(&k_lbl, false, false, 0);

        let d_lbl = Label::new(Some(desc));
        d_lbl.style_context().add_class("footer-text");
        footer_box.pack_start(&d_lbl, false, false, 0);
    }
    main_box.pack_start(&footer_box, false, false, 2);

    let state = Rc::new(HudState {
        filtered: RefCell::new(keybinds.clone()),
        keybinds,
        current_cat_idx: RefCell::new(0),
        categories,
        filter_buttons,
        theme,
        window: window.clone(),
        search_entry: search_entry.clone(),
        count_badge,
        scrolled_window: scrolled_window.clone(),
        listbox: listbox.clone(),
    });

    // Wire filter button click events
    for (idx, (btn, _, _)) in state.filter_buttons.iter().enumerate() {
        let s = Rc::clone(&state);
        btn.connect_clicked(move |_| {
            s.set_active_category(idx);
        });
    }

    // Wire search change event
    {
        let s = Rc::clone(&state);
        search_entry.connect_changed(move |_| {
            s.populate_list();
        });
    }

    // Wire search entry key press event
    {
        let s = Rc::clone(&state);
        search_entry.connect_key_press_event(move |_, event| {
            let keyval = event.keyval();
            let state_mask = event.state();

            if keyval == keys::Tab {
                let cur = *s.current_cat_idx.borrow();
                let n = s.categories.len();
                let next_idx = if state_mask.contains(ModifierType::SHIFT_MASK) {
                    if cur == 0 { n - 1 } else { cur - 1 }
                } else {
                    (cur + 1) % n
                };
                s.set_active_category(next_idx);
                return glib::Propagation::Stop;
            }

            if keyval == keys::Return || keyval == keys::KP_Enter {
                s.execute_selected();
                return glib::Propagation::Stop;
            }

            if keyval == keys::Down {
                let opt_row = s.listbox.selected_row();
                let idx = opt_row.map(|r| r.index()).unwrap_or(-1);
                if let Some(next_row) = s.listbox.row_at_index(idx + 1) {
                    if next_row.is_selectable() {
                        s.listbox.select_row(Some(&next_row));
                        next_row.grab_focus();
                        s.ensure_row_visible(&next_row);
                    }
                }
                return glib::Propagation::Stop;
            }

            if keyval == keys::Up {
                let opt_row = s.listbox.selected_row();
                let idx = opt_row.map(|r| r.index()).unwrap_or(0);
                if idx > 0 {
                    if let Some(prev_row) = s.listbox.row_at_index(idx - 1) {
                        if prev_row.is_selectable() {
                            s.listbox.select_row(Some(&prev_row));
                            prev_row.grab_focus();
                            s.ensure_row_visible(&prev_row);
                        }
                    }
                }
                return glib::Propagation::Stop;
            }

            if keyval == keys::Escape {
                if !s.search_entry.text().is_empty() {
                    s.search_entry.set_text("");
                    return glib::Propagation::Stop;
                }
                s.window.close();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
    }

    // Wire window key press event
    {
        let s = Rc::clone(&state);
        window.connect_key_press_event(move |_, event| {
            let keyval = event.keyval();
            let state_mask = event.state();

            if keyval == keys::Escape {
                s.window.close();
                return glib::Propagation::Stop;
            }

            if state_mask.contains(ModifierType::CONTROL_MASK)
                && (keyval == keys::c || keyval == keys::C)
            {
                if let Some(row) = s.listbox.selected_row() {
                    let idx = row.index();
                    let binds = s.filtered.borrow();
                    if idx >= 0 && (idx as usize) < binds.len() {
                        let b = &binds[idx as usize];
                        let mut combo = b.mods.clone();
                        combo.push(b.key.clone());
                        let full_combo = combo.join(" + ");
                        s.copy_to_clipboard(&full_combo);
                    }
                }
                return glib::Propagation::Stop;
            }

            if keyval == keys::Return || keyval == keys::KP_Enter {
                s.execute_selected();
                return glib::Propagation::Stop;
            }

            if !s.search_entry.has_focus() {
                s.search_entry.grab_focus();
                s.search_entry.set_position(-1);
            }

            glib::Propagation::Proceed
        });
    }

    // Wire row-activated event
    {
        let s = Rc::clone(&state);
        listbox.connect_row_activated(move |_, row| {
            let idx = row.index();
            let binds = s.filtered.borrow();
            if idx >= 0 && (idx as usize) < binds.len() {
                s.execute_bind(&binds[idx as usize]);
            }
        });
    }

    // Wire window destroy
    window.connect_destroy(|_| {
        gtk::main_quit();
    });

    // Set initial query if passed
    if let Some(ref q) = initial_query {
        search_entry.set_text(q);
        search_entry.set_position(-1);
    }

    // Initial population and render
    state.populate_list();
    window.show_all();
    gtk::main();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_matching() {
        // Fuzzy match "actve" against "Screenshot (Active Window)"
        let target = "Screenshot (Active Window)";
        assert!(fuzzy_score("actve", target).is_some());
        assert!(match_query("actve", target).is_some());
        assert!(match_query("scr actve", target).is_some());

        // Negative check
        assert!(match_query("xyz999", target).is_none());

        // Highlighting check
        let h = highlight_fuzzy(target, "actve");
        assert!(h.contains("<b><u>"));
        assert!(h.contains("</u></b>"));
    }
}
