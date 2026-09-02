use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

/// 分块大小：4 MiB，兼顾请求数与内存占用。
const CHUNK_SIZE: u64 = 4 * 1024 * 1024;

/// 单次协议响应的最大字节数（防止整文件读入内存）。
const MAX_RESPONSE_BYTES: u64 = CHUNK_SIZE * 2;

/// 下载状态：由顺序预取任务与按需下载任务共享。
#[derive(Debug)]
pub struct Download {
    /// 顺序已确认写盘的连续位置（无空洞前缀长度）。
    written: u64,
    /// 总长度；未知时为 None。
    total: Option<u64>,
    /// 分块索引 → 是否已落盘。
    chunks: HashMap<u64, bool>,
    /// 顺序预取任务是否在跑（单飞标记）。
    running: bool,
}

impl Download {
    fn new() -> Self {
        Self {
            written: 0,
            total: None,
            chunks: HashMap::new(),
            running: false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RangeRequest {
    pub start: u64,
    /// None = 到文件末尾。
    pub end: Option<u64>,
}

struct CacheEntry {
    state: Arc<Mutex<Download>>,
    /// 未完成文件路径（.part）。
    partial: PathBuf,
}

pub struct AudioCache {
    directory: PathBuf,
    entries: Mutex<HashMap<String, CacheEntry>>,
    /// Tauri 事件发射器；setup 完成后注入。
    app: Mutex<Option<AppHandle>>,
}
impl AudioCache {
    pub fn new(base_dir: &Path) -> Arc<Self> {
        Arc::new(Self {
            directory: base_dir.join("audio-cache"),
            entries: Mutex::new(HashMap::new()),
            app: Mutex::new(None),
        })
    }

    pub async fn set_app_handle(&self, handle: AppHandle) {
        *self.app.lock().await = Some(handle);
    }

    fn final_path(&self, key: &str) -> PathBuf {
        self.directory.join(format!("{key}.audio"))
    }

    fn partial_path(&self, key: &str) -> PathBuf {
        self.directory.join(format!("{key}.audio.part"))
    }

    /// 是否已有完整缓存文件，返回其大小。
    async fn completed_size(&self, key: &str) -> Option<u64> {
        let meta = tokio::fs::metadata(self.final_path(key)).await.ok()?;
        (meta.len() > 0).then_some(meta.len())
    }

    /// 查询缓存进度：(已连续可用字节数或完整大小, 总长度)。
    pub async fn status(&self, key: &str) -> Option<(u64, Option<u64>)> {
        if let Some(size) = self.completed_size(key).await {
            return Some((size, Some(size)));
        }

        let entries = self.entries.lock().await;
        let entry = entries.get(key)?;
        let state = entry.state.lock().await;
        Some((state.written, state.total))
    }

    /// 列出所有已完整缓存的 episode id（扫描目录里的 .audio 文件）。
    pub async fn list_complete(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut dir = match tokio::fs::read_dir(&self.directory).await {
            Ok(dir) => dir,
            Err(_) => return result,
        };
        while let Ok(Some(entry)) = dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".audio") {
                result.push(id.to_owned());
            }
        }
        result
    }

    /// 幂等启动缓存；已完整或已有任务在跑时直接返回。
    pub async fn ensure(&self, key: &str, url: &str) -> Result<(), String> {
        if self.completed_size(key).await.is_some() {
            return Ok(());
        }

        let mut entries = self.entries.lock().await;
        let (state, partial) = match entries.get(key) {
            Some(entry) => (Arc::clone(&entry.state), entry.partial.clone()),
            None => {
                let state = Arc::new(Mutex::new(Download::new()));
                let partial = self.partial_path(key);
                entries.insert(
                    key.to_owned(),
                    CacheEntry {
                        state: Arc::clone(&state),
                        partial: partial.clone(),
                    },
                );
                (state, partial)
            }
        };

        {
            let mut state = state.lock().await;
            if state.running {
                return Ok(());
            }
            state.running = true;
        }
        drop(entries);

        let url = url.to_owned();
        let key_owned = key.to_owned();
        let cache_dir = self.directory.clone();
        let app = self.app.lock().await.clone();

        // 单飞：标记在任务结束后（完成或失败）清除。
        tokio::spawn(async move {
            let result =
                run_sequential_download(&cache_dir, &key_owned, &url, &partial, &state, app.as_ref()).await;
            let mut state = state.lock().await;
            state.running = false;
            if let Err(error) = result {
                eprintln!("[rustcast] 音频缓存后台下载失败: {error}");
            }
        });
        Ok(())
    }

    /// 协议读取入口：保证 [start, min(start+MAX_RESPONSE, 连续可用末尾)) 可读后返回数据。
    /// 返回 (数据, 文件总长)。
    pub async fn read_range(
        self: &Arc<Self>,
        key: &str,
        url: &str,
        start: u64,
        end: Option<u64>,
    ) -> Result<(Vec<u8>, Option<u64>), String> {
        if let Some(size) = self.completed_size(key).await {
            // 完整文件：非 Range 请求也限长返回，Accept-Ranges 驱动后续分页拉取。
            let take = end
                .map(|e| e.saturating_sub(start) + 1)
                .unwrap_or(size.saturating_sub(start))
                .min(MAX_RESPONSE_BYTES);
            let data = read_file_span(
                &self.final_path(key),
                start,
                Some(start + take.saturating_sub(1)),
                size,
            )
            .await?;
            return Ok((data, Some(size)));
        }

        self.ensure(key, url).await?;
        self.wait_for_offset(key, url, start).await?;

        let entries = self.entries.lock().await;
        let entry = entries
            .get(key)
            .ok_or_else(|| "音频缓存条目缺失".to_owned())?;
        let state = entry.state.lock().await;
        let (written, total) = (state.written, state.total);

        // 连续可用末尾：start 落在已写前缀内则可用到 written；否则按需块内最多到该块末尾。
        let chunk_end = (start / CHUNK_SIZE + 1) * CHUNK_SIZE;
        let contiguous_end = if start < written {
            written
        } else {
            chunk_end.min(total.unwrap_or(chunk_end))
        };
        drop(state);
        drop(entries);

        let requested_end = end.unwrap_or(u64::MAX).min(total.unwrap_or(u64::MAX));
        let take = requested_end
            .saturating_sub(start)
            .min(contiguous_end.saturating_sub(start))
            .min(MAX_RESPONSE_BYTES)
            .max(1);

        let data = read_file_span(&self.partial_path(key), start, Some(start + take - 1), start + take).await?;
        Ok((data, total))
    }

    /// 等待 offset 所在分块可读（未就绪时触发按需下载）。
    async fn wait_for_offset(self: &Arc<Self>, key: &str, url: &str, offset: u64) -> Result<(), String> {
        let chunk_index = offset / CHUNK_SIZE;
        loop {
            if self.completed_size(key).await.is_some() {
                return Ok(());
            }

            {
                let entries = self.entries.lock().await;
                let Some(entry) = entries.get(key) else {
                    return Ok(());
                };
                let state = entry.state.lock().await;
                if state.total.is_some_and(|t| offset >= t) {
                    return Ok(());
                }
                if offset < state.written {
                    return Ok(());
                }
                if state.chunks.get(&chunk_index).copied().unwrap_or(false) {
                    return Ok(());
                }
            }

            self.download_chunk(key, url, chunk_index).await?;
        }
    }

    /// 按需下载一个分块（含去重检查；与顺序任务并发安全）。
    async fn download_chunk(self: &Arc<Self>, key: &str, url: &str, chunk: u64) -> Result<(), String> {
        {
            let entries = self.entries.lock().await;
            let Some(entry) = entries.get(key) else {
                return Ok(());
            };
            let state = entry.state.lock().await;
            if state.chunks.get(&chunk).copied().unwrap_or(false) {
                return Ok(());
            }
        }

        let start = chunk * CHUNK_SIZE;
        let end = start + CHUNK_SIZE - 1;
        let bytes = fetch_range(url, start, Some(end)).await?;

        let entries = self.entries.lock().await;
        let Some(entry) = entries.get(key) else {
            return Ok(());
        };
        let mut state = entry.state.lock().await;
        write_span(&entry.partial, start, &bytes).await?;
        state.chunks.insert(chunk, true);
        if start == state.written {
            state.written = start + bytes.len() as u64;
        }

        let (written, total) = (state.written, state.total);
        drop(state);
        drop(entries);
        self.emit_progress(key, written, total, false).await;
        Ok(())
    }

    async fn emit_progress(&self, key: &str, written: u64, total: Option<u64>, complete: bool) {
        let app = self.app.lock().await;
        if let Some(handle) = app.as_ref() {
            emit_event(handle, key, written, total, complete);
        }
    }
}

/// 独立事件发射（供后台顺序任务直接使用，不需 &AudioCache）。
fn emit_event(handle: &AppHandle, key: &str, written: u64, total: Option<u64>, complete: bool) {
    let payload = serde_json::json!({
        "episodeId": key,
        "written": written,
        "total": total,
        "complete": complete,
    });
    let _ = handle.emit("audio-cache-progress", payload);
}

/// 顺序预取：从 written 位置逐块下载到文件末尾，完成后 .part 改名。
async fn run_sequential_download(
    directory: &Path,
    key: &str,
    url: &str,
    partial: &Path,
    state: &Arc<Mutex<Download>>,
    app: Option<&AppHandle>,
) -> Result<(), String> {
    if let Some(parent) = directory.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("音频缓存目录创建失败: {e}"))?;
    }
    tokio::fs::create_dir_all(directory)
        .await
        .map_err(|e| format!("音频缓存目录创建失败: {e}"))?;

    let client = reqwest::Client::builder()
        .user_agent("Rustcast/0.4")
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("音频下载客户端创建失败: {e}"))?;

    // 用 0-0 探测请求拿 Content-Range 总长（比 HEAD 对 CDN 兼容性好）。
    let total = detect_total(url, &client).await;
    {
        let mut state = state.lock().await;
        state.total = total;
    }

    let final_path = directory.join(format!("{key}.audio"));

    let mut chunk_index = {
        let state = state.lock().await;
        state.written / CHUNK_SIZE
    };

    loop {
        let (skip, done) = {
            let state = state.lock().await;
            let done = state
                .total
                .is_some_and(|t| chunk_index * CHUNK_SIZE >= t);
            let skip = state.chunks.get(&chunk_index).copied().unwrap_or(false);
            (skip, done)
        };
        if done {
            break;
        }
        if skip {
            chunk_index += 1;
            continue;
        }

        let start = chunk_index * CHUNK_SIZE;
        let end = total.map(|t| (start + CHUNK_SIZE - 1).min(t.saturating_sub(1)));
        let bytes = fetch_range_with(url, start, end, &client).await?;

        {
            let mut state = state.lock().await;
            write_span(partial, start, &bytes).await?;
            state.chunks.insert(chunk_index, true);
            if start == state.written {
                state.written = start + bytes.len() as u64;
            }
            let written = state.written;
            let total_now = state.total;
            drop(state);
            if let Some(handle) = app {
                emit_event(handle, key, written, total_now, false);
            }
        }

        chunk_index += 1;
    }

    // 全部完成：改名并广播。
    let _ = tokio::fs::remove_file(&final_path).await;
    tokio::fs::rename(partial, &final_path)
        .await
        .map_err(|e| format!("音频缓存完成改名失败: {e}"))?;

    let size = tokio::fs::metadata(&final_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    if let Some(handle) = app {
        emit_event(handle, key, size, Some(size), true);
    }
    Ok(())
}

