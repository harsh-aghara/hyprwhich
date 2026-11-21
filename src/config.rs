// config.rs: Hyprland configuration parser (.conf and .lua) with variable expansion and humanizer

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Keybind {
    pub mods: Vec<String>,
    pub key: String,
    pub desc: String,
    pub category: String,
    pub dispatcher: String,
    pub arg: String,
    pub is_custom: bool,
    pub source: String,
}

pub fn expand_hypr_cmd(cmd: &str) -> String {
    let config_home = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_fallback().join(".config"));
    let hypr_dir = config_home.join("hypr");
    let hypr_str = hypr_dir.to_string_lossy();
    let home = dirs_fallback().to_string_lossy().to_string();

    cmd.replace("$scriptsDir", &format!("{}/scripts", hypr_str))
        .replace("$UserScripts", &format!("{}/UserScripts", hypr_str))
        .replace("$userScripts", &format!("{}/UserScripts", hypr_str))
        .replace("$UserConfigs", &format!("{}/UserConfigs", hypr_str))
        .replace("$configs", &format!("{}/configs", hypr_str))
        .replace("$term", "kitty")
        .replace("$files", "thunar")
        .replace("$mainMod", "SUPER")
        .replace("$HOME", &home)
}

fn dirs_fallback() -> PathBuf {
    env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"))
}

