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
pub fn split_top_level(s: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut paren_depth: usize = 0;
    let mut bracket_depth: usize = 0;
    let mut brace_depth: usize = 0;
    let mut escape = false;

    for c in s.chars() {
        if escape {
            cur.push(c);
            escape = false;
            continue;
        }

        if c == '\\' {
            cur.push(c);
            escape = true;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            cur.push(c);
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            cur.push(c);
        } else if !in_single_quote && !in_double_quote {
            match c {
                '(' => {
                    paren_depth += 1;
                    cur.push(c);
                }
                ')' => {
                    paren_depth = paren_depth.saturating_sub(1);
                    cur.push(c);
                }
                '[' => {
                    bracket_depth += 1;
                    cur.push(c);
                }
                ']' => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    cur.push(c);
                }
                '{' => {
                    brace_depth += 1;
                    cur.push(c);
                }
                '}' => {
                    brace_depth = brace_depth.saturating_sub(1);
                    cur.push(c);
                }
                _ if c == delimiter && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    parts.push(cur.trim().to_string());
                    cur.clear();
                }
                _ => cur.push(c),
            }
        } else {
            cur.push(c);
        }
    }

    if !cur.is_empty() || !parts.is_empty() {
        parts.push(cur.trim().to_string());
    }

    parts
}

/// Given a call like `hl.dsp.exec_cmd(os.getenv("HOME") .. "/face.sh")` or `exec_cmd(...)`,
/// extracts the dispatcher name (e.g. "exec_cmd") and the content inside the balanced `(...)`.
pub fn extract_balanced_call(s: &str) -> Option<(String, String)> {
    let trimmed = s.trim();
    let open_idx = trimmed.find('(')?;
    let prefix = trimmed[..open_idx].trim();
    let disp_name = if let Some(dot_idx) = prefix.rfind('.') {
        &prefix[dot_idx + 1..]
    } else {
        prefix
    };

    let tail = &trimmed[open_idx + 1..];
    let mut depth = 1usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    let mut end_idx = None;

    for (i, c) in tail.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(i);
                    break;
                }
            }
        }
    }

    let inside = match end_idx {
        Some(i) => &tail[..i],
        None => tail.trim_end_matches(')'),
    };

    Some((disp_name.trim().to_string(), inside.trim().to_string()))
}

/// Normalizes Lua expressions including os.getenv, variable replacements, and string concatenations.
pub fn normalize_lua_expr(expr: &str) -> String {
    let mut res = expr.to_string();

    // 1. Expand os.getenv("HOME") or os.getenv('HOME')
    res = res
        .replace("os.getenv(\"HOME\")", "~")
        .replace("os.getenv('HOME')", "~")
        .replace("os.getenv(\"USER\")", &env::var("USER").unwrap_or_default())
        .replace("os.getenv('USER')", &env::var("USER").unwrap_or_default());

    // Expand any other os.getenv("FOO")
    while let Some(start) = res.find("os.getenv(") {
        let after = &res[start + 10..];
        if let Some(close) = after.find(')') {
            let var_name = after[..close].trim().trim_matches(|c| c == '"' || c == '\'');
            let val = env::var(var_name).unwrap_or_default();
            res = format!("{}{}{}", &res[..start], val, &after[close + 1..]);
        } else {
            break;
        }
    }

    // 2. Variable substitutions
    res = res
        .replace("scriptsDir", "~/.config/hypr/scripts")
        .replace("userScripts", "~/.config/hypr/UserScripts")
        .replace("UserConfigs", "~/.config/hypr/UserConfigs")
        .replace("configs", "~/.config/hypr/configs")
        .replace("mainMod", "SUPER")
        .replace("\\\"", "\"")
        .replace("\\'", "'");

    // 3. Evaluate Lua string concatenations (..)
    if res.contains("..") {
        let parts: Vec<&str> = res.split("..").collect();
        let mut combined = String::new();
        for p in parts {
            let clean = p.trim().trim_matches(|c| c == '"' || c == '\'');
            combined.push_str(clean);
        }
        res = combined;
    }

    res.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string()
}

