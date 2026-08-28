use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;

pub const SYNTAX_FEED_URL: &str = "https://feed.syntax.fm/";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedDto {
    pub title: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub episodes: Vec<EpisodeDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub article_html: String,
    pub published_ts: i64,
    pub duration_secs: Option<f64>,
    pub audio_url: Option<String>,
    pub image_url: Option<String>,
}

pub async fn fetch_default_feed() -> Result<FeedDto, String> {
    let client = reqwest::Client::builder()
        .user_agent("Rustcast/0.2")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("订阅源客户端创建失败: {e}"))?;

    let bytes = client
        .get(SYNTAX_FEED_URL)
        .send()
        .await
        .map_err(|e| format!("订阅源下载失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("订阅源响应异常: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("订阅源内容读取失败: {e}"))?;

    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Result<FeedDto, String> {
    let raw = feed_rs::parser::parse(std::io::Cursor::new(bytes))
        .map_err(|e| format!("RSS 解析失败: {e}"))?;

    let episodes = raw
        .entries
        .iter()
        .filter_map(|entry| {
            let title = entry
                .title
                .as_ref()
                .map(|t| strip_html(&t.content))
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| "未命名单集".to_owned());

            let id = entry_id(SYNTAX_FEED_URL, entry)?;

            Some(EpisodeDto {
                id,
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
                image_url: pick_image(entry),
            })
        })
        .collect();

    Ok(FeedDto {
        title: raw
            .title
            .map(|t| strip_html(&t.content))
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "未命名播客".to_owned()),
        description: raw
            .description
            .map(|d| strip_html(&d.content))
            .filter(|d| !d.trim().is_empty()),
        logo_url: raw.logo.as_ref().and_then(|image| normalize_media_url(&image.uri)),
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

    candidate.url.as_ref().and_then(|url| normalize_media_url(url.as_str()))
}

fn pick_image(entry: &feed_rs::model::Entry) -> Option<String> {
    entry
        .media
        .iter()
        .flat_map(|media| media.thumbnails.iter())
        .next()
        .and_then(|thumbnail| normalize_media_url(&thumbnail.image.uri))
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

    collapse_whitespace(&decode_entities(&out)).trim().to_owned()
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
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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
}
