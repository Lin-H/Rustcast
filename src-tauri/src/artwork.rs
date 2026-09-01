use std::path::PathBuf;

use crate::opml::ArtworkCache;

/// 封面缓存状态：目录在启动时确定（与 rustcast.db 同级的便携式布局）。
pub struct ArtworkState {
    cache: ArtworkCache,
}

impl ArtworkState {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            cache: ArtworkCache::new(&base_dir),
        }
    }
}

/// 可执行文件所在目录；数据库文件 rustcast.db 也放在这里。
pub fn exe_base_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法确定应用位置: {e}"))?;
    exe.parent()
        .map(PathBuf::from)
        .ok_or_else(|| "无法确定应用所在目录".to_owned())
}

/// 拉取封面到本地缓存，返回绝对路径；失败返回 None（前端回落远程 URL）。
#[tauri::command]
pub async fn cache_artwork_command(
    artwork: tauri::State<'_, ArtworkState>,
    url: String,
) -> Result<Option<String>, String> {
    match artwork.cache.get_or_download(&url).await {
        Ok(path) => Ok(Some(path)),
        Err(error) => {
            eprintln!("[rustcast] 封面缓存失败: {error}");
            Ok(None)
        }
    }
}
