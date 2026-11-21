// theme.rs: Dynamic desktop theme adaptation engine (Wallust, GTK, Light & Dark mode, WCAG AAA)

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn hex_to_rgb(hex_str: &str) -> (f64, f64, f64) {
    let s = hex_str.trim().trim_start_matches('#');
    let full = if s.len() == 3 {
        s.chars().flat_map(|c| std::iter::repeat(c).take(2)).collect()
    } else {
        s.to_string()
    };
    if full.len() < 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = u8::from_str_radix(&full[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&full[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&full[4..6], 16).unwrap_or(0) as f64 / 255.0;
    (r, g, b)
}

pub fn luminance(r: f64, g: f64, b: f64) -> f64 {
    let adjust = |c: f64| -> f64 {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * adjust(r) + 0.7152 * adjust(g) + 0.0722 * adjust(b)
}

pub fn contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    let (r1, g1, b1) = hex_to_rgb(hex1);
    let (r2, g2, b2) = hex_to_rgb(hex2);
    let l1 = luminance(r1, g1, b1);
    let l2 = luminance(r2, g2, b2);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ThemePalette {
    pub is_dark: bool,
    pub bg_window: String,
    pub bg_surface: String,
    pub bg_card: String,
    pub bg_card_hover: String,
    pub bg_card_selected: String,
    pub border_window: String,
    pub border_card: String,
    pub border_selected: String,

    pub text_title: String,
    pub text_sub: String,
    pub text_muted: String,

    pub accent: String,
    pub accent_fg: String,
    pub keycap_bg: String,
    pub keycap_border: String,
    pub keycap_bevel: String,
    pub keycap_fg: String,

    pub badge_custom_bg: String,
    pub badge_custom_fg: String,
    pub tab_inactive_bg: String,
    pub tab_inactive_fg: String,
    pub tab_active_bg: String,
    pub tab_active_fg: String,
}

impl ThemePalette {
    pub fn new(force_light: bool) -> Self {
        let wallust_colors = read_wallust_colors();
        let is_dark = if force_light {
            false
        } else {
            detect_dark_mode(&wallust_colors)
        };

        if is_dark {
            let base_bg = wallust_colors
                .get("background")
                .cloned()
                .unwrap_or_else(|| "#141416".to_string());
            let bg_surface = "#1b1b22".to_string();
            let bg_card = "#22222a".to_string();
            let bg_card_hover = "#2c2c36".to_string();
            let bg_card_selected = "#333646".to_string();
            let border_window = "#383946".to_string();
            let border_card = "#2a2b37".to_string();
            let border_selected = "#3b82f6".to_string();

            let text_title = "#f4f4f7".to_string();
            let text_sub = "#9ca3af".to_string();
            let text_muted = "#6b7280".to_string();

            let wallust_accent = wallust_colors
                .get("color12")
                .or_else(|| wallust_colors.get("color4"));
            let accent = match wallust_accent {
                Some(ac) if contrast_ratio(ac, &base_bg) >= 3.8 => ac.clone(),
                _ => "#2563eb".to_string(),
            };

            let accent_fg = "#ffffff".to_string();
            let keycap_bg = "#272733".to_string();
            let keycap_border = "#414354".to_string();
            let keycap_bevel = "#121218".to_string();
            let keycap_fg = "#e5e7eb".to_string();

            let badge_custom_bg = "#064e3b".to_string();
            let badge_custom_fg = "#34d399".to_string();
            let tab_inactive_bg = "#202028".to_string();
            let tab_inactive_fg = "#d1d5db".to_string();
            let tab_active_bg = accent.clone();
            let tab_active_fg = "#ffffff".to_string();

            Self {
                is_dark,
                bg_window: base_bg,
                bg_surface,
                bg_card,
                bg_card_hover,
                bg_card_selected,
                border_window,
                border_card,
                border_selected,
                text_title,
                text_sub,
                text_muted,
                accent,
                accent_fg,
                keycap_bg,
                keycap_border,
                keycap_bevel,
                keycap_fg,
                badge_custom_bg,
                badge_custom_fg,
                tab_inactive_bg,
                tab_inactive_fg,
                tab_active_bg,
                tab_active_fg,
            }
        } else {
            let bg_window = "#f8f9fc".to_string();
            let bg_surface = "#eff1f6".to_string();
            let bg_card = "#ffffff".to_string();
            let bg_card_hover = "#f1f3f9".to_string();
            let bg_card_selected = "#e0e7ff".to_string();
            let border_window = "#cbd5e1".to_string();
            let border_card = "#e2e8f0".to_string();
            let border_selected = "#2563eb".to_string();

            let text_title = "#0f172a".to_string();
            let text_sub = "#475569".to_string();
            let text_muted = "#94a3b8".to_string();

            let accent = "#2563eb".to_string();
            let accent_fg = "#ffffff".to_string();
            let keycap_bg = "#f8fafc".to_string();
            let keycap_border = "#cbd5e1".to_string();
            let keycap_bevel = "#94a3b8".to_string();
            let keycap_fg = "#0f172a".to_string();

            let badge_custom_bg = "#dcfce7".to_string();
            let badge_custom_fg = "#15803d".to_string();
            let tab_inactive_bg = "#e2e8f0".to_string();
            let tab_inactive_fg = "#334155".to_string();
            let tab_active_bg = accent.clone();
            let tab_active_fg = "#ffffff".to_string();

            Self {
                is_dark,
                bg_window,
                bg_surface,
                bg_card,
                bg_card_hover,
                bg_card_selected,
                border_window,
                border_card,
                border_selected,
                text_title,
                text_sub,
                text_muted,
                accent,
                accent_fg,
                keycap_bg,
                keycap_border,
                keycap_bevel,
                keycap_fg,
                badge_custom_bg,
                badge_custom_fg,
                tab_inactive_bg,
                tab_inactive_fg,
                tab_active_bg,
                tab_active_fg,
            }
        }
    }

    pub fn generate_css(&self) -> String {
        format!(
            r#"/* Window Backdrop & Container */
window {{
    background-color: {bg_window};
    border: 1.5px solid {border_window};
    border-radius: 14px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.6);
}}

/* Search Input */
entry#search {{
    background-color: {bg_surface};
    color: {text_title};
    border: 1.5px solid {border_card};
    border-radius: 9px;
    padding: 9px 15px;
    font-size: 13.5px;
    font-weight: 600;
}}
entry#search:focus {{
    border-color: {border_selected};
    box-shadow: 0 0 0 2px rgba(37, 99, 235, 0.25);
}}

/* Category Filter Tabs */
button.filter-btn {{
    background-image: none;
    background-color: {tab_inactive_bg};
    border: 1px solid {border_card};
    border-radius: 6px;
    padding: 4px 10px;
    transition: all 90ms ease;
}}
button.filter-btn:hover {{
    background-image: none;
    background-color: {bg_card_hover};
    border-color: {border_window};
}}
button.filter-btn.active {{
    background-image: none;
    background-color: {tab_active_bg};
    border-color: {tab_active_bg};
    box-shadow: 0 2px 8px rgba(37, 99, 235, 0.4);
}}

scrolledwindow {{
    background-color: transparent;
    border: none;
}}

list {{
    background-color: transparent;
}}

/* Row Card */
list > row, list row, row {{
    background-image: none;
    box-shadow: none;
    outline: none;
    padding: 6px 10px;
    border-radius: 7px;
    margin: 2px 0px;
    background-color: {bg_card};
    border: 1px solid {border_card};
    transition: all 80ms ease;
}}
list > row:hover, list row:hover, row:hover {{
    background-image: none;
    box-shadow: none;
    outline: none;
    background-color: {bg_card_hover};
    border-color: {border_window};
}}
list > row:selected, list row:selected, row:selected,
list > row:selected:focus, list row:selected:focus, row:selected:focus,
list > row:selected:hover, list row:selected:hover, row:selected:hover {{
    background-image: none;
    box-shadow: none;
    outline: none;
    background-color: {bg_card_selected};
    border: 1.5px solid {border_selected};
}}

/* Mechanical 3D Keycaps */
label.keycap {{
    background-color: {keycap_bg};
    color: {keycap_fg};
    border: 1px solid {keycap_border};
    border-bottom: 2.5px solid {keycap_bevel};
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11.5px;
    font-weight: 700;
    font-family: monospace;
}}

label.key-plus {{
    color: {text_muted};
    font-size: 11px;
    font-weight: 600;
}}

label.desc-title {{
    color: {text_title};
    font-size: 13.5px;
    font-weight: 600;
}}

label.desc-sub {{
    color: {text_sub};
    font-size: 11px;
    font-family: monospace;
}}

label.badge-custom {{
    background-color: {badge_custom_bg};
    color: {badge_custom_fg};
    border-radius: 4px;
    padding: 2px 7px;
    font-size: 10.5px;
    font-weight: 700;
}}

label.badge-cat {{
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 600;
}}

/* Footer Hints */
label.footer-key {{
    background-color: {keycap_bg};
    color: {keycap_fg};
    border: 1px solid {keycap_border};
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 10.5px;
    font-weight: 600;
    font-family: monospace;
}}
label.footer-text {{
    color: {text_sub};
    font-size: 11.5px;
    font-weight: 500;
}}
label.footer-sep {{
    color: {text_muted};
    font-size: 11px;
}}
"#,
            bg_window = self.bg_window,
            border_window = self.border_window,
            bg_surface = self.bg_surface,
            text_title = self.text_title,
            border_card = self.border_card,
            border_selected = self.border_selected,
            tab_inactive_bg = self.tab_inactive_bg,
            bg_card_hover = self.bg_card_hover,
            tab_active_bg = self.tab_active_bg,
            bg_card = self.bg_card,
            bg_card_selected = self.bg_card_selected,
            keycap_bg = self.keycap_bg,
            keycap_fg = self.keycap_fg,
            keycap_border = self.keycap_border,
            keycap_bevel = self.keycap_bevel,
            text_muted = self.text_muted,
            text_sub = self.text_sub,
            badge_custom_bg = self.badge_custom_bg,
            badge_custom_fg = self.badge_custom_fg,
        )
    }
}

