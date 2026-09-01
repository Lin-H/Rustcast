#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artwork;
mod db;
mod feed;
mod opml;

use std::path::PathBuf;

use db::{
    SaveProgressInput, add_feed, delete_feed, list_feed_summaries, load_feed, load_initial_state,
    refresh_feed, save_progress, set_selected_feed,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use turso::Database;

use crate::artwork::{ArtworkState, exe_base_dir};
use crate::opml::{OpmlOutline, parse_opml, render_opml};

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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOpmlResult {
    pub imported: usize,
    pub skipped: usize,
    pub failed: Vec<FailedImport>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedImport {
    pub url: String,
    pub error: String,
}

#[tauri::command]
async fn import_opml_command(app: tauri::AppHandle, database: State<'_, Database>) -> Result<ImportOpmlResult, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("OPML 订阅文件", &["opml", "xml"])
        .blocking_pick_file();
    let Some(path) = picked else {
        return Ok(ImportOpmlResult {
            imported: 0,
            skipped: 0,
            failed: Vec::new(),
        });
    };
    let path: PathBuf = path.into_path().map_err(|e| format!("所选路径无效: {e}"))?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("OPML 文件读取失败: {e}"))?;
    let outlines: Vec<OpmlOutline> = parse_opml(&bytes)?;

    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failed = Vec::new();

    for outline in outlines {
        match add_feed(&database, &outline.xml_url).await {
            Ok(result) => {
                if result.already_exists {
                    skipped += 1;
                } else {
                    imported += 1;
                }
            }
            Err(error) => failed.push(FailedImport {
                url: outline.xml_url,
                error,
            }),
        }
    }

    Ok(ImportOpmlResult {
        imported,
        skipped,
        failed,
    })
}

#[tauri::command]
async fn export_opml_command(app: tauri::AppHandle, database: State<'_, Database>) -> Result<Option<String>, String> {
    let feeds = list_feed_summaries(&database).await?;
    if feeds.is_empty() {
        return Err("没有订阅源可导出".to_owned());
    }

    let picked = app
        .dialog()
        .file()
        .add_filter("OPML 订阅文件", &["opml"])
        .set_file_name("rustcast-subscriptions.opml")
        .blocking_save_file();
    let Some(path) = picked else {
        return Ok(None);
    };
    let path: PathBuf = path.into_path().map_err(|e| format!("所选路径无效: {e}"))?;

    let xml = render_opml(&feeds);
    tokio::fs::write(&path, xml)
        .await
        .map_err(|e| format!("OPML 文件写入失败: {e}"))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let database = tauri::async_runtime::block_on(db::open_database())?;
            app.manage(database);

            let base_dir = exe_base_dir()?;
            app.manage(ArtworkState::new(base_dir.clone()));

            // 封面缓存目录加入 asset protocol scope，供 WebView <img> 读取。
            let artwork_dir = base_dir.join("artwork-cache");
            std::fs::create_dir_all(&artwork_dir)?;
            app.asset_protocol_scope()
                .allow_directory(&artwork_dir, false)?;

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
            save_progress_command,
            import_opml_command,
            export_opml_command,
            artwork::cache_artwork_command
        ])
        .run(tauri::generate_context!())
        .expect("Rustcast failed to start");
}