async fn detect_total(url: &str, client: &reqwest::Client) -> Option<u64> {
    let response = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await
        .ok()?;
    let content_range = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()?;
    // Content-Range: bytes 0-0/12345
    content_range
        .rsplit('/')
        .next()?
        .trim()
        .parse::<u64>()
        .ok()
}

async fn fetch_range(url: &str, start: u64, end: Option<u64>) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent("Rustcast/0.4")
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("音频下载客户端创建失败: {e}"))?;
    fetch_range_with(url, start, end, &client).await
}

async fn fetch_range_with(
    url: &str,
    start: u64,
    end: Option<u64>,
    client: &reqwest::Client,
) -> Result<Vec<u8>, String> {
    let range = match end {
        Some(end) => format!("bytes={start}-{end}"),
        None => format!("bytes={start}-"),
    };

    let response = client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .await
        .map_err(|e| format!("音频分块下载失败: {e}"))?;

    let status = response.status().as_u16();
    if status != 206 && !response.status().is_success() {
        return Err(format!("音频分块下载失败: HTTP {status}"));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("音频分块内容读取失败: {e}"))?;
    Ok(bytes.to_vec())
}

/// 在文件偏移处写入一段字节（不维护状态）。
async fn write_span(path: &Path, start: u64, bytes: &[u8]) -> Result<(), String> {
    use tokio::fs::OpenOptions;

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("音频缓存目录创建失败: {e}"))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .await
        .map_err(|e| format!("音频缓存文件打开失败: {e}"))?;

    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| format!("音频缓存定位失败: {e}"))?;
    file.write_all(bytes)
        .await
        .map_err(|e| format!("音频缓存写入失败: {e}"))?;
    file.flush()
        .await
        .map_err(|e| format!("音频缓存写入失败: {e}"))?;
    Ok(())
}

