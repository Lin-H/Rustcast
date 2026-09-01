use std::path::Path;
use quick_xml::events::Event;
use quick_xml::Reader;
use quick_xml::XmlVersion;
use sha2::{Digest, Sha256};
use url::Url;

use crate::db::FeedSummaryDto;

const OPML_XML_DECL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;
const OPML_BODY_OPEN: &str = "<opml version=\"2.0\">\n  <head>\n    <title>Rustcast 订阅</title>\n  </head>\n  <body>";

#[derive(Debug)]
pub struct OpmlOutline {
    pub xml_url: String,
}

/// 从 OPML 文件字节中提取所有带 xmlUrl 的 outline。
/// 返回 (text, xmlUrl) 列表，跳过纯目录节点；解析完全不成功时返回 Err。
pub fn parse_opml(bytes: &[u8]) -> Result<Vec<OpmlOutline>, String> {
    let content = String::from_utf8_lossy(bytes);
    let trimmed = content.trim();
    if !trimmed.starts_with('<') {
        return Err("不是有效的 OPML 文件".to_owned());
    }

    let mut reader = Reader::from_str(trimmed);
    reader.config_mut().trim_text(true);

    let mut outlines = Vec::new();
    let mut buf = Vec::new();
    let mut saw_opml = false;
    let mut saw_outline = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let name = event.name();
                if name.local_name().as_ref() == "opml" {
                    saw_opml = true;
                }
                if name.local_name().as_ref() != "outline" {
                    continue;
                }
                saw_outline = true;

                let mut xml_url: Option<String> = None;
                for attr in event.attributes() {
                    let attr = attr.map_err(|e| format!("OPML 属性读取失败: {e}"))?;
                    if attr.key.local_name().as_ref() == "xmlUrl" {
                        xml_url = Some(
                            attr.normalized_value(XmlVersion::Implicit1_0)
                                .map_err(|e| format!("OPML 属性解码失败: {e}"))?
                                .into_owned(),
                        );
                    }
                }

                if let Some(url) = xml_url {
                    outlines.push(OpmlOutline { xml_url: url });
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(format!("OPML 解析失败: {e}")),
        }
        buf.clear();
    }

    if !saw_opml {
        return Err("缺少 <opml> 根节点，不是有效的 OPML 文件".to_owned());
    }
    if !saw_outline && outlines.is_empty() {
        return Err("OPML 中没有找到任何订阅条目".to_owned());
    }

    Ok(outlines)
}

fn xml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// 把订阅列表渲染为 OPML 2.0 文本。
pub fn render_opml(feeds: &[FeedSummaryDto]) -> String {
    let mut out = String::new();
    out.push_str(OPML_XML_DECL);
    out.push('\n');
    out.push_str(OPML_BODY_OPEN);

    for feed in feeds {
        out.push_str("\n    <outline type=\"rss\" text=\"");
        out.push_str(&xml_escape(&feed.title));
        out.push_str("\" xmlUrl=\"");
        out.push_str(&xml_escape(&feed.url));
        out.push_str("\"/>");
    }

    out.push_str("\n  </body>\n</opml>\n");
    out
}

fn content_type_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return Some("avif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    if bytes.starts_with(b"GIF8") {
        return Some("gif");
    }
    None
}

fn artwork_cache_dir(base_dir: &Path) -> std::path::PathBuf {
    base_dir.join("artwork-cache")
}

/// 为远程图片 URL 生成本地缓存文件名（sha256 + 扩展名）。
fn cache_file_name(url: &str, bytes: &[u8]) -> String {
    let digest = Sha256::digest(url.as_bytes());
    let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let ext = content_type_extension(bytes).unwrap_or("img");
    format!("{hash}.{ext}")
}

pub struct ArtworkCache {
    directory: std::path::PathBuf,
}

