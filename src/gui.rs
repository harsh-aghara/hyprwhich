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


pub fn match_query(query: &str, text: &str) -> Option<i32> {
    if query.is_empty() { return Some(0); }
    if text.to_lowercase().contains(&query.to_lowercase()) {
        Some(100)
    } else {
        None
    }
}

pub fn highlight_fuzzy(text: &str, _query: &str) -> String {
    glib::markup_escape_text(text).to_string()
}
