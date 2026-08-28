#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod feed;

use feed::{fetch_default_feed, FeedDto};

#[tauri::command]
async fn load_default_feed() -> Result<FeedDto, String> {
    fetch_default_feed().await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![load_default_feed])
        .run(tauri::generate_context!())
        .expect("Rustcast failed to start");
}
