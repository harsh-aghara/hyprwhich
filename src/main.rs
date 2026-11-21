// main.rs: hyprwhich CLI & Wayland Layer-Shell HUD entry point

mod config;
mod gui;
mod theme;

use std::env;
use std::io::{self, Write};
use std::time::Instant;

use config::{parse_all_configs, Keybind};

fn print_help() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(
        handle,
        r#"hyprwhich 1.0.0 - Wayland HUD & Keybind Discovery for Hyprland

USAGE:
    hyprwhich [OPTIONS]

OPTIONS:
    -h, --help       Print this help message and exit
    -v, --version    Print version information
    --list           Print all keybindings formatted in an ASCII table
    --json           Output all keybindings in structured JSON format
    --tsv            Output tab-separated values for piping
    --light          Force Light Mode palette (skips dynamic multi-signal detection)
    --bench          Benchmark parser latency and exit

KEYBOARD SHORTCUTS IN HUD:
    Type             Instant fuzzy search across keys, titles, dispatchers
    Tab / Shift+Tab  Cycle through category filter tabs
    ↑ / ↓            Navigate row items
    ↵ Enter          Dispatch selected command and close HUD
    Ctrl+C           Copy key combination to clipboard
    Esc              Clear search query (or exit if query is empty)
"#
    );
}

fn print_table(binds: &[Keybind]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if writeln!(
        handle,
        "{:<28} │ {:<12} │ {:<35} │ DISPATCHER / ARG",
        "COMBO", "CATEGORY", "DESCRIPTION"
    )
    .is_err()
    {
        return;
    }
    if writeln!(handle, "{}", "─".repeat(115)).is_err() {
        return;
    }
    for b in binds {
        let mut combo = b.mods.clone();
        combo.push(b.key.clone());
        let combo_str = combo.join(" + ");
        if writeln!(
            handle,
            "{:<28} │ {:<12} │ {:<35} │ {} {}",
            combo_str, b.category, b.desc, b.dispatcher, b.arg
        )
        .is_err()
        {
            return;
        }
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn print_json(binds: &[Keybind]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if writeln!(handle, "[").is_err() {
        return;
    }
    for (i, b) in binds.iter().enumerate() {
        let comma = if i + 1 < binds.len() { "," } else { "" };
        let mods_json: Vec<String> = b
            .mods
            .iter()
            .map(|m| format!("\"{}\"", escape_json(m)))
            .collect();
        if writeln!(handle, "  {{").is_err() {
            return;
        }
        if writeln!(handle, "    \"mods\": [{}],", mods_json.join(", ")).is_err() {
            return;
        }
        if writeln!(handle, "    \"key\": \"{}\",", escape_json(&b.key)).is_err() {
            return;
        }
        if writeln!(handle, "    \"desc\": \"{}\",", escape_json(&b.desc)).is_err() {
            return;
        }
        if writeln!(handle, "    \"category\": \"{}\",", escape_json(&b.category)).is_err() {
            return;
        }
        if writeln!(handle, "    \"dispatcher\": \"{}\",", escape_json(&b.dispatcher)).is_err() {
            return;
        }
        if writeln!(handle, "    \"arg\": \"{}\",", escape_json(&b.arg)).is_err() {
            return;
        }
        if writeln!(handle, "    \"is_custom\": {},", b.is_custom).is_err() {
            return;
        }
        if writeln!(handle, "    \"source\": \"{}\"", escape_json(&b.source)).is_err() {
            return;
        }
        if writeln!(handle, "  }}{}", comma).is_err() {
            return;
        }
    }
    let _ = writeln!(handle, "]");
}

fn print_tsv(binds: &[Keybind]) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if writeln!(
        handle,
        "MODS\tKEY\tCATEGORY\tDESCRIPTION\tDISPATCHER\tARG\tCUSTOM\tSOURCE"
    )
    .is_err()
    {
        return;
    }
    for b in binds {
        if writeln!(
            handle,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            b.mods.join("+"),
            b.key,
            b.category,
            b.desc,
            b.dispatcher,
            b.arg,
            b.is_custom,
            b.source
        )
        .is_err()
        {
            return;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("hyprwhich 1.0.0");
        return;
    }

    let mut show_list = false;
    let mut show_json = false;
    let mut show_tsv = false;
    let mut show_bench = false;
    let mut force_light = false;
    let mut positional_terms: Vec<String> = Vec::new();

    for arg in &args {
        match arg.as_str() {
            "--list" => show_list = true,
            "--json" => show_json = true,
            "--tsv" => show_tsv = true,
            "--bench" => show_bench = true,
            "--light" => force_light = true,
            opt if opt.starts_with('-') => {
                eprintln!("hyprwhich: unrecognized option '{}'\n", opt);
                print_help();
                std::process::exit(1);
            }
            term => positional_terms.push(term.to_string()),
        }
    }

    let initial_query = if !positional_terms.is_empty() {
        Some(positional_terms.join(" "))
    } else {
        None
    };

    let start = Instant::now();
    let binds = parse_all_configs();
    let parse_duration = start.elapsed();

    if show_bench {
        println!("Parsed {} Hyprland keybindings in {:?}", binds.len(), parse_duration);
        let custom_count = binds.iter().filter(|b| b.is_custom).count();
        println!("Found {} authentic user custom actions", custom_count);
        return;
    }

    if show_list {
        print_table(&binds);
        return;
    }

    if show_json {
        print_json(&binds);
        return;
    }

    if show_tsv {
        print_tsv(&binds);
        return;
    }

    if binds.is_empty() {
        eprintln!("hyprwhich: No keybindings found in Hyprland configs.");
        std::process::exit(1);
    }

    gui::run_gui(binds, force_light, initial_query);
}