/// 读取文件 [start, min(end, hard_limit)) 跨度。
async fn read_file_span(
    path: &Path,
    start: u64,
    end: Option<u64>,
    hard_limit: u64,
) -> Result<Vec<u8>, String> {
    let meta = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("音频缓存文件不存在: {e}"))?;
    let file_len = meta.len();
    let take = end
        .map(|e| e.saturating_sub(start) + 1)
        .unwrap_or(file_len.saturating_sub(start))
        .min(hard_limit.saturating_sub(start))
        .min(file_len.saturating_sub(start));

    if take == 0 {
        return Ok(Vec::new());
    }

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("音频缓存文件打开失败: {e}"))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| format!("音频缓存定位失败: {e}"))?;
    let mut data = Vec::with_capacity(take as usize);
    file.take(take)
        .read_to_end(&mut data)
        .await
        .map_err(|e| format!("音频缓存读取失败: {e}"))?;
    Ok(data)
}

/// 解析 Range 头（支持 bytes=start-end / start- / -suffix）。
pub fn parse_range_header(header: &str, total: Option<u64>) -> Option<RangeRequest> {
    let value = header.strip_prefix("bytes=")?;
    let mut parts = value.splitn(2, '-');
    let start_str = parts.next()?.trim();
    let end_str = parts.next().unwrap_or("").trim();

    if start_str.is_empty() {
        let suffix: u64 = end_str.parse().ok()?;
        let total = total?;
        let start = total.saturating_sub(suffix);
        return Some(RangeRequest {
            start,
            end: Some(total.saturating_sub(1)),
        });
    }

    let start: u64 = start_str.parse().ok()?;
    let end = if end_str.is_empty() {
        None
    } else {
        Some(end_str.parse().ok()?)
    };
    Some(RangeRequest { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_byte_ranges() {
        assert_eq!(
            parse_range_header("bytes=0-99", Some(1000)),
            Some(RangeRequest { start: 0, end: Some(99) })
        );
        assert_eq!(
            parse_range_header("bytes=100-", Some(1000)),
            Some(RangeRequest { start: 100, end: None })
        );
        assert_eq!(
            parse_range_header("bytes=-50", Some(1000)),
            Some(RangeRequest { start: 950, end: Some(999) })
        );
        assert_eq!(parse_range_header("bytes=abc", None), None);
        assert_eq!(parse_range_header("items=0-99", None), None);
        assert_eq!(parse_range_header("bytes=-50", None), None);
    }

    #[tokio::test]
    async fn writes_and_reads_spans() {
        let dir = std::env::temp_dir().join(format!("rustcast-audio-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("test.audio");

        write_span(&path, 0, &[1, 2, 3, 4]).await.unwrap();
        write_span(&path, 4, &[5, 6, 7, 8]).await.unwrap();

        let data = read_file_span(&path, 2, Some(5), 8).await.unwrap();
        assert_eq!(data, vec![3, 4, 5, 6]);

        let data = read_file_span(&path, 0, None, 8).await.unwrap();
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6, 7, 8]);

        // hard_limit 限制读取跨度。
        let data = read_file_span(&path, 0, None, 4).await.unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);

        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_dir(&dir).await;
    }

    #[test]
    fn contiguous_end_logic() {
        // start < written → 可用到 written；否则最多到块末尾。
        let written = 10 * 1024 * 1024u64;
        let start = 1024u64;
        let chunk_end = (start / CHUNK_SIZE + 1) * CHUNK_SIZE;
        let contiguous = if start < written { written } else { chunk_end };
        assert_eq!(contiguous, written);

        let start2 = 20 * 1024 * 1024u64;
        let chunk_end2 = (start2 / CHUNK_SIZE + 1) * CHUNK_SIZE;
        let contiguous2 = if start2 < written { written } else { chunk_end2 };
        assert_eq!(contiguous2, 24 * 1024 * 1024);
    }
}
