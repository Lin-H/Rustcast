use std::time::Duration;

use quick_xml::events::Event;
use quick_xml::Reader;
use sha2::{Digest, Sha256};
use url::Url;

pub const DEFAULT_FEED_URL: &str = "https://feed.syntax.fm/";

#[derive(Debug, Clone)]
pub struct ParsedEpisode {
    pub entry_id: String,
    pub title: String,
    pub description: String,
    pub article_html: String,
    pub published_ts: i64,
    pub duration_secs: Option<f64>,
    pub audio_url: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedFeed {
    pub feed_id: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub episodes: Vec<ParsedEpisode>,
}

pub async fn fetch_and_parse_url(raw_url: &str) -> Result<ParsedFeed, String> {
    let url = normalize_feed_url(raw_url)?;
    let feed_id = hash_feed_url(&url)?;
    let client = reqwest::Client::builder()
        .user_agent("Rustcast/0.2")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("订阅源客户端创建失败: {e}"))?;

    let bytes = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("订阅源下载失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("订阅源响应异常: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("订阅源内容读取失败: {e}"))?;

    let host = Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "订阅源".to_owned());
    parse(&bytes, &url, &feed_id, &host)
}

fn parse(
    bytes: &[u8],
    feed_url: &str,
    feed_id: &str,
    fallback_title: &str,
) -> Result<ParsedFeed, String> {
    let raw = feed_rs::parser::parse(std::io::Cursor::new(bytes))
        .map_err(|e| format!("RSS 解析失败: {e}"))?;

    // 旁路预扫：feed-rs 不认识的命名空间扩展（libsyn:widescreen-image 等）
    // 里的单集封面，按 item 文档顺序与 entries 对齐。
    let extension_images = extract_extension_item_images(bytes);

    let episodes = raw
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let title = entry
                .title
                .as_ref()
                .map(|t| strip_html(&t.content))
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| "未命名单集".to_owned());

            let entry_id = entry_id(feed_url, entry)?;

            Some(ParsedEpisode {
                entry_id,
                title,
                description: entry
                    .summary
                    .as_ref()
                    .map(|s| strip_html(&s.content))
                    .unwrap_or_default(),
                article_html: entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.as_deref())
                    .unwrap_or_default()
                    .to_owned(),
                published_ts: entry
                    .published
                    .or(entry.updated)
                    .map(|t| t.timestamp_millis() / 1000)
                    .unwrap_or(0),
                duration_secs: entry
                    .media
                    .iter()
                    .find_map(|media| media.duration)
                    .or_else(|| {
                        entry
                            .media
                            .iter()
                            .flat_map(|media| media.content.iter())
                            .find_map(|content| content.duration)
                    })
                    .map(|duration| duration.as_secs_f64()),
                audio_url: pick_audio(entry),
                image_url: pick_image(entry)
                    .or_else(|| extension_images.get(index).and_then(|u| normalize_media_url(u))),
            })
        })
        .collect();

    Ok(ParsedFeed {
        feed_id: feed_id.to_owned(),
        url: feed_url.to_owned(),
        title: raw
            .title
            .map(|t| strip_html(&t.content))
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| fallback_title.to_owned()),
        description: raw
            .description
            .map(|d| strip_html(&d.content))
            .filter(|d| !d.trim().is_empty()),
        logo_url: raw
            .logo
            .as_ref()
            .and_then(|image| normalize_media_url(&image.uri)),
        episodes,
    })
}

fn entry_id(feed_url: &str, entry: &feed_rs::model::Entry) -> Option<String> {
    let id = entry.id.trim();
    if !id.is_empty() {
        return Some(id.to_owned());
    }

    let link = entry.links.first().map(|link| link.href.as_str());
    let title = entry.title.as_ref().map(|title| title.content.as_str());
    let timestamp = entry
        .published
        .or(entry.updated)
        .map(|time| time.timestamp_millis());

    let identity = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        feed_url,
        link.unwrap_or(""),
        title.unwrap_or(""),
        timestamp.unwrap_or(0)
    );

    if link.is_none() && title.is_none() && timestamp.is_none() {
        return None;
    }

    let digest = Sha256::digest(identity.as_bytes());
    Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn normalize_feed_url(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("请输入订阅源 URL".to_owned());
    }
    if value.chars().count() > 2048 {
        return Err("订阅源 URL 不能超过 2048 个字符".to_owned());
    }

    let mut url = Url::parse(value).map_err(|_| "订阅源 URL 格式无效".to_owned())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("订阅源 URL 只支持 HTTP 或 HTTPS".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("订阅源 URL 不能包含用户名或密码".to_owned());
    }
    if url.host_str().is_none() {
        return Err("订阅源 URL 缺少主机名".to_owned());
    }

    url.set_fragment(None);
    let normalized = url.to_string();
    if normalized.chars().count() > 2048 {
        return Err("订阅源 URL 规范化后超过 2048 个字符".to_owned());
    }

    Ok(normalized)
}

