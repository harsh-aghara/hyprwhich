// config.rs: Hyprland configuration parser (.conf and .lua) with variable expansion and humanizer

use std::collections::HashSet;
use std::env;
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
}
