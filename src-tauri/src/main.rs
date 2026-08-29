#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod feed;

use db::{
    SaveProgressInput, add_feed, delete_feed, list_feed_summaries, load_feed, load_initial_state,
    refresh_feed, save_progress, set_selected_feed,
};
use tauri::{Manager, State};
use turso::Database;

#[tauri::command]
async fn load_initial_state_command(
    database: State<'_, Database>,
) -> Result<db::AppStateDto, String> {
    load_initial_state(&database).await
}

#[tauri::command]
async fn list_feeds_command(
    database: State<'_, Database>,
) -> Result<Vec<db::FeedSummaryDto>, String> {
    list_feed_summaries(&database).await
}

#[tauri::command]
async fn load_feed_command(
    database: State<'_, Database>,
    feed_id: String,
) -> Result<db::FeedDetailDto, String> {
    load_feed(&database, &feed_id).await
}

#[tauri::command]
async fn set_selected_feed_command(
    database: State<'_, Database>,
    feed_id: String,
) -> Result<(), String> {
    set_selected_feed(&database, &feed_id).await
}

#[tauri::command]
async fn add_feed_command(
    database: State<'_, Database>,
    url: String,
) -> Result<db::AddFeedResult, String> {
    add_feed(&database, &url).await
}

#[tauri::command]
async fn refresh_feed_command(
    database: State<'_, Database>,
    feed_id: String,
) -> Result<db::RefreshFeedResult, String> {
    refresh_feed(&database, &feed_id).await
}

#[tauri::command]
async fn delete_feed_command(database: State<'_, Database>, feed_id: String) -> Result<(), String> {
    delete_feed(&database, &feed_id).await
}

#[tauri::command]
async fn save_progress_command(
    database: State<'_, Database>,
    input: SaveProgressInput,
) -> Result<(), String> {
    save_progress(&database, input).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let database = tauri::async_runtime::block_on(db::open_database())?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_initial_state_command,
            list_feeds_command,
            load_feed_command,
            set_selected_feed_command,
            add_feed_command,
            refresh_feed_command,
            delete_feed_command,
            save_progress_command
        ])
        .run(tauri::generate_context!())
        .expect("Rustcast failed to start");
}