/// Extracts optional desc/description field from Lua options table: { desc = "Foo", locked = true }
pub fn extract_desc_from_lua_table(table_str: &str) -> Option<String> {
    for key in &["desc", "description"] {
        if let Some(pos) = table_str.find(key) {
            let after = &table_str[pos + key.len()..];
            if let Some(eq_pos) = after.find('=') {
                let rest = after[eq_pos + 1..].trim();
                let mut chars = rest.chars();
                if let Some(quote) = chars.next() {
                    if quote == '"' || quote == '\'' {
                        let inside = &rest[1..];
                        if let Some(close_pos) = inside.find(quote) {
                            return Some(inside[..close_pos].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parses a line of Lua keybind, e.g.:
/// hl.bind(mainMod .. " + Escape", hl.dsp.exec_cmd(os.getenv("HOME") .. "/face_unlock.sh"), { locked = true })
pub fn parse_lua_bind_line(line: &str) -> Option<(Vec<String>, String, String, String, Option<String>)> {
    let trimmed = line.trim();
    let is_lua_bind = trimmed.starts_with("hl.bind(")
        || trimmed.starts_with("hl.bind (")
        || trimmed.starts_with("hypr.bind(")
        || (trimmed.starts_with("bind(") && !trimmed.starts_with("bind ="));

    if !is_lua_bind {
        return None;
    }

    let open_idx = trimmed.find('(')?;
    let tail = &trimmed[open_idx + 1..];
    let mut depth = 1usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    let mut end_idx = None;

    for (i, c) in tail.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end_idx = Some(i);
                    break;
                }
            }
        }
    }

    let inner = match end_idx {
        Some(i) => &tail[..i],
        None => tail.trim_end_matches(')'),
    };

    let args = split_top_level(inner, ',');
    if args.len() < 2 {
        return None;
    }

    // Arg 0: Combo
    let raw_combo = normalize_lua_expr(&args[0]);
    let tokens: Vec<&str> = raw_combo.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if tokens.is_empty() {
        return None;
    }
    let key = normalize_key(tokens[tokens.len() - 1]);
    let mods_str = tokens[..tokens.len() - 1].join(" ");
    let mods = normalize_mods(&mods_str);

    // Arg 1: Dispatcher
    let (dsp_type, raw_arg) = extract_balanced_call(&args[1])?;
    let normalized_arg = normalize_lua_expr(&raw_arg);

    // Arg 2 (optional): Options table
    let table_desc = if args.len() > 2 {
        extract_desc_from_lua_table(&args[2])
    } else {
        None
    };

    Some((mods, key, dsp_type, normalized_arg, table_desc))
}

pub fn is_user_config_path(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    if s.contains("/configs/") || s.contains("/etc/hypr/") || s.contains("/base/") {
        return false;
    }
    true
}

pub fn config_file_priority(path: &Path) -> (u8, String) {
    let s = path.to_string_lossy().to_lowercase();
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("").to_lowercase();

    let prio = if s.contains("/configs/") || s.contains("/etc/") || s.contains("/base/") {
        4
    } else if s.contains("userconfig")
        || s.contains("/custom/")
        || s.contains("/conf.d/")
        || s.contains("override")
        || file_name.starts_with("user")
        || file_name.starts_with("my_")
        || file_name.starts_with("custom")
    {
        0
    } else if file_name.contains("keybind") || file_name.contains("bind") {
        1
    } else if file_name == "hyprland.conf" || file_name == "hyprland.lua" {
        2
    } else {
        3
    };

    (prio, s)
}

pub fn discover_all_config_files(hypr_dir: &Path) -> Vec<(PathBuf, bool)> {
    let mut discovered: HashSet<PathBuf> = HashSet::new();
    let mut queue = vec![hypr_dir.to_path_buf()];

    // 1. Recursive directory walk up to depth 4
    let mut depth_map: HashMap<PathBuf, usize> = HashMap::new();
    depth_map.insert(hypr_dir.to_path_buf(), 0);

    while let Some(current_dir) = queue.pop() {
        let current_depth = depth_map.get(&current_dir).copied().unwrap_or(0);
        if current_depth >= 4 {
            continue;
        }

        let entries = match fs::read_dir(&current_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "build" || name == "node_modules" {
                continue;
            }

            if path.is_dir() {
                depth_map.insert(path.clone(), current_depth + 1);
                queue.push(path);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "conf" || ext == "lua" {
                        discovered.insert(path);
                    }
                }
            }
        }
    }

    // 2. Also follow source = directives in any .conf files
    let existing_files: Vec<PathBuf> = discovered.iter().cloned().collect();
    for conf_path in existing_files {
        if conf_path.extension().and_then(|e| e.to_str()) == Some("conf") {
            if let Ok(file) = File::open(&conf_path) {
                let reader = BufReader::new(file);
                for line in reader.lines().flatten() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("source") {
                        if let Some(eq_idx) = trimmed.find('=') {
                            let raw_target = trimmed[eq_idx + 1..].trim();
                            let expanded = expand_hypr_cmd(raw_target);
                            let target_path = PathBuf::from(&expanded);
                            if target_path.exists() && target_path.is_file() {
                                discovered.insert(target_path);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback: check /etc/hypr/hyprland.conf if discovered list is empty
    if discovered.is_empty() {
        let etc_hypr = PathBuf::from("/etc/hypr/hyprland.conf");
        if etc_hypr.exists() {
            discovered.insert(etc_hypr);
        }
    }

    // 4. Convert to Vec with priority and is_user_config flag
    let mut file_list: Vec<(PathBuf, bool, u8, String)> = discovered
        .into_iter()
        .map(|p| {
            let (prio, sort_key) = config_file_priority(&p);
            let is_user = is_user_config_path(&p);
            (p, is_user, prio, sort_key)
        })
        .collect();

    file_list.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.3.cmp(&b.3)));

    file_list.into_iter().map(|(p, is_user, _, _)| (p, is_user)).collect()
}

pub fn parse_all_configs() -> Vec<Keybind> {
    let config_home = env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_fallback().join(".config"));
    let hypr_dir = config_home.join("hypr");

    let files = discover_all_config_files(&hypr_dir);

    let mut all_binds = Vec::new();
    let mut seen = HashSet::new();

    for (fpath, is_user_config) in &files {
        if !fpath.exists() {
            continue;
        }

        let file = match File::open(fpath) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);

        let mut pending_comment: Option<String> = None;
        let mut conf_vars: HashMap<String, String> = HashMap::new();

        for line in reader.lines().flatten() {
            let line = line.trim();
            if line.is_empty() {
                pending_comment = None;
                continue;
            }

            // Check comment line
            if line.starts_with('#') || line.starts_with("--") || line.starts_with("//") {
                let stripped = line.trim_start_matches(|c| c == '#' || c == '-' || c == '/').trim();
                if !stripped.is_empty() && stripped.chars().any(|c| c.is_alphanumeric()) {
                    pending_comment = Some(stripped.to_string());
                }
                continue;
            }

            // Check variable definition in .conf ($foo = bar)
            if line.starts_with('$') && !line.starts_with("$mainMod") {
                if let Some(eq_idx) = line.find('=') {
                    let var_name = line[..eq_idx].trim().to_string();
                    let var_val = line[eq_idx + 1..].trim().to_string();
                    conf_vars.insert(var_name, var_val);
                    pending_comment = None;
                    continue;
                }
            }

            // 1. Parse .conf syntax: bind[flags] = MOD, KEY, ...
            if let Some(idx) = line.find('=') {
                let (prefix, rest) = line.split_at(idx);
                let flag = prefix.trim().to_lowercase();
                if flag.starts_with("bind") {
                    let mut rest_str = rest[1..].trim().to_string();

                    // Expand conf variables
                    for (k, v) in &conf_vars {
                        if rest_str.contains(k) {
                            rest_str = rest_str.replace(k, v);
                        }
                    }
                    rest_str = expand_hypr_cmd(&rest_str);

                    let (clean_rest, inline_cmt) = if let Some(h_idx) = rest_str.find('#') {
                        (rest_str[..h_idx].trim().to_string(), rest_str[h_idx + 1..].trim().to_string())
                    } else {
                        (rest_str, String::new())
                    };

                    let parts: Vec<&str> = clean_rest.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        let raw_mods = parts[0];
                        let raw_key = parts[1];
                        let mods = normalize_mods(raw_mods);
                        let key = normalize_key(raw_key);

                        let has_desc = flag.contains('d') && flag != "bind";
                        let (desc, disp, arg) = if has_desc {
                            let d = parts.get(2).copied().unwrap_or("");
                            let disp = parts.get(3).copied().unwrap_or("");
                            let arg = if parts.len() > 4 { parts[4..].join(",") } else { String::new() };
                            (d, disp, arg)
                        } else {
                            let disp = parts.get(2).copied().unwrap_or("");
                            let arg = if parts.len() > 3 { parts[3..].join(",") } else { String::new() };
                            ("", disp, arg)
                        };

                        let effective_desc = if !desc.is_empty() {
                            desc.to_string()
                        } else if !inline_cmt.is_empty() {
                            inline_cmt
                        } else {
                            pending_comment.take().unwrap_or_default()
                        };

                        let (eff_disp, eff_arg) = if (disp == "exec" || disp == "execr" || disp == "exec_cmd") && arg.starts_with("hyprctl dispatch ") {
                            let remainder = arg[17..].trim();
                            let mut parts = remainder.splitn(2, ' ');
                            (parts.next().unwrap_or(disp), parts.next().unwrap_or("").to_string())
                        } else {
                            (disp, arg.clone())
                        };

                        let (final_desc, mut cat) = humanize_dispatcher_ext(eff_disp, &eff_arg, &effective_desc, has_desc);

                        let is_truly_custom = is_custom_action(*is_user_config, disp, &arg, &final_desc);
                        if is_truly_custom {
                            cat = "[Custom]".to_string();
                        }

                        let key_id = (mods.join(" + "), key.clone());
                        if !seen.contains(&key_id) {
                            seen.insert(key_id);
                            all_binds.push(Keybind {
                                mods,
                                key,
                                desc: final_desc,
                                category: cat,
                                dispatcher: disp.to_string(),
                                arg,
                                is_custom: is_truly_custom,
                                source: fpath.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string(),
                            });
                        }
                    }
                    continue;
                }
            }

            // 2. Parse .lua syntax: hl.bind(...)
            if let Some((mods, key, dsp_type, normalized_arg, table_desc)) = parse_lua_bind_line(line) {
                let is_explicit = table_desc.is_some();
                let effective_desc = if let Some(d) = table_desc {
                    d
                } else {
                    pending_comment.take().unwrap_or_default()
                };

                let (eff_disp, eff_arg) = if (dsp_type == "exec" || dsp_type == "execr" || dsp_type == "exec_cmd") && normalized_arg.starts_with("hyprctl dispatch ") {
                    let remainder = normalized_arg[17..].trim();
                    let mut parts = remainder.splitn(2, ' ');
                    (parts.next().unwrap_or(&dsp_type).to_string(), parts.next().unwrap_or("").to_string())
                } else {
                    (dsp_type.clone(), normalized_arg.clone())
                };

                let (final_desc, mut cat) = humanize_dispatcher_ext(&eff_disp, &eff_arg, &effective_desc, is_explicit);
                let is_truly_custom = is_custom_action(*is_user_config, &dsp_type, &normalized_arg, &final_desc);
                if is_truly_custom {
                    cat = "[Custom]".to_string();
                }

                let key_id = (mods.join(" + "), key.clone());
                if !seen.contains(&key_id) {
                    seen.insert(key_id);
                    all_binds.push(Keybind {
                        mods,
                        key,
                        desc: final_desc,
                        category: cat,
                        dispatcher: dsp_type,
                        arg: normalized_arg,
                        is_custom: is_truly_custom,
                        source: fpath.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string(),
                    });
                }
                continue;
            }

            // Non-bind code line resets pending comment
            pending_comment = None;
        }
    }

    let mut filtered: Vec<Keybind> = all_binds
        .into_iter()
        .filter(|b| {
            let kl = b.key.to_lowercase();
            !kl.contains("switch") && kl != "xf86launch1" && kl != "xf86launch3" && kl != "xf86launch4"
        })
        .collect();

    filtered.sort_by(|a, b| {
        let prio_a = if a.is_custom { 0 } else { 1 };
        let prio_b = if b.is_custom { 0 } else { 1 };
        prio_a.cmp(&prio_b)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.desc.cmp(&b.desc))
    });

    filtered
}

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
        assert_eq!(
            humanize_dispatcher("changegroupactive", "f", ""),
            ("Next Tab in Group".to_string(), "[Window]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("changegroupactive", "b", ""),
            ("Previous Tab in Group".to_string(), "[Window]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("workspace", "+1", ""),
            ("Next Workspace".to_string(), "[Workspace]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("movetoworkspacesilent", "3", ""),
            ("Move Window to Workspace 3 (Silent)".to_string(), "[Workspace]".to_string())
        );
        assert_eq!(
            humanize_dispatcher("movefocus", "l", ""),
            ("Focus Window Left".to_string(), "[Window]".to_string())
        );
    }

    #[test]
    fn test_humanize_exec_dynamic() {
        // Dynamic commands without static maps
        let (desc, cat) = humanize_dispatcher("exec", "foot", "");
        assert_eq!(desc, "Terminal (Foot)");
        assert_eq!(cat, "[Apps]");

        let (desc, cat) = humanize_dispatcher("exec", "dolphin", "");
        assert_eq!(desc, "File Manager (Dolphin)");
        assert_eq!(cat, "[Apps]");

        let (desc, cat) = humanize_dispatcher("exec", "zen-browser", "");
        assert_eq!(desc, "Web Browser (Zen Browser)");
        assert_eq!(cat, "[Apps]");

        let (desc, cat) = humanize_dispatcher("exec", "playerctl play-pause", "");
        assert_eq!(desc, "Play / Pause Audio");
        assert_eq!(cat, "[Media]");

        let (desc, cat) = humanize_dispatcher("exec", "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+", "");
        assert_eq!(desc, "Increase Audio Volume");
        assert_eq!(cat, "[Media]");

        let (desc, cat) = humanize_dispatcher("exec", "screenshot.sh --active", "");
        assert_eq!(desc, "Screenshot (Active Window)");
        assert_eq!(cat, "[Screenshot]");

        // Arbitrary unknown user script decomposed algorithmically
        let (desc, _) = humanize_dispatcher("exec", "~/.config/hypr/scripts/my_backup_photos.sh --quick", "");
        assert_eq!(desc, "My Backup Photos");

        let (desc2, _) = humanize_dispatcher("exec", "toggle-presentation-mode.sh", "");
        assert_eq!(desc2, "Toggle Presentation Mode");
    }

    #[test]
    fn test_split_identifier_words() {
        assert_eq!(split_identifier_words("my_cool_script.sh"), "My Cool Script");
        assert_eq!(split_identifier_words("toggle-feature"), "Toggle Feature");
        assert_eq!(split_identifier_words("rofiEmoji"), "Rofi Emoji");
        assert_eq!(split_identifier_words("face_unlock"), "Face Unlock");
    }

    #[test]
    fn test_custom_action_detection() {
        assert!(is_custom_action(true, "exec", "~/.config/hypr/scripts/custom_tool.sh", "Custom Tool"));
        assert!(!is_custom_action(false, "exec", "~/.config/hypr/scripts/custom_tool.sh", "Custom Tool"));
        assert!(!is_custom_action(true, "workspace", "1", "Switch to Workspace 1"));
    }

    #[test]
    fn test_split_top_level() {
        let input = r#"mainMod .. " + Escape", hl.dsp.exec_cmd(os.getenv("HOME") .. "/face_unlock.sh"), { locked = true }"#;
        let parts = split_top_level(input, ',');
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], r#"mainMod .. " + Escape""#);
        assert_eq!(parts[1], r#"hl.dsp.exec_cmd(os.getenv("HOME") .. "/face_unlock.sh")"#);
        assert_eq!(parts[2], r#"{ locked = true }"#);
    }

    #[test]
    fn test_extract_balanced_call() {
        let input = r#"hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/hyprlock/face_unlock.sh")"#;
        let (dsp, inside) = extract_balanced_call(input).unwrap();
        assert_eq!(dsp, "exec_cmd");
        assert_eq!(inside, r#"os.getenv("HOME") .. "/.config/hypr/hyprlock/face_unlock.sh""#);

        let input_simple = "hl.dsp.killactive()";
        let (dsp, inside) = extract_balanced_call(input_simple).unwrap();
        assert_eq!(dsp, "killactive");
        assert_eq!(inside, "");
    }

    #[test]
    fn test_normalize_lua_expr() {
        let expr = r#"os.getenv("HOME") .. "/.config/hypr/hyprlock/face_unlock.sh""#;
        let res = normalize_lua_expr(expr);
        assert_eq!(res, "~/.config/hypr/hyprlock/face_unlock.sh");

        let expr_scripts = r#"scriptsDir .. "/ScreenShot.sh""#;
        let res_scripts = normalize_lua_expr(expr_scripts);
        assert_eq!(res_scripts, "~/.config/hypr/scripts/ScreenShot.sh");
    }

    #[test]
    fn test_parse_lua_bind_line() {
        let line = r#"hl.bind(mainMod .. " + Escape", hl.dsp.exec_cmd(os.getenv("HOME") .. "/.config/hypr/hyprlock/face_unlock.sh"), { locked = true })"#;
        let (mods, key, dsp, arg, desc) = parse_lua_bind_line(line).unwrap();
        assert_eq!(mods, vec!["SUPER"]);
        assert_eq!(key, "Esc");
        assert_eq!(dsp, "exec_cmd");
        assert_eq!(arg, "~/.config/hypr/hyprlock/face_unlock.sh");
        assert_eq!(desc, None);

        let (final_desc, _) = humanize_dispatcher(&dsp, &arg, "Gaze On-Demand Facial Authentication (Lock Screen)");
        assert_eq!(final_desc, "Gaze On-Demand Facial Authentication (Lock Screen)");
        assert!(is_custom_action(true, &dsp, &arg, &final_desc));
    }

    #[test]
    fn test_hyprctl_dispatch_unwrap() {
        let (desc, cat) = humanize_dispatcher("exec_cmd", "hyprctl dispatch movecurrentworkspacetomonitor l", "");
        assert_eq!(desc, "Move Workspace to Monitor Left");
        assert_eq!(cat, "[Workspace]");
    }

    #[test]
    fn test_config_file_priority() {
        let p_user = Path::new("/home/user/.config/hypr/UserConfigs/UserKeybinds.lua");
        let p_custom = Path::new("/home/user/.config/hypr/custom/keys.conf");
        let p_conf_d = Path::new("/home/user/.config/hypr/conf.d/test.conf");
        let p_main = Path::new("/home/user/.config/hypr/hyprland.lua");
        let p_base = Path::new("/home/user/.config/hypr/configs/Keybinds.lua");

        assert_eq!(config_file_priority(p_user).0, 0);
        assert_eq!(config_file_priority(p_custom).0, 0);
        assert_eq!(config_file_priority(p_conf_d).0, 0);
        assert_eq!(config_file_priority(p_main).0, 2);
        assert_eq!(config_file_priority(p_base).0, 4);

        assert!(is_user_config_path(p_user));
        assert!(is_user_config_path(p_custom));
        assert!(is_user_config_path(p_conf_d));
        assert!(!is_user_config_path(p_base));
    }
}
