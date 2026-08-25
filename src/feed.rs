use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Feed {
    pub title: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: String,
    pub title: String,
    /// Short description (prefers summary/description).
    pub description: String,
    /// Full show notes, plain text (from content:encoded / content).
    pub article: String,
    pub published_ts: i64,
    pub duration: Option<Duration>,
    pub audio_url: String,
    pub image_url: Option<String>,
}

pub const SYNTAX_FEED_URL: &str = "https://feed.syntax.fm/";

pub async fn fetch_feed(url: String) -> Result<Feed, String> {
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| format!("订阅源下载失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("订阅源响应异常: {e}"))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    parse(&bytes)
}

pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    reqwest::get(url)
        .await
        .map_err(|e| format!("资源下载失败: {e}"))?
        .error_for_status()
        .map_err(|e| format!("资源响应异常: {e}"))?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

/// Fetch an image while carrying its cache key through the result.
pub async fn fetch_image(url: String) -> Result<(String, Vec<u8>), String> {
    let bytes = fetch_bytes(&url).await?;
    Ok((url, bytes))
}

fn parse(bytes: &[u8]) -> Result<Feed, String> {
    let raw = feed_rs::parser::parse(std::io::Cursor::new(bytes))
        .map_err(|e| format!("RSS 解析失败: {e}"))?;

    let episodes = raw
        .entries
        .iter()
        .filter_map(|entry| {
            let audio = pick_audio(entry)?;
            let title = entry.title.as_ref().map(|t| strip_html(&t.content))?;
            let description = match entry.summary.as_ref() {
                Some(s) => strip_html(&s.content),
                None => entry
                    .content
                    .as_ref()
                    .and_then(|c| c.body.as_deref())
                    .map(strip_html)
                    .unwrap_or_default(),
            };
            let article = entry
                .content
                .as_ref()
                .and_then(|c| c.body.as_deref())
                .map(strip_html)
                .unwrap_or_default();
            let image = entry
                .media
                .iter()
                .flat_map(|m| m.thumbnails.iter())
                .next()
                .map(|t| t.image.uri.clone());
            let duration = entry
                .media
                .iter()
                .find_map(|m| m.duration)
                .or_else(|| {
                    entry
                        .media
                        .iter()
                        .flat_map(|m| m.content.iter())
                        .find_map(|c| c.duration)
                });

            Some(Episode {
                id: entry.id.clone(),
                title,
                description,
                article,
                published_ts: entry
                    .published
                    .or(entry.updated)
                    .map(|t| t.timestamp_millis() / 1000)
                    .unwrap_or(0),
                duration,
                audio_url: audio,
                image_url: image,
            })
        })
        .collect();

    Ok(Feed {
        title: raw
            .title
            .map(|t| strip_html(&t.content))
            .unwrap_or_else(|| "未命名播客".into()),
        description: raw.description.map(|d| strip_html(&d.content)),
        logo_url: raw.logo.map(|l| l.uri),
        episodes,
    })
}

fn pick_audio(entry: &feed_rs::model::Entry) -> Option<String> {
    let contents = entry.media.iter().flat_map(|m| m.content.iter());
    let best = contents
        .clone()
        .find(|c| c.content_type.as_ref().is_some_and(|t| t.ty() == "audio"))
        .or_else(|| {
            entry
                .media
                .iter()
                .flat_map(|m| m.content.iter())
                .next()
        })?;
    best.url.as_ref().map(|u| u.to_string())
}

/// Minimal HTML → plain-text conversion good enough for podcast notes.
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
            // skip comment blocks wholesale
            while let Some(c2) = chars.next() {
                if c2 == '>' {
                    break;
                }
            }
        }
    }
    let decoded = decode_entities(&out);
    collapse_whitespace(&decoded)
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#8217;", "'")
        .replace("&#8216;", "'")
        .replace("&#8220;", "\u{201C}")
        .replace("&#8221;", "\u{201D}")
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out.trim().to_owned()
}