impl ArtworkCache {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            directory: artwork_cache_dir(base_dir),
        }
    }

    fn client() -> Result<reqwest::Client, String> {
        reqwest::Client::builder()
            .user_agent("Rustcast/0.2")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("封面下载客户端创建失败: {e}"))
    }

    fn sanitize_image_url(raw: &str) -> Result<String, String> {
        let url = Url::parse(raw).map_err(|_| "封面 URL 无效".to_owned())?;
        match url.scheme() {
            "https" => Ok(url.to_string()),
            "http" => {
                let mut upgraded = url;
                upgraded
                    .set_scheme("https")
                    .map_err(|_| "封面 URL 协议不支持".to_owned())?;
                Ok(upgraded.to_string())
            }
            _ => Err("封面 URL 只支持 HTTP/HTTPS".to_owned()),
        }
    }

    /// 命中缓存返回已存在文件路径；否则下载并写入缓存。
    /// 下载失败返回 Err（调用方自行回落远程 URL）。
    pub async fn get_or_download(&self, raw_url: &str) -> Result<String, String> {
        let url = Self::sanitize_image_url(raw_url)?;

        // 先按 url hash 探测任意扩展的命中。
        let digest = Sha256::digest(url.as_bytes());
        let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        if let Some(path) = self.find_cached(&hash) {
            return Ok(path.to_string_lossy().into_owned());
        }

        let response = Self::client()?
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("封面下载失败: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("封面下载失败: HTTP {}", response.status()));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("封面内容读取失败: {e}"))?;
        if content_type_extension(&bytes).is_none() {
            return Err("封面内容不是受支持的图片格式".to_owned());
        }

        let file_name = cache_file_name(&url, &bytes);
        let path = self.directory.join(&file_name);
        if let Some(existing) = self.find_cached(&hash) {
            // 并发下载竞态：别的请求已写好。
            return Ok(existing.to_string_lossy().into_owned());
        }

        tokio::fs::create_dir_all(&self.directory)
            .await
            .map_err(|e| format!("封面缓存目录创建失败: {e}"))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| format!("封面缓存写入失败: {e}"))?;

        Ok(path.to_string_lossy().into_owned())
    }

    fn find_cached(&self, hash: &str) -> Option<std::path::PathBuf> {
        let dir = self.directory.read_dir().ok()?;
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(hash) {
                return Some(entry.path());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(id: &str, title: &str, url: &str) -> FeedSummaryDto {
        FeedSummaryDto {
            id: id.to_owned(),
            url: url.to_owned(),
            title: title.to_owned(),
            description: None,
            logo_url: None,
            episode_count: 0,
            last_refreshed_at: None,
            last_error: None,
        }
    }

    const SAMPLE_OPML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Subs</title></head>
  <body>
    <outline text="Tech" title="Tech">
      <outline type="rss" text="Syntax FM" xmlUrl="https://feed.syntax.fm/" htmlUrl="https://syntax.fm"/>
    </outline>
    <outline type="rss" text="Waveform" xmlUrl="https://www.theverge.com/rss/waveform.xml"/>
  </body>
</opml>"#;

    #[test]
    fn parses_nested_and_flat_outlines() {
        let outlines = parse_opml(SAMPLE_OPML.as_bytes()).unwrap();
        assert_eq!(outlines.len(), 2);
        assert_eq!(outlines[0].xml_url, "https://feed.syntax.fm/");
        assert_eq!(outlines[1].xml_url, "https://www.theverge.com/rss/waveform.xml");
    }

    #[test]
    fn skips_directory_outlines_without_xml_url() {
        let outlines = parse_opml(SAMPLE_OPML.as_bytes()).unwrap();
        // "Tech" 目录节点无 xmlUrl，被跳过。
        assert!(outlines.iter().all(|o| !o.xml_url.is_empty()));
        assert!(!outlines.iter().any(|o| o.xml_url == "Tech"));
    }

    #[test]
    fn rejects_non_opml_content() {
        assert!(parse_opml(b"hello world").is_err());
        assert!(parse_opml(b"<html><body>no</body></html>").is_err());
        let empty = parse_opml(
            br#"<?xml version="1.0"?><opml version="2.0"><head/><body></body></opml>"#,
        );
        assert!(empty.is_err());
    }

    #[test]
    fn renders_and_round_trips_opml() {
        let feeds = vec![
            feed("a", "Syntax FM", "https://feed.syntax.fm/"),
            feed("b", "A & B <播客>", "https://example.com/feed?a=1&b=2"),
        ];
        let xml = render_opml(&feeds);
        assert!(xml.contains("<opml version=\"2.0\">"));
        assert!(xml.contains("text=\"A &amp; B &lt;播客&gt;\""));
        assert!(xml.contains("xmlUrl=\"https://example.com/feed?a=1&amp;b=2\""));

        let parsed = parse_opml(xml.as_bytes()).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].xml_url, "https://feed.syntax.fm/");
        assert_eq!(parsed[1].xml_url, "https://example.com/feed?a=1&b=2");
    }

    #[test]
    fn caches_by_url_hash_with_extension_change() {
        // PNG 魔数 → png 扩展；同一 URL 重新以 JPG 命中（hash 前缀匹配）。
        let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
        let name_png = cache_file_name("https://x/img", &png);
        assert!(name_png.ends_with(".png"));

        let jpg = [0xFFu8, 0xD8, 0xFF, 0xE0, 4, 5, 6];
        let name_jpg = cache_file_name("https://x/img", &jpg);
        assert!(name_jpg.ends_with(".jpg"));

        let hash_png = &name_png[..name_png.len() - 4];
        let hash_jpg = &name_jpg[..name_jpg.len() - 4];
        assert_eq!(hash_png, hash_jpg);
    }

    #[test]
    fn detects_image_extensions() {
        assert_eq!(
            content_type_extension(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("png")
        );
        assert_eq!(content_type_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(content_type_extension(b"GIF89a"), Some("gif"));
        assert_eq!(
            content_type_extension(&[0, 0, 0, 0x20, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f']),
            Some("avif")
        );
        assert_eq!(
            content_type_extension(&[b'R', b'I', b'F', b'F', 0, 0, 0, 0, b'W', b'E', b'B', b'P']),
            Some("webp")
        );
        assert_eq!(content_type_extension(b"<html>no"), None);
    }
}
