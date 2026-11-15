#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tgdiff::view::{self, ComparisonView};

/// Compares two pasted texts and returns their side-by-side comparison.
#[tauri::command]
fn compare_text(left: String, right: String) -> ComparisonView {
    view::build(&left, &right)
}

/// Starts the tgdiff desktop window and blocks until it closes.
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compare_text])
        .run(tauri::generate_context!())
        .expect("error while running tgdiff desktop");
}