pub fn normalize_key(raw: &str) -> String {
    let k = raw.trim();
    let k_lower = k.to_lowercase();
    if k_lower.starts_with("code:") {
        let code = k[5..].trim();
        return match code {
            "10" => "1".to_string(), "11" => "2".to_string(), "12" => "3".to_string(),
            "13" => "4".to_string(), "14" => "5".to_string(), "15" => "6".to_string(),
            "16" => "7".to_string(), "17" => "8".to_string(), "18" => "9".to_string(),
            "19" => "0".to_string(), "36" => "Return".to_string(), "65" => "Space".to_string(),
            "107" => "Print".to_string(), "119" => "Delete".to_string(),
            _ => code.to_string(),
        };
    }

    match k_lower.as_str() {
        "return" | "enter" => "Return".to_string(),
        "space" | "spacebar" => "Space".to_string(),
        "print" => "Print".to_string(),
        "tab" => "Tab".to_string(),
        "escape" | "esc" => "Esc".to_string(),
        "mouse_down" => "Scroll Down".to_string(),
        "mouse_up" => "Scroll Up".to_string(),
        "mouse:272" => "Left Click".to_string(),
        "mouse:273" => "Right Click".to_string(),
        "mouse:274" => "Middle Click".to_string(),
        "xf86audioraisevolume" => "Volume Up".to_string(),
        "xf86audiolowervolume" => "Volume Down".to_string(),
        "xf86audiomute" => "Mute Audio".to_string(),
        "xf86audiomicmute" => "Mute Mic".to_string(),
        "xf86audioplay" => "Media Play/Pause".to_string(),
        "xf86audiopause" => "Media Pause".to_string(),
        "xf86audionext" => "Media Next Track".to_string(),
        "xf86audioprev" => "Media Prev Track".to_string(),
        "xf86audiostop" => "Media Stop".to_string(),
        "xf86monbrightnessup" => "Brightness Up".to_string(),
        "xf86monbrightnessdown" => "Brightness Down".to_string(),
        "xf86kbdbrightnessup" => "Kbd Brightness Up".to_string(),
        "xf86kbdbrightnessdown" => "Kbd Brightness Down".to_string(),
        "xf86touchpadtoggle" => "Touchpad Toggle".to_string(),
        s if s.len() == 1 => s.to_uppercase(),
        s => {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    }
}

pub fn normalize_mods(mods_str: &str) -> Vec<String> {
    let cleaned = mods_str.replace("$mainMod", "SUPER").replace("mainMod", "SUPER");
    let parts: Vec<&str> = cleaned
        .split(|c: char| c.is_whitespace() || c == '+' || c == '_')
        .filter(|s| !s.is_empty())
        .collect();

    let mut active = HashSet::new();
    for p in parts {
        let pl = p.to_lowercase();
        match pl.as_str() {
            "super" | "mod4" | "win" | "logo" => { active.insert("SUPER".to_string()); },
            "ctrl" | "control" => { active.insert("CTRL".to_string()); },
            "alt" | "mod1" | "alt_l" | "alt_r" => { active.insert("ALT".to_string()); },
            "shift" | "shift_l" | "shift_r" => { active.insert("SHIFT".to_string()); },
            other => { active.insert(other.to_uppercase()); },
        }
    }

    let mut ordered = Vec::new();
    for m in &["SUPER", "CTRL", "ALT", "SHIFT"] {
        if active.contains(*m) {
            ordered.push(m.to_string());
        }
    }
    for m in &active {
        if !["SUPER", "CTRL", "ALT", "SHIFT"].contains(&m.as_str()) {
            ordered.push(m.clone());
        }
    }
    ordered
}

pub fn clean_title(text: &str) -> String {
    let t = text.trim().trim_start_matches(|c| c == '#' || c == '-' || c == ' ' || c == '/').trim();
    if t.is_empty() {
        return String::new();
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    let mut capitalized = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let low = w.to_lowercase();
        if i > 0 && ["and", "or", "in", "on", "using", "by", "of", "to", "for", "with", "a", "an", "the"].contains(&low.as_str()) {
            capitalized.push(low);
        } else if w.starts_with('(') && w.len() > 1 {
            let mut chars = w[1..].chars();
            if let Some(first) = chars.next() {
                capitalized.push(format!("({}{}", first.to_uppercase(), chars.as_str()));
            } else {
                capitalized.push(w.to_string());
            }
        } else {
            let mut chars = w.chars();
            if let Some(first) = chars.next() {
                capitalized.push(format!("{}{}", first.to_uppercase(), chars.as_str()));
            } else {
                capitalized.push(w.to_string());
            }
        }
    }
    capitalized.join(" ")
}

pub fn split_identifier_words(token: &str) -> String {
    let stem = token
        .trim_end_matches(".sh")
        .trim_end_matches(".py")
        .trim_end_matches(".bin")
        .trim_end_matches(".lua")
        .trim_end_matches(".bash");

    let mut words = Vec::new();
    let mut cur = String::new();

    let chars: Vec<char> = stem.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '-' || c == '_' || c == '.' || c == ' ' {
            if !cur.is_empty() {
                words.push(cur.clone());
                cur.clear();
            }
        } else if c.is_uppercase() && !cur.is_empty() {
            // Check if camelCase boundary
            let prev = chars[i - 1];
            let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            if prev.is_lowercase() || next_is_lower {
                words.push(cur.clone());
                cur.clear();
            }
            cur.push(c);
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }

    words
        .into_iter()
        .map(|w| {
            let mut chars = w.chars();
            if let Some(f) = chars.next() {
                format!("{}{}", f.to_uppercase(), chars.as_str())
            } else {
                w
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn humanize_direction(dir: &str) -> &'static str {
    match dir {
        "l" | "left" => "Left",
        "r" | "right" => "Right",
        "u" | "up" => "Up",
        "d" | "down" => "Down",
        _ => "",
    }
}

pub fn humanize_hypr_dispatcher(disp: &str, arg: &str) -> Option<(String, String)> {
    let dl = disp.trim().to_lowercase();
    let al = arg.trim();
    let al_low = al.to_lowercase();

    match dl.as_str() {
        "killactive" | "closewindow" => {
            Some(("Close Active Window".to_string(), "[Window]".to_string()))
        }
        "togglefloating" | "setfloating" => {
            Some(("Toggle Floating Window".to_string(), "[Window]".to_string()))
        }
        "settiled" => {
            Some(("Set Window Tiled".to_string(), "[Window]".to_string()))
        }
        "fullscreen" => {
            let title = match al_low.as_str() {
                "1" | "maximize" => "Maximize Window",
                "2" => "Toggle Fullscreen (Keep Gaps)",
                _ => "Toggle Fullscreen",
            };
            Some((title.to_string(), "[Window]".to_string()))
        }
        "fakefullscreen" => {
            Some(("Toggle Fake Fullscreen".to_string(), "[Window]".to_string()))
        }
        "fullscreenstate" => {
            Some(("Toggle Fullscreen State".to_string(), "[Window]".to_string()))
        }
        "pin" => {
            Some(("Pin Window (Visible on All Workspaces)".to_string(), "[Window]".to_string()))
        }
        "centerwindow" => {
            Some(("Center Active Window".to_string(), "[Window]".to_string()))
        }
        "togglesplit" => {
            Some(("Toggle Split Direction (Dwindle)".to_string(), "[Window]".to_string()))
        }
        "swapsplit" => {
            Some(("Swap Split Direction".to_string(), "[Window]".to_string()))
        }
        "pseudo" => {
            Some(("Toggle Pseudo-Tiling".to_string(), "[Window]".to_string()))
        }
        "togglegroup" => {
            Some(("Toggle Window Group (Tabs)".to_string(), "[Window]".to_string()))
        }
        "changegroupactive" => {
            let title = if al_low == "b" || al_low == "prev" || al_low == "-1" {
                "Previous Tab in Group"
            } else {
                "Next Tab in Group"
            };
            Some((title.to_string(), "[Window]".to_string()))
        }
        "moveoutofgroup" => {
            Some(("Move Window Out of Group".to_string(), "[Window]".to_string()))
        }
        "moveintogroup" => {
            let dir = humanize_direction(&al_low);
            if dir.is_empty() {
                Some(("Move Window into Group".to_string(), "[Window]".to_string()))
            } else {
                Some((format!("Move Window into Group {}", dir), "[Window]".to_string()))
            }
        }
        "movegroupwindow" => {
            let title = if al_low == "b" || al_low == "prev" || al_low == "-1" {
                "Move Tab Backward"
            } else {
                "Move Tab Forward"
            };
            Some((title.to_string(), "[Window]".to_string()))
        }
        "movefocus" => {
            let dir = humanize_direction(&al_low);
            if dir.is_empty() {
                Some((format!("Focus Window ({})", al), "[Window]".to_string()))
            } else {
                Some((format!("Focus Window {}", dir), "[Window]".to_string()))
            }
        }
        "movewindow" => {
            let dir = humanize_direction(&al_low);
            if dir.is_empty() {
                Some((format!("Move Window ({})", al), "[Window]".to_string()))
            } else {
                Some((format!("Move Window {}", dir), "[Window]".to_string()))
            }
        }
        "swapwindow" => {
            let dir = humanize_direction(&al_low);
            if dir.is_empty() {
                Some((format!("Swap Window ({})", al), "[Window]".to_string()))
            } else {
                Some((format!("Swap Window {}", dir), "[Window]".to_string()))
            }
        }
        "cyclenext" => {
            let title = if al_low == "prev" || al_low == "b" {
                "Cycle to Previous Window"
            } else {
                "Cycle to Next Window"
            };
            Some((title.to_string(), "[Window]".to_string()))
        }
        "swapnext" => {
            let title = if al_low == "prev" || al_low == "b" {
                "Swap with Previous Window"
            } else {
                "Swap with Next Window"
            };
            Some((title.to_string(), "[Window]".to_string()))
        }
        "resizeactive" => {
            Some((format!("Resize Window ({})", al), "[Window]".to_string()))
        }
        "moveactive" => {
            Some((format!("Move Window Position ({})", al), "[Window]".to_string()))
        }
        "workspace" => {
            let desc = match al_low.as_str() {
                "+1" | "m+1" | "e+1" => "Next Workspace".to_string(),
                "-1" | "m-1" | "e-1" => "Previous Workspace".to_string(),
                "previous" | "empty" => format!("Switch to {} Workspace", clean_title(&al_low)),
                s if s.starts_with("name:") => format!("Switch to Workspace '{}'", &al[5..]),
                _ => format!("Switch to Workspace {}", al),
            };
            Some((desc, "[Workspace]".to_string()))
        }
        "movetoworkspace" => {
            let desc = if al_low.starts_with("special") {
                format!("Move Window to Scratchpad ({})", al)
            } else {
                format!("Move Window to Workspace {}", al)
            };
            Some((desc, "[Workspace]".to_string()))
        }
        "movetoworkspacesilent" => {
            let desc = if al_low.starts_with("special") {
                format!("Move Window to Scratchpad ({}) (Silent)", al)
            } else {
                format!("Move Window to Workspace {} (Silent)", al)
            };
            Some((desc, "[Workspace]".to_string()))
        }
        "togglespecialworkspace" => {
            let desc = if al.is_empty() {
                "Toggle Scratchpad".to_string()
            } else {
                format!("Toggle Scratchpad ({})", al)
            };
            Some((desc, "[Workspace]".to_string()))
        }
        "splitratio" => {
            Some((format!("Adjust Split Ratio ({})", al), "[Window]".to_string()))
        }
        "focusmonitor" => {
            Some((format!("Focus Monitor {}", al), "[Window]".to_string()))
        }
        "movecurrentworkspacetomonitor" => {
            let dir = humanize_direction(&al_low);
            let target = if dir.is_empty() { al } else { dir };
            Some((format!("Move Workspace to Monitor {}", target), "[Workspace]".to_string()))
        }
        "dpms" => {
            let title = match al_low.as_str() {
                "on" => "Turn Displays On",
                "off" => "Turn Displays Off",
                _ => "Toggle Displays",
            };
            Some((title.to_string(), "[Settings]".to_string()))
        }
        "exit" => {
            Some(("Exit Hyprland Session".to_string(), "[System]".to_string()))
        }
        "forcerendererreload" => {
            Some(("Force Reload Renderer".to_string(), "[Settings]".to_string()))
        }
        "submap" => {
            let desc = if al_low == "reset" {
                "Reset Keymap (Exit Submap)".to_string()
            } else {
                format!("Enter Submap ({})", clean_title(al))
            };
            Some((desc, "[Settings]".to_string()))
        }
        "global" => {
            Some((format!("Desktop Overview / Action ({})", clean_title(al)), "[Window]".to_string()))
        }
        _ => None,
    }
}

pub fn humanize_exec_command(arg: &str) -> (String, String) {
    // 1. Evaluate Lua string concatenation: A .. B
    let mut cleaned = arg
        .replace("scriptsDir", "~/.config/hypr/scripts")
        .replace("userScripts", "~/.config/hypr/UserScripts")
        .replace("\\\"", "\"");

    while let Some(dotdot_idx) = cleaned.find("..") {
        let before = cleaned[..dotdot_idx].trim_end();
        let after = cleaned[dotdot_idx + 2..].trim_start();
        let clean_before = before.trim_end_matches(|c| c == '\"' || c == '\'');
        let clean_after = after.trim_start_matches(|c| c == '\"' || c == '\'');
        cleaned = format!("{}{}", clean_before, clean_after);
    }

    let trimmed = cleaned.replace('\"', "").replace('\'', "").trim().to_string();
    if trimmed.is_empty() {
        return ("Run Application".to_string(), "[Apps]".to_string());
    }

    let full_low = trimmed.to_lowercase();

    // Unwrap hyprctl dispatch commands run through exec
    if full_low.starts_with("hyprctl dispatch ") {
        let remainder = trimmed[17..].trim();
        let mut parts = remainder.splitn(2, ' ');
        let inner_disp = parts.next().unwrap_or("");
        let inner_arg = parts.next().unwrap_or("");
        return humanize_dispatcher(inner_disp, inner_arg, "");
    }

    // Variable shortcuts common in Hyprland configs ($term, $files, $browser)
    if full_low == "term" || full_low == "$term" {
        return ("Terminal".to_string(), "[Apps]".to_string());
    }
    if full_low == "files" || full_low == "$files" {
        return ("File Manager".to_string(), "[Apps]".to_string());
    }
    if full_low == "browser" || full_low == "$browser" {
        return ("Web Browser".to_string(), "[Apps]".to_string());
    }

    // Hyprland internal IPC runtime commands run via exec
    if full_low.contains("cursor:zoom_factor") {
        if full_low.contains('*') || full_low.contains("2.0") {
            return ("Zoom In (Screen Magnifier)".to_string(), "[Window]".to_string());
        }
        return ("Zoom Out (Screen Magnifier)".to_string(), "[Window]".to_string());
    }
    if full_low.contains("setprop active opaque toggle") {
        return ("Toggle Window Opacity".to_string(), "[Window]".to_string());
    }
    if full_low.contains("pkill -sigusr1 waybar") || full_low.contains("killall -sigusr1 waybar") {
        return ("Toggle Waybar Visibility".to_string(), "[Settings]".to_string());
    }

    // Extract the primary executable/script token
    // If the command is a shell wrapper (e.g. bash script.sh or pidof ... || rofi ...), find the main tool
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    let mut primary_token = "";
    for tok in &tokens {
        let low = tok.to_lowercase();
        if low == "bash" || low == "sh" || low == "exec" || low == "sudo" || low == "pidof" || low == "pkill" || low == "||" || low == "&&" {
            continue;
        }
        primary_token = tok;
        break;
    }
    if primary_token.is_empty() {
        primary_token = tokens.first().copied().unwrap_or("");
    }

    let file_path = Path::new(primary_token);
    let bin_name = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(primary_token);
    let bin_low = bin_name.to_lowercase();

    // 1. Media Controllers
    if bin_low.contains("playerctl") || bin_low.contains("media") {
        if full_low.contains("play") || full_low.contains("pause") {
            return ("Play / Pause Audio".to_string(), "[Media]".to_string());
        } else if full_low.contains("next") || full_low.contains("nxt") {
            return ("Next Audio Track".to_string(), "[Media]".to_string());
        } else if full_low.contains("previous") || full_low.contains("prev") || full_low.contains("prv") {
            return ("Previous Audio Track".to_string(), "[Media]".to_string());
        } else if full_low.contains("stop") {
            return ("Stop Audio".to_string(), "[Media]".to_string());
        }
        return ("Media Player Control".to_string(), "[Media]".to_string());
    }

    // 2. Audio Volume
    if bin_low.contains("volume") || bin_low.contains("wpctl") || bin_low.contains("pactl") || bin_low.contains("amixer") {
        if full_low.contains("toggle-mic") || (full_low.contains("mic") && full_low.contains("mute")) {
            return ("Toggle Microphone Mute".to_string(), "[Media]".to_string());
        } else if full_low.contains("toggle") || full_low.contains("mute") {
            return ("Toggle Audio Mute".to_string(), "[Media]".to_string());
        } else if full_low.contains("inc") || full_low.contains("up") || full_low.contains("+") {
            return ("Increase Audio Volume".to_string(), "[Media]".to_string());
        } else if full_low.contains("dec") || full_low.contains("down") || full_low.contains("-") {
            return ("Decrease Audio Volume".to_string(), "[Media]".to_string());
        }
        return ("Audio Volume Control".to_string(), "[Media]".to_string());
    }

    // 2b. Online Music & Radio
    if bin_low.contains("beat") || bin_low.contains("music") || bin_low.contains("radio") {
        return ("Online Music (Radio)".to_string(), "[Media]".to_string());
    }

    // 3. Screen / Keyboard Brightness
    if bin_low.contains("brightness") || bin_low.contains("brightnessctl") || bin_low.contains("light") {
        let is_kbd = full_low.contains("kbd") || full_low.contains("keyboard");
        let target = if is_kbd { "Keyboard Backlight" } else { "Screen Brightness" };
        if full_low.contains("inc") || full_low.contains("up") || full_low.contains("+") {
            return (format!("Increase {}", target), "[Settings]".to_string());
        } else if full_low.contains("dec") || full_low.contains("down") || full_low.contains("-") {
            return (format!("Decrease {}", target), "[Settings]".to_string());
        }
        return (format!("Adjust {}", target), "[Settings]".to_string());
    }

    // 4. Screenshots & Screen Recording
    if bin_low.contains("screenshot") || bin_low.contains("grim") || bin_low.contains("hyprshot") || bin_low.contains("wayshot") {
        if full_low.contains("area") || full_low.contains("slurp") || full_low.contains("region") {
            return ("Screenshot (Selected Area)".to_string(), "[Screenshot]".to_string());
        } else if full_low.contains("active") || full_low.contains("window") {
            return ("Screenshot (Active Window)".to_string(), "[Screenshot]".to_string());
        } else if full_low.contains("swappy") || full_low.contains("editor") {
            return ("Screenshot (Swappy Editor)".to_string(), "[Screenshot]".to_string());
        } else if full_low.contains("in5") || full_low.contains("5s") {
            return ("Screenshot in 5s".to_string(), "[Screenshot]".to_string());
        } else if full_low.contains("in10") || full_low.contains("10s") {
            return ("Screenshot in 10s".to_string(), "[Screenshot]".to_string());
        } else if full_low.contains("now") || full_low.contains("output") || full_low.contains("screen") {
            return ("Screenshot (Fullscreen)".to_string(), "[Screenshot]".to_string());
        }
        return ("Capture Screenshot".to_string(), "[Screenshot]".to_string());
    }

    // 5. System Session & Power
    if bin_low.contains("lock") || bin_low.contains("hyprlock") || bin_low.contains("swaylock") {
        return ("Lock Screen".to_string(), "[System]".to_string());
    }
    if bin_low.contains("wlogout") || bin_low.contains("wleave") {
        return ("Power Menu (Logout/Reboot/Shutdown)".to_string(), "[System]".to_string());
    }
    if full_low.contains("poweroff") || full_low.contains("shutdown") {
        return ("Shutdown System".to_string(), "[System]".to_string());
    }
    if full_low.contains("reboot") {
        return ("Reboot System".to_string(), "[System]".to_string());
    }

    // 6. Application Launchers & Switchers
    if bin_low.contains("rofi") || bin_low.contains("wofi") || bin_low.contains("fuzzel") || bin_low.contains("tofi") {
        if full_low.contains("emoji") {
            return ("Emoji Selector".to_string(), "[Apps]".to_string());
        } else if full_low.contains("calc") {
            return ("Calculator".to_string(), "[Apps]".to_string());
        } else if full_low.contains("clip") {
            return ("Clipboard Manager".to_string(), "[Apps]".to_string());
        } else if full_low.contains("window") {
            return ("Window Switcher".to_string(), "[Apps]".to_string());
        } else if full_low.contains("search") {
            return ("Web Search".to_string(), "[Apps]".to_string());
        } else if full_low.contains("drun") || full_low.contains("launcher") {
            return ("Application Launcher".to_string(), "[Apps]".to_string());
        }
        return (format!("Menu ({})", split_identifier_words(bin_name)), "[Apps]".to_string());
    }

    // 7. Dynamic Application Class Detection
    let terminal_names = ["kitty", "alacritty", "ghostty", "foot", "wezterm", "konsole", "gnome-terminal", "xterm"];
    for term in &terminal_names {
        if bin_low.contains(term) {
            let label = split_identifier_words(term);
            return (format!("Terminal ({})", label), "[Apps]".to_string());
        }
    }

    let file_mgr_names = ["thunar", "dolphin", "nautilus", "nemo", "pcmanfm", "yazi", "ranger"];
    for fm in &file_mgr_names {
        if bin_low.contains(fm) {
            let label = split_identifier_words(fm);
            return (format!("File Manager ({})", label), "[Apps]".to_string());
        }
    }

    let browser_names = ["firefox", "zen-browser", "zen", "chromium", "brave", "google-chrome", "librewolf", "vivaldi"];
    for b in &browser_names {
        if bin_low == *b || bin_low.starts_with(b) {
            let label = split_identifier_words(b);
            return (format!("Web Browser ({})", label), "[Apps]".to_string());
        }
    }

    // 8. General Script / Binary Decomposition (Zero Hardcoding!)
    let clean_prog = split_identifier_words(bin_name);

    // Extract any prominent flags
    let prog_low = clean_prog.to_lowercase();
    let qualifier = if (full_low.contains("--toggle") || full_low.contains("toggle")) && !prog_low.contains("toggle") {
        " (Toggle)"
    } else if full_low.contains("--area") && !prog_low.contains("area") {
        " (Area)"
    } else if full_low.contains("--active") && !prog_low.contains("active") {
        " (Active)"
    } else if full_low.contains("--now") && !prog_low.contains("now") {
        " (Now)"
    } else {
        ""
    };

    let cat = categorize_bind("exec", &trimmed, &clean_prog);
    let title = if clean_prog.is_empty() {
        clean_title(&trimmed)
    } else if qualifier.is_empty() {
        clean_prog
    } else {
        format!("{}{}", clean_prog, qualifier)
    };

    (title, cat)
}

pub fn humanize_dispatcher(disp: &str, arg: &str, desc: &str) -> (String, String) {
    humanize_dispatcher_ext(disp, arg, desc, false)
}

pub fn humanize_dispatcher_ext(disp: &str, arg: &str, desc: &str, is_explicit: bool) -> (String, String) {
    // Tier 1: Explicit user intent (bindd description or Lua { desc = "..." })
    if is_explicit && !desc.is_empty() {
        let clean = clean_title(desc);
        let cat = categorize_bind(disp, arg, &clean);
        return (clean, cat);
    }

    // Tier 2: Core Hyprland Compositor Dispatchers take precedence over informal comments
    if let Some((action, cat)) = humanize_hypr_dispatcher(disp, arg) {
        return (action, cat);
    }

    // Tier 3: Informal comment for commands without core dispatchers (e.g. exec scripts)
    if !desc.is_empty() && !desc.starts_with("HL.Dispatcher") {
        let clean = clean_title(desc);
        let cat = categorize_bind(disp, arg, &clean);
        return (clean, cat);
    }

    // Tier 4: Dynamic Command & Exec Decomposition
    let dl = disp.trim().to_lowercase();
    if dl == "exec" || dl == "execr" || dl == "exec_cmd" {
        return humanize_exec_command(arg);
    }

    // Fallback
    (clean_title(&format!("{} {}", disp, arg)), categorize_bind(disp, arg, ""))
}

pub fn categorize_bind(disp: &str, arg: &str, desc: &str) -> String {
    let combined = format!("{} {} {}", disp, arg, desc).to_lowercase();
    if ["killactive", "togglefloat", "fullscreen", "togglesplit", "resize", "group", "float", "movefocus", "swapwindow", "movewindow", "cyclenext"]
        .iter().any(|k| combined.contains(k)) {
        return "[Window]".to_string();
    }
    if ["workspace", "scratchpad", "special", "movecurrentworkspace"]
        .iter().any(|k| combined.contains(k)) {
        return "[Workspace]".to_string();
    }
    if ["screenshot", "grim", "slurp", "swappy"]
        .iter().any(|k| combined.contains(k)) {
        return "[Screenshot]".to_string();
    }
    if ["volume", "audio", "mic", "media", "playerctl", "play", "pause", "music", "beat", "radio"]
        .iter().any(|k| combined.contains(k)) {
        return "[Media]".to_string();
    }
    if ["exit", "lock", "power", "shutdown", "reboot", "suspend", "sleep"]
        .iter().any(|k| combined.contains(k)) {
        return "[System]".to_string();
    }
    if ["terminal", "kitty", "browser", "thunar", "launcher", "drun", "emoji", "calc", "clip"]
        .iter().any(|k| combined.contains(k)) {
        return "[Apps]".to_string();
    }
    if ["blur", "wallust", "theme", "wallpaper", "layout", "sunset", "game", "waybar", "switchkeyboard"]
        .iter().any(|k| combined.contains(k)) {
        return "[Settings]".to_string();
    }
    "[General]".to_string()
}

fn is_custom_action(is_user_config: bool, disp: &str, arg: &str, desc: &str) -> bool {
    if !is_user_config {
        return false;
    }
    let al = arg.to_lowercase();
    let dl = disp.to_lowercase();
    let desc_l = desc.to_lowercase();

    if desc_l.contains("[custom]") || desc_l.contains("custom:") {
        return true;
    }

    if dl == "exec" || dl == "execr" || dl == "exec_cmd" {
        if al.contains("scripts/") || al.contains("userscripts/") || al.contains(".sh") || al.contains(".py") || al.contains("~/") || al.contains("/home/") {
            // Keep specialized media, screenshot, and brightness in their dedicated categories
            if al.contains("screenshot") || al.contains("volume") || al.contains("brightness") || al.contains("media") {
                return false;
            }
            return true;
        }
    }
    false
}

/// Splits a string by delimiter, ignoring delimiters inside single/double quotes,
/// parentheses, brackets, and braces.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("return"), "Return");
        assert_eq!(normalize_key("enter"), "Return");
        assert_eq!(normalize_key("space"), "Space");
        assert_eq!(normalize_key("code:10"), "1");
        assert_eq!(normalize_key("code:36"), "Return");
        assert_eq!(normalize_key("q"), "Q");
    }

    #[test]
    fn test_normalize_mods() {
        assert_eq!(normalize_mods("$mainMod"), vec!["SUPER"]);
        assert_eq!(normalize_mods("SUPER_SHIFT"), vec!["SUPER", "SHIFT"]);
        assert_eq!(normalize_mods("CTRL ALT"), vec!["CTRL", "ALT"]);
    }

    #[test]
    fn test_humanize_hypr_dispatchers() {
        assert_eq!(
            humanize_dispatcher("killactive", "", ""),
            ("Close Active Window".to_string(), "[Window]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("togglefloating", "", ""),
            ("Toggle Floating Window".to_string(), "[Window]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("fullscreen", "1", ""),
            ("Maximize Window".to_string(), "[Window]".to_string())
        );
    }

    #[test]
    fn test_split_identifier_words() {
        assert_eq!(split_identifier_words("my_cool_script.sh"), "My Cool Script");
        assert_eq!(split_identifier_words("toggle-feature"), "Toggle Feature");
        assert_eq!(split_identifier_words("face_unlock"), "Face Unlock");
    }
}