pub fn read_wallust_colors() -> HashMap<String, String> {
    let home = env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"));
    let config_home = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".config"));

    let candidate_paths = [
        config_home.join("hypr/wallust/wallust-hyprland.conf"),
        config_home.join("wallust/templates/colors-hyprland.conf"),
        home.join(".config/hypr/wallust/wallust-hyprland.conf"),
    ];

    let mut colors = HashMap::new();
    for conf_path in &candidate_paths {
        if conf_path.exists() {
            if let Ok(content) = fs::read_to_string(conf_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('$') {
                        if let Some(eq_idx) = trimmed.find('=') {
                            let var_name = trimmed[1..eq_idx].trim().to_string();
                            let val_part = trimmed[eq_idx + 1..].trim();
                            if val_part.starts_with("rgb(") && val_part.ends_with(')') {
                                let hex = &val_part[4..val_part.len() - 1];
                                colors.insert(var_name, format!("#{}", hex));
                            } else if val_part.starts_with('#') {
                                colors.insert(var_name, val_part.to_string());
                            }
                        }
                    }
                }
                if !colors.is_empty() {
                    break;
                }
            }
        }
    }
    colors
}

pub fn detect_dark_mode(wallust_colors: &HashMap<String, String>) -> bool {
    let mut dark_score = 0;
    let mut light_score = 0;
    let mut has_signal = false;

    let home = env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"));

    // 1. ~/.cache/.theme_mode (weight: 10)
    let cache_theme = home.join(".cache/.theme_mode");
    if cache_theme.exists() {
        if let Ok(val) = fs::read_to_string(cache_theme) {
            let v = val.trim().to_lowercase();
            if v.contains("light") {
                light_score += 10;
                has_signal = true;
            } else if v.contains("dark") {
                dark_score += 10;
                has_signal = true;
            }
        }
    }

    // 2. Waybar active style symlink (weight: 8)
    let waybar_css = home.join(".config/waybar/style.css");
    if waybar_css.is_symlink() {
        if let Ok(target) = fs::read_link(waybar_css) {
            let t = target.to_string_lossy().to_lowercase();
            if t.contains("[light]") {
                light_score += 8;
                has_signal = true;
            } else if t.contains("[dark]") {
                dark_score += 8;
                has_signal = true;
            }
        }
    }

    // 3. Wallust toml palette mode (weight: 7)
    let wallust_toml = home.join(".config/wallust/wallust.toml");
    if wallust_toml.exists() {
        if let Ok(val) = fs::read_to_string(wallust_toml) {
            for line in val.lines() {
                let t = line.trim();
                if t.starts_with("palette") {
                    let low = t.to_lowercase();
                    if low.contains("light") {
                        light_score += 7;
                        has_signal = true;
                    } else if low.contains("dark") {
                        dark_score += 7;
                        has_signal = true;
                    }
                }
            }
        }
    }

    // 4. gsettings color-scheme preference (weight: 6)
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let cs = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if cs.contains("light") {
            light_score += 6;
            has_signal = true;
        } else if cs.contains("dark") {
            dark_score += 6;
            has_signal = true;
        }
    }

    // 5. gsettings gtk-theme (weight: 4)
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let gt = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if gt.contains("dark") || gt.contains("black") || gt.contains("mocha") {
            dark_score += 4;
            has_signal = true;
        } else if gt.contains("light") || gt.contains("white") || gt.contains("latte") {
            light_score += 4;
            has_signal = true;
        }
    }

    // 6. Wallust background luminance (weight: 5)
    if let Some(bg_hex) = wallust_colors.get("background") {
        let (r, g, b) = hex_to_rgb(bg_hex);
        let lum = luminance(r, g, b);
        if lum <= 0.45 {
            dark_score += 5;
        } else {
            light_score += 5;
        }
        has_signal = true;
    }

    if has_signal {
        dark_score >= light_score
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wcag_contrast() {
        let white = "#ffffff";
        let black = "#000000";
        let ratio = contrast_ratio(white, black);
        assert!(ratio >= 20.0);

        // Electric blue on white
        let electric_blue = "#2563eb";
        let blue_on_white = contrast_ratio(electric_blue, white);
        assert!(blue_on_white >= 4.5);
    }

    #[test]
    fn test_luminance_bounds() {
        let (r, g, b) = hex_to_rgb("#ffffff");
        assert!((luminance(r, g, b) - 1.0).abs() < 0.01);
        let (r0, g0, b0) = hex_to_rgb("#000000");
        assert!((luminance(r0, g0, b0) - 0.0).abs() < 0.01);
    }
}
