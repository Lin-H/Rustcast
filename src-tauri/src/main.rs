#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod artwork;
mod audio_cache;
mod db;
mod feed;
mod opml;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use db::{
    SaveProgressInput, add_feed, delete_feed, list_feed_summaries, load_feed, load_initial_state,
    refresh_feed, reorder_feeds, save_progress, set_selected_feed,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;
use turso::Database;

use crate::artwork::{ArtworkState, exe_base_dir};
use crate::audio_cache::AudioCache;
use crate::opml::{OpmlOutline, parse_opml, render_opml};

/// 媒体协议的 URL 注册表：episode id → (远程 URL, content-type)。
/// 前端每次播放前调 ensure_audio_cache_command 注册，协议处理器从这里取源 URL。
#[derive(Default)]
pub struct MediaUrls(pub Mutex<HashMap<String, (String, String)>>);
pub type SharedMediaUrls = Arc<MediaUrls>;

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
async fn reorder_feeds_command(
    database: State<'_, Database>,
    feed_ids: Vec<String>,
) -> Result<(), String> {
    reorder_feeds(&database, &feed_ids).await
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
pub struct AudioCacheStatus {
    pub written: u64,
    pub total: Option<u64>,
    pub complete: bool,
}

/// 注册并启动音频缓存；返回初始进度。
#[tauri::command]
async fn ensure_audio_cache_command(
    cache: State<'_, Arc<AudioCache>>,
    urls: State<'_, SharedMediaUrls>,
    episode_id: String,
    url: String,
    content_type: Option<String>,
) -> Result<AudioCacheStatus, String> {
    urls.0.lock()
        .await
        .insert(
            episode_id.clone(),
            (url.clone(), content_type.unwrap_or_else(|| "audio/mpeg".to_owned())),
        );

    cache.ensure(&episode_id, &url).await?;
    let (written, total) = cache
        .status(&episode_id)
        .await
        .unwrap_or((0, None));
    Ok(AudioCacheStatus {
        written,
        total,
        complete: total.is_some_and(|t| written >= t && t > 0),
    })
}

/// 查询某集缓存进度（切回时刷新徽标用）。
#[tauri::command]
async fn audio_cache_status_command(
    cache: State<'_, Arc<AudioCache>>,
    urls: State<'_, SharedMediaUrls>,
    episode_id: String,
    url: String,
) -> Result<AudioCacheStatus, String> {
    if let Some((written, total)) = cache.status(&episode_id).await {
        return Ok(AudioCacheStatus {
            written,
            total,
            complete: total.is_some_and(|t| written >= t && t > 0),
        });
    }
    // 未开始：注册 URL 供后续协议请求使用。
    urls.0.lock()
        .await
        .insert(episode_id, (url, "audio/mpeg".to_owned()));
    Ok(AudioCacheStatus {
        written: 0,
        total: None,
        complete: false,
    })
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

/// 列出所有已完整缓存的 episode id（列表徽标用）。
#[tauri::command]
async fn list_cached_episodes_command(
    cache: State<'_, Arc<AudioCache>>,
) -> Result<Vec<String>, String> {
    Ok(cache.list_complete().await)
}

fn main() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("rustcast-media", |ctx, request, responder| {
            let cache = ctx
                .app_handle()
                .state::<Arc<AudioCache>>()
                .inner()
                .clone();
            let urls: Arc<MediaUrls> = ctx
                .app_handle()
                .state::<SharedMediaUrls>()
                .inner()
                .clone();
            let range_header = request
                .headers()
                .get("range")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);

            // rustcast-media://localhost/{id} 或 http://rustcast-media.localhost/{id}
            // （WebView2 workaround 形式）；取 path 末段作为 episode id。
            let mut episode_id = request
                .uri()
                .path()
                .trim_start_matches('/')
                .to_owned();
            if let Some((_, tail)) = episode_id.rsplit_once('/') {
                episode_id = tail.to_string();
            }

            tauri::async_runtime::spawn(async move {
                let response = serve_media(
                    cache,
                    urls,
                    episode_id,
                    range_header.as_deref(),
                )
                .await;
                responder.respond(response);
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let database = tauri::async_runtime::block_on(db::open_database())?;
            app.manage(database);

            let base_dir = exe_base_dir()?;
            app.manage(ArtworkState::new(base_dir.clone()));

            // 音频缓存：目录与数据库同层；注入 app handle 供事件广播。
            let audio_cache = AudioCache::new(&base_dir);
            {
                let handle = app.handle().clone();
                let cache = Arc::clone(&audio_cache);
                tauri::async_runtime::spawn(async move {
                    cache.set_app_handle(handle).await;
                });
            }
            app.manage(audio_cache);
            app.manage(Arc::new(MediaUrls::default()) as SharedMediaUrls);

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
            reorder_feeds_command,
            add_feed_command,
            refresh_feed_command,
            delete_feed_command,
            save_progress_command,
            import_opml_command,
            export_opml_command,
            artwork::cache_artwork_command,
            ensure_audio_cache_command,
            audio_cache_status_command,
            list_cached_episodes_command
        ])
        .run(tauri::generate_context!())
        .expect("Rustcast failed to start");
}

/// rustcast-media:// 协议处理器：命中本地缓存段读文件，否则按需拉取。
async fn serve_media(
    cache: Arc<AudioCache>,
    urls: Arc<MediaUrls>,
    episode_id: String,
    range_header: Option<&str>,
) -> tauri::http::Response<Vec<u8>> {
    let Some((source_url, content_type)) = urls.0.lock().await.get(&episode_id).cloned() else {
        return media_response(
            tauri::http::StatusCode::NOT_FOUND,
            None,
            None,
            "text/plain".to_owned(),
            b"episode not registered".to_vec(),
        );
    };

    // 总长：Range 语义需要，先拿缓存状态。
    let (_, total) = cache.status(&episode_id).await.unwrap_or((0, None));
    let total = match total {
        Some(t) => Some(t),
        None => {
            // 状态未知（例如应用重启后第一次访问）：ensure 触发探测后重查。
            let _ = cache.ensure(&episode_id, &source_url).await;
            cache.status(&episode_id).await.and_then(|(_, t)| t)
        }
    };

    let range = range_header
        .and_then(|h| audio_cache::parse_range_header(h, total));

    let (start, end) = match &range {
        Some(r) => (r.start, r.end),
        None => (0, None),
    };

    // 起点超出总长：416。
    if let Some(t) = total {
        if start >= t {
            return media_response(
                tauri::http::StatusCode::RANGE_NOT_SATISFIABLE,
                Some(format!("bytes */{t}")),
                None,
                "text/plain".to_owned(),
                b"range not satisfiable".to_vec(),
            );
        }
    }

    match cache
        .read_range(&episode_id, &source_url, start, end)
        .await
    {
        Ok((data, actual_total)) => {
            let len = data.len() as u64;
            let end = start + len.saturating_sub(1);
            let status = if range.is_some() {
                tauri::http::StatusCode::PARTIAL_CONTENT
            } else {
                tauri::http::StatusCode::OK
            };
            let content_range = range.map(|_| {
                format!("bytes {start}-{end}/{}", actual_total.unwrap_or(len))
            });
            let accept_ranges = Some("bytes".to_owned());
            media_response(status, content_range, accept_ranges, content_type, data)
        }
        Err(error) => {
            eprintln!("[rustcast] 媒体协议读取失败: {error}");
            media_response(
                tauri::http::StatusCode::INTERNAL_SERVER_ERROR,
                None,
                None,
                "text/plain".to_owned(),
                error.as_bytes().to_vec(),
            )
        }
    }
}

fn media_response(
    status: tauri::http::StatusCode,
    content_range: Option<String>,
    accept_ranges: Option<String>,
    content_type: String,
    body: Vec<u8>,
) -> tauri::http::Response<Vec<u8>> {
    let mut builder = tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Content-Length", body.len().to_string());
    if let Some(range) = content_range {
        builder = builder.header("Content-Range", range);
    }
    if let Some(accept) = accept_ranges {
        builder = builder.header("Accept-Ranges", accept);
    }
    builder.body(body).unwrap_or_else(|_| {
        tauri::http::Response::builder()
            .status(tauri::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(b"response build failed".to_vec())
            .expect("静态错误响应必然可构造")
    })
}