pub fn hash_feed_url(normalized_url: &str) -> Result<String, String> {
    let digest = Sha256::digest(normalized_url.as_bytes());
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn episode_key(feed_id: &str, entry_id: &str) -> String {
    let identity = format!("{feed_id}\u{1f}{entry_id}");
    let digest = Sha256::digest(identity.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn pick_audio(entry: &feed_rs::model::Entry) -> Option<String> {
    let candidate = entry
        .media
        .iter()
        .flat_map(|media| media.content.iter())
        .find(|content| {
            content
                .content_type
                .as_ref()
                .is_some_and(|media_type| media_type.ty() == "audio")
        })
        .or_else(|| {
            entry
                .media
                .iter()
                .flat_map(|media| media.content.iter())
                .next()
        })?;

    candidate
        .url
        .as_ref()
        .and_then(|url| normalize_media_url(url.as_str()))
}

fn pick_image(entry: &feed_rs::model::Entry) -> Option<String> {
    entry
        .media
        .iter()
        .flat_map(|media| media.thumbnails.iter())
        .next()
        .and_then(|thumbnail| normalize_media_url(&thumbnail.image.uri))
}

/// 预扫原始 XML：按 item 文档顺序提取 feed-rs 不识别的命名空间扩展里的封面。
/// 覆盖 libsyn:widescreen-image / libsyn:itunes-image / podcast:cover-art
/// （podcast namespace 提案）等 <prefix:*image href="..."/> 形态。
/// 与 feed-rs 的 entries 同为文档顺序，按索引对齐。
fn extract_extension_item_images(bytes: &[u8]) -> Vec<String> {
    let content = match std::str::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut images = Vec::new();
    let mut buf = Vec::new();
    let mut in_item = false;
    let mut current: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let name = event.name();
                let local_name = name.local_name();
                let local = local_name.as_ref();
                if !in_item {
                    if local == "item" {
                        in_item = true;
                        current = None;
                    }
                } else {
                    // 任意命名空间下、local name 以 image 结尾且带 href 的元素。
                    // 同时覆盖 Start 与 Empty（自闭合 <foo:image href="..." />）两种形态。
                    if local.ends_with("image") {
                        for attr in event.attributes().flatten() {
                            if attr.key.as_ref() == "href" {
                                if let Ok(value) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                                    current = Some(value.into_owned());
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let name = event.name();
                let local_name = name.local_name();
                let local = local_name.as_ref();
                if in_item && local == "item" {
                    images.push(current.take().unwrap_or_default());
                    in_item = false;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        buf.clear();
    }

    images
}

fn normalize_media_url(raw: &str) -> Option<String> {
    let mut url = Url::parse(raw).ok()?;
    match url.scheme() {
        "https" => Some(url.to_string()),
        "http" => {
            url.set_scheme("https").ok()?;
            Some(url.to_string())
        }
        _ => None,
    }
}

fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut depth = 0usize;
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }

        if c == '<' && chars.peek() == Some(&'!') {
            for skipped in chars.by_ref() {
                if skipped == '>' {
                    break;
                }
            }
            depth = depth.saturating_sub(1);
        }
    }

    collapse_whitespace(&decode_entities(&out))
        .trim()
        .to_owned()
}

fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }

        let rest = &value[chars.peek().map_or(value.len(), |(index, _)| *index)..];
        let entity = if let Some(end) = rest.find(';') {
            Some(&rest[..end])
        } else {
            None
        };

        let decoded = match entity {
            Some("amp") => Some('&'),
            Some("lt") => Some('<'),
            Some("gt") => Some('>'),
            Some("quot") => Some('"'),
            Some("apos") => Some('\''),
            Some("nbsp") => Some(' '),
            _ => None,
        };

        match decoded {
            Some(c) => {
                out.push(c);
                for _ in 0..entity.map_or(0, |item| item.chars().count() + 1) {
                    chars.next();
                }
            }
            None => out.push('&'),
        }
    }

    out
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_http_media_to_https() {
        assert_eq!(
            normalize_media_url("http://cdn.example.com/audio.mp3"),
            Some("https://cdn.example.com/audio.mp3".to_owned())
        );
    }

    #[test]
    fn preserves_https_media() {
        assert_eq!(
            normalize_media_url("https://cdn.example.com/audio.mp3?x=1"),
            Some("https://cdn.example.com/audio.mp3?x=1".to_owned())
        );
    }

    #[test]
    fn rejects_unsupported_media_schemes() {
        assert_eq!(normalize_media_url("ftp://cdn.example.com/audio.mp3"), None);
        assert_eq!(normalize_media_url("not a url"), None);
    }

    #[test]
    fn strips_html_for_plain_descriptions() {
        assert_eq!(strip_html("<p>Hello &amp; world</p>"), "Hello & world");
    }

    #[test]
    fn normalizes_feed_urls() {
        let normalized =
            normalize_feed_url(" https://Example.com:443/path?keep=1#section ").unwrap();
        assert_eq!(normalized, "https://example.com/path?keep=1");
        assert!(normalize_feed_url("file:///tmp/feed.xml").is_err());
        assert!(normalize_feed_url("https://user@example.com/feed").is_err());
    }

    #[test]
    fn creates_stable_episode_keys() {
        assert_eq!(episode_key("feed", "entry"), episode_key("feed", "entry"));
        assert_ne!(episode_key("feed", "entry"), episode_key("feed2", "entry"));
    }

    #[test]
    fn extracts_libsyn_extension_images_per_item() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:libsyn="https://libsyn.com/rss">
  <channel>
    <title>Test</title>
    <item>
      <title>Ep 1</title>
      <guid>ep-1</guid>
      <libsyn:widescreen-image href="https://cdn.example.com/ep1.png" />
    </item>
    <item>
      <title>Ep 2</title>
      <guid>ep-2</guid>
      <libsyn:itunes-image href="https://cdn.example.com/ep2.png" />
    </item>
    <item>
      <title>Ep 3</title>
      <guid>ep-3</guid>
    </item>
  </channel>
</rss>"#;

        let images = extract_extension_item_images(xml);
        assert_eq!(images.len(), 3);
        assert_eq!(images[0], "https://cdn.example.com/ep1.png");
        assert_eq!(images[1], "https://cdn.example.com/ep2.png");
        assert_eq!(images[2], "");
    }

    #[test]
    fn extension_images_fall_back_to_media_thumbnails_first() {
        // parse() 的对齐逻辑：媒体缩略图优先，扩展图仅兜底。
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/" xmlns:libsyn="https://libsyn.com/rss">
  <channel>
    <item>
      <title>Ep 1</title>
      <guid>ep-1</guid>
      <media:thumbnail url="https://cdn.example.com/media.png" />
      <libsyn:widescreen-image href="https://cdn.example.com/libsyn.png" />
    </item>
  </channel>
</rss>"#;

        let parsed = parse(xml, "https://example.com/feed", "feedid", "fallback").unwrap();
        assert_eq!(parsed.episodes.len(), 1);
        assert_eq!(
            parsed.episodes[0].image_url.as_deref(),
            Some("https://cdn.example.com/media.png")
        );
    }

    #[test]
    fn extension_images_used_when_no_media_thumbnail() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0" xmlns:libsyn="https://libsyn.com/rss">
  <channel>
    <item>
      <title>Ep 1</title>
      <guid>ep-1</guid>
      <libsyn:widescreen-image href="https://static.libsyn.com/p/assets/ep1.png" />
    </item>
  </channel>
</rss>"#;

        let parsed = parse(xml, "https://example.com/feed", "feedid", "fallback").unwrap();
        assert_eq!(parsed.episodes.len(), 1);
        assert_eq!(
            parsed.episodes[0].image_url.as_deref(),
            Some("https://static.libsyn.com/p/assets/ep1.png")
        );
    }
}
