use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use turso::{Database, Value, params_from_iter};

use crate::feed::{
    ParsedEpisode, episode_key, fetch_and_parse_url, hash_feed_url, normalize_feed_url,
};

const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_create_subscriptions",
    r#"
    CREATE TABLE IF NOT EXISTS feeds (
        id TEXT PRIMARY KEY,
        url TEXT NOT NULL UNIQUE,
        title TEXT NOT NULL,
        description TEXT,
        logo_url TEXT,
        added_at INTEGER NOT NULL,
        last_refreshed_at INTEGER,
        last_error TEXT
    );

    CREATE TABLE IF NOT EXISTS episodes (
        id TEXT PRIMARY KEY,
        feed_id TEXT NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
        entry_id TEXT NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL,
        article_html TEXT NOT NULL,
        published_ts INTEGER NOT NULL,
        duration_secs REAL,
        audio_url TEXT,
        image_url TEXT,
        first_seen_at INTEGER NOT NULL,
        last_seen_at INTEGER NOT NULL,
        UNIQUE(feed_id, entry_id)
    );

    CREATE INDEX IF NOT EXISTS idx_episodes_feed_published
        ON episodes(feed_id, published_ts DESC, id ASC);

    CREATE TABLE IF NOT EXISTS playback_progress (
        episode_id TEXT PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
        position_secs REAL NOT NULL,
        duration_secs REAL,
        completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
        updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_progress_updated
        ON playback_progress(updated_at DESC);

    CREATE TABLE IF NOT EXISTS app_state (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
)];

const KNOWN_MIGRATIONS: &[&str] = &["0001_create_subscriptions"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedSummaryDto {
    pub id: String,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub episode_count: i64,
    pub last_refreshed_at: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressDto {
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub completed: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeDto {
    pub id: String,
    pub feed_id: String,
    pub entry_id: String,
    pub title: String,
    pub description: String,
    pub article_html: String,
    pub published_ts: i64,
    pub duration_secs: Option<f64>,
    pub audio_url: Option<String>,
    pub image_url: Option<String>,
    pub progress: Option<ProgressDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedDetailDto {
    pub feed: FeedSummaryDto,
    pub episodes: Vec<EpisodeDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateDto {
    pub feeds: Vec<FeedSummaryDto>,
    pub selected_feed_id: Option<String>,
    pub selected_feed: Option<FeedDetailDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddFeedResult {
    pub feed: FeedDetailDto,
    pub already_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshFeedResult {
    pub feed: FeedDetailDto,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProgressInput {
    pub episode_id: String,
    pub position_secs: f64,
    pub duration_secs: Option<f64>,
    pub completed: bool,
}

pub async fn open_database() -> Result<Database, String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法确定应用位置: {e}"))?;
    let directory = exe
        .parent()
        .ok_or_else(|| "无法确定应用所在目录".to_owned())?;
    let path = directory.join("rustcast.db");
    let database = turso::Builder::new_local(path.to_string_lossy().as_ref())
        .build()
        .await
        .map_err(|e| format!("无法打开本地数据库: {e}"))?;
    initialize_database(&database).await?;
    Ok(database)
}

pub async fn initialize_database(database: &Database) -> Result<(), String> {
    let mut conn = connection(database).await?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );
        "#,
    )
    .await
    .map_err(db_error("创建迁移表失败"))?;

    let mut rows = conn
        .query("SELECT version FROM schema_migrations", ())
        .await
        .map_err(db_error("读取数据库版本失败"))?;
    let mut applied = HashSet::new();
    while let Some(row) = rows.next().await.map_err(db_error("读取数据库版本失败"))? {
        let version: String = row.get(0).map_err(db_error("读取数据库版本失败"))?;
        applied.insert(version);
    }

    for version in &applied {
        if !KNOWN_MIGRATIONS.contains(&version.as_str()) {
            return Err(format!("数据库版本过新：未知迁移 {version}"));
        }
    }

    for (version, sql) in MIGRATIONS {
        if applied.contains(*version) {
            continue;
        }

        let tx = conn
            .transaction()
            .await
            .map_err(db_error("开启数据库迁移失败"))?;
        tx.execute_batch(sql)
            .await
            .map_err(db_error("执行数据库迁移失败"))?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            (version.to_string(), now_secs()),
        )
        .await
        .map_err(db_error("记录数据库迁移失败"))?;
        tx.commit().await.map_err(db_error("提交数据库迁移失败"))?;
    }

    Ok(())
}

pub async fn connection(database: &Database) -> Result<turso::Connection, String> {
    let conn = database.connect().map_err(db_error("打开数据库连接失败"))?;
    conn.execute("PRAGMA foreign_keys = ON", ())
        .await
        .map_err(db_error("启用数据库外键失败"))?;
    Ok(conn)
}

pub async fn load_initial_state(database: &Database) -> Result<AppStateDto, String> {
    let mut conn = connection(database).await?;
    let mut feeds = query_all_feed_summaries(&conn).await?;

    if feeds.is_empty() {
        drop(conn);
        if let Err(error) = add_feed(database, crate::feed::DEFAULT_FEED_URL).await {
            eprintln!("[rustcast] 默认订阅源初始化失败: {error}");
        }
        conn = connection(database).await?;
        feeds = query_all_feed_summaries(&conn).await?;
    }

    let stored_id = app_state_value(&conn, "selected_feed_id").await?;
    let selected_feed_id = match stored_id {
        Some(id) if feeds.iter().any(|feed| feed.id == id) => Some(id),
        Some(_) if !feeds.is_empty() => {
            let id = feeds[0].id.clone();
            set_app_state_value(&conn, "selected_feed_id", &id).await?;
            Some(id)
        }
        Some(_) => {
            clear_app_state_value(&conn, "selected_feed_id").await?;
            None
        }
        None => None,
    };

    let selected_feed = match selected_feed_id.as_ref() {
        Some(id) => Some(load_feed_detail(&conn, id).await?),
        None => None,
    };

    Ok(AppStateDto {
        feeds,
        selected_feed_id,
        selected_feed,
    })
}

pub async fn list_feed_summaries(database: &Database) -> Result<Vec<FeedSummaryDto>, String> {
    let conn = connection(database).await?;
    query_all_feed_summaries(&conn).await
}

pub async fn load_feed(database: &Database, feed_id: &str) -> Result<FeedDetailDto, String> {
    let conn = connection(database).await?;
    load_feed_detail(&conn, feed_id).await
}

pub async fn set_selected_feed(database: &Database, feed_id: &str) -> Result<(), String> {
    let conn = connection(database).await?;
    let exists = feed_exists(&conn, feed_id).await?;
    if !exists {
        return Err("订阅源不存在或已删除".to_owned());
    }
    set_app_state_value(&conn, "selected_feed_id", feed_id).await
}

pub async fn add_feed(database: &Database, raw_url: &str) -> Result<AddFeedResult, String> {
    let normalized_url = normalize_feed_url(raw_url)?;
    let feed_id = hash_feed_url(&normalized_url)?;
    let conn = connection(database).await?;

    if feed_exists(&conn, &feed_id).await? {
        let feed = load_feed_detail(&conn, &feed_id).await?;
        return Ok(AddFeedResult {
            feed,
            already_exists: true,
        });
    }
    drop(conn);

    let parsed = fetch_and_parse_url(&normalized_url).await?;
    let mut conn = connection(database).await?;
    if feed_exists(&conn, &feed_id).await? {
        let feed = load_feed_detail(&conn, &feed_id).await?;
        return Ok(AddFeedResult {
            feed,
            already_exists: true,
        });
    }

    let tx = conn
        .transaction()
        .await
        .map_err(db_error("开启订阅写入事务失败"))?;
    tx.execute(
        r#"
        INSERT INTO feeds
            (id, url, title, description, logo_url, added_at, last_refreshed_at, last_error)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)
        "#,
        params_from_iter(vec![
            Value::from(parsed.feed_id.clone()),
            Value::from(parsed.url.clone()),
            Value::from(parsed.title.clone()),
            Value::from(parsed.description.clone()),
            Value::from(parsed.logo_url.clone()),
            Value::from(now_secs()),
            Value::from(now_secs()),
        ]),
    )
    .await
    .map_err(|e| {
        if e.to_string().to_lowercase().contains("unique") {
            "该订阅源已存在".to_owned()
        } else {
            format!("保存订阅源失败: {e}")
        }
    })?;

    let timestamp = now_secs();
    for episode in &parsed.episodes {
        insert_episode(&tx, &parsed.feed_id, episode, timestamp).await?;
    }
    tx.commit().await.map_err(db_error("保存订阅源失败"))?;

    let feed = load_feed_detail(&conn, &parsed.feed_id).await?;
    Ok(AddFeedResult {
        feed,
        already_exists: false,
    })
}

pub async fn refresh_feed(database: &Database, feed_id: &str) -> Result<RefreshFeedResult, String> {
    let old = {
        let conn = connection(database).await?;
        load_feed_detail(&conn, feed_id).await?
    };

    match fetch_and_parse_url(&old.feed.url).await {
        Ok(parsed) => {
            if parsed.feed_id != old.feed.id {
                return Err("订阅源 URL 与数据库记录不一致".to_owned());
            }

            let mut conn = connection(database).await?;
            let tx = conn
                .transaction()
                .await
                .map_err(db_error("开启刷新事务失败"))?;
            let timestamp = now_secs();
            tx.execute(
                r#"
                UPDATE feeds
                SET title = ?1,
                    description = COALESCE(?2, description),
                    logo_url = COALESCE(?3, logo_url),
                    last_refreshed_at = ?4,
                    last_error = NULL
                WHERE id = ?5
                "#,
                params_from_iter(vec![
                    Value::from(parsed.title.clone()),
                    Value::from(parsed.description.clone()),
                    Value::from(parsed.logo_url.clone()),
                    Value::from(timestamp),
                    Value::from(feed_id),
                ]),
            )
            .await
            .map_err(db_error("更新订阅源失败"))?;

            for episode in &parsed.episodes {
                upsert_episode(&tx, feed_id, episode, timestamp).await?;
            }
            tx.commit().await.map_err(db_error("提交刷新失败"))?;

            let feed = load_feed_detail(&conn, feed_id).await?;
            Ok(RefreshFeedResult { feed, error: None })
        }
        Err(error) => {
            {
                let conn = connection(database).await?;
                conn.execute(
                    "UPDATE feeds SET last_error = ?1 WHERE id = ?2",
                    params_from_iter(vec![Value::from(error.clone()), Value::from(feed_id)]),
                )
                .await
                .map_err(db_error("记录刷新错误失败"))?;
            }

            let conn = connection(database).await?;
            let feed = load_feed_detail(&conn, feed_id).await?;
            Ok(RefreshFeedResult {
                feed,
                error: Some(error),
            })
        }
    }
}

pub async fn delete_feed(database: &Database, feed_id: &str) -> Result<(), String> {
    let mut conn = connection(database).await?;
    let selected = app_state_value(&conn, "selected_feed_id").await?;
    let tx = conn
        .transaction()
        .await
        .map_err(db_error("开启删除事务失败"))?;

    let affected = tx
        .execute(
            "DELETE FROM feeds WHERE id = ?1",
            params_from_iter(vec![Value::from(feed_id)]),
        )
        .await
        .map_err(db_error("删除订阅源失败"))?;
    if affected == 0 {
        return Err("订阅源不存在或已删除".to_owned());
    }

    if selected.as_deref() == Some(feed_id) {
        tx.execute("DELETE FROM app_state WHERE key = 'selected_feed_id'", ())
            .await
            .map_err(db_error("清理选中状态失败"))?;
    }

    tx.commit().await.map_err(db_error("提交删除失败"))
}

pub async fn save_progress(database: &Database, input: SaveProgressInput) -> Result<(), String> {
    if !input.position_secs.is_finite() || input.position_secs < 0.0 {
        return Err("播放进度无效".to_owned());
    }

    let conn = connection(database).await?;
    let mut rows = conn
        .query(
            r#"
            SELECT e.feed_id, COALESCE(p.duration_secs, e.duration_secs)
            FROM episodes e
            LEFT JOIN playback_progress p ON p.episode_id = e.id
            WHERE e.id = ?1
            "#,
            params_from_iter(vec![Value::from(input.episode_id.clone())]),
        )
        .await
        .map_err(db_error("读取单集失败"))?;
    let Some(row) = rows.next().await.map_err(db_error("读取单集失败"))? else {
        return Err("单集不存在或已删除".to_owned());
    };

    let _feed_id: String = row.get(0).map_err(db_error("读取单集失败"))?;
    let effective_duration: Option<f64> = match input.duration_secs {
        Some(value) if value.is_finite() && value > 0.0 => Some(value),
        _ => row.get(1).map_err(db_error("读取时长失败"))?,
    };

    let mut position = input.position_secs;
    if let Some(duration) = effective_duration {
        position = position.min(duration.max(0.0));
    }

    conn.execute(
        r#"
        INSERT INTO playback_progress
            (episode_id, position_secs, duration_secs, completed, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(episode_id) DO UPDATE SET
            position_secs = excluded.position_secs,
            duration_secs = COALESCE(excluded.duration_secs, playback_progress.duration_secs),
            completed = excluded.completed,
            updated_at = excluded.updated_at
        "#,
        params_from_iter(vec![
            Value::from(input.episode_id),
            Value::from(position),
            Value::from(effective_duration),
            Value::from(input.completed),
            Value::from(now_secs()),
        ]),
    )
    .await
    .map_err(db_error("保存播放进度失败"))?;

    Ok(())
}

async fn query_all_feed_summaries(conn: &turso::Connection) -> Result<Vec<FeedSummaryDto>, String> {
    let mut rows = conn
        .query(
            r#"
            SELECT f.id, f.url, f.title, f.description, f.logo_url,
                   COUNT(e.id) AS episode_count,
                   f.last_refreshed_at, f.last_error
            FROM feeds f
            LEFT JOIN episodes e ON e.feed_id = f.id
            GROUP BY f.id
            ORDER BY f.added_at ASC, f.id ASC
            "#,
            (),
        )
        .await
        .map_err(db_error("读取订阅源失败"))?;

    let mut feeds = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_error("读取订阅源失败"))? {
        feeds.push(FeedSummaryDto {
            id: row.get(0).map_err(db_error("读取订阅源失败"))?,
            url: row.get(1).map_err(db_error("读取订阅源失败"))?,
            title: row.get(2).map_err(db_error("读取订阅源失败"))?,
            description: row.get(3).map_err(db_error("读取订阅源失败"))?,
            logo_url: row.get(4).map_err(db_error("读取订阅源失败"))?,
            episode_count: row.get(5).map_err(db_error("读取订阅源失败"))?,
            last_refreshed_at: row.get(6).map_err(db_error("读取订阅源失败"))?,
            last_error: row.get(7).map_err(db_error("读取订阅源失败"))?,
        });
    }
    Ok(feeds)
}

async fn load_feed_detail(
    conn: &turso::Connection,
    feed_id: &str,
) -> Result<FeedDetailDto, String> {
    let feed = get_feed_summary(conn, feed_id)
        .await?
        .ok_or_else(|| "订阅源不存在或已删除".to_owned())?;
    let mut rows = conn
        .query(
            r#"
            SELECT e.id, e.feed_id, e.entry_id, e.title, e.description,
                   e.article_html, e.published_ts, e.duration_secs,
                   e.audio_url, e.image_url,
                   p.position_secs, p.duration_secs, p.completed, p.updated_at
            FROM episodes e
            LEFT JOIN playback_progress p ON p.episode_id = e.id
            WHERE e.feed_id = ?1
            ORDER BY e.published_ts DESC, e.id ASC
            "#,
            (feed_id,),
        )
        .await
        .map_err(db_error("读取单集失败"))?;

    let mut episodes = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_error("读取单集失败"))? {
        let position: Option<f64> = row.get(10).map_err(db_error("读取单集失败"))?;
        let progress = if position.is_some() {
            Some(ProgressDto {
                position_secs: position.unwrap_or_default(),
                duration_secs: row.get(11).map_err(db_error("读取单集失败"))?,
                completed: row.get::<bool>(12).map_err(db_error("读取单集失败"))?,
                updated_at: row.get(13).map_err(db_error("读取单集失败"))?,
            })
        } else {
            None
        };

        episodes.push(EpisodeDto {
            id: row.get(0).map_err(db_error("读取单集失败"))?,
            feed_id: row.get(1).map_err(db_error("读取单集失败"))?,
            entry_id: row.get(2).map_err(db_error("读取单集失败"))?,
            title: row.get(3).map_err(db_error("读取单集失败"))?,
            description: row.get(4).map_err(db_error("读取单集失败"))?,
            article_html: row.get(5).map_err(db_error("读取单集失败"))?,
            published_ts: row.get(6).map_err(db_error("读取单集失败"))?,
            duration_secs: row.get(7).map_err(db_error("读取单集失败"))?,
            audio_url: row.get(8).map_err(db_error("读取单集失败"))?,
            image_url: row.get(9).map_err(db_error("读取单集失败"))?,
            progress,
        });
    }

    Ok(FeedDetailDto { feed, episodes })
}

async fn get_feed_summary(
    conn: &turso::Connection,
    feed_id: &str,
) -> Result<Option<FeedSummaryDto>, String> {
    let mut feeds = query_feed_summaries(
        conn,
        "SELECT id, url, title, description, logo_url, 0, last_refreshed_at, last_error FROM feeds WHERE id = ?1",
        (feed_id,),
    )
    .await?;
    Ok(feeds.pop())
}

async fn query_feed_summaries<P>(
    conn: &turso::Connection,
    sql: &str,
    params: P,
) -> Result<Vec<FeedSummaryDto>, String>
where
    P: turso::IntoParams,
{
    let mut rows = conn
        .query(sql, params)
        .await
        .map_err(db_error("读取订阅源失败"))?;
    let mut feeds = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_error("读取订阅源失败"))? {
        feeds.push(FeedSummaryDto {
            id: row.get(0).map_err(db_error("读取订阅源失败"))?,
            url: row.get(1).map_err(db_error("读取订阅源失败"))?,
            title: row.get(2).map_err(db_error("读取订阅源失败"))?,
            description: row.get(3).map_err(db_error("读取订阅源失败"))?,
            logo_url: row.get(4).map_err(db_error("读取订阅源失败"))?,
            episode_count: row.get(5).map_err(db_error("读取订阅源失败"))?,
            last_refreshed_at: row.get(6).map_err(db_error("读取订阅源失败"))?,
            last_error: row.get(7).map_err(db_error("读取订阅源失败"))?,
        });
    }
    Ok(feeds)
}

async fn feed_exists(conn: &turso::Connection, feed_id: &str) -> Result<bool, String> {
    let mut rows = conn
        .query("SELECT 1 FROM feeds WHERE id = ?1", (feed_id,))
        .await
        .map_err(db_error("检查订阅源失败"))?;
    Ok(rows
        .next()
        .await
        .map_err(db_error("检查订阅源失败"))?
        .is_some())
}

async fn insert_episode(
    conn: &turso::Connection,
    feed_id: &str,
    episode: &ParsedEpisode,
    timestamp: i64,
) -> Result<(), String> {
    let id = episode_key(feed_id, &episode.entry_id);
    conn.execute(
        r#"
        INSERT INTO episodes
            (id, feed_id, entry_id, title, description, article_html,
             published_ts, duration_secs, audio_url, image_url,
             first_seen_at, last_seen_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params_from_iter(vec![
            Value::from(id),
            Value::from(feed_id),
            Value::from(episode.entry_id.clone()),
            Value::from(episode.title.clone()),
            Value::from(episode.description.clone()),
            Value::from(episode.article_html.clone()),
            Value::from(episode.published_ts),
            Value::from(episode.duration_secs),
            Value::from(episode.audio_url.clone()),
            Value::from(episode.image_url.clone()),
            Value::from(timestamp),
            Value::from(timestamp),
        ]),
    )
    .await
    .map_err(|e| format!("保存单集失败: {e}"))?;
    Ok(())
}

async fn upsert_episode(
    conn: &turso::Connection,
    feed_id: &str,
    episode: &ParsedEpisode,
    timestamp: i64,
) -> Result<(), String> {
    let id = episode_key(feed_id, &episode.entry_id);
    conn.execute(
        r#"
        INSERT INTO episodes
            (id, feed_id, entry_id, title, description, article_html,
             published_ts, duration_secs, audio_url, image_url,
             first_seen_at, last_seen_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            article_html = excluded.article_html,
            published_ts = excluded.published_ts,
            duration_secs = excluded.duration_secs,
            audio_url = excluded.audio_url,
            image_url = excluded.image_url,
            last_seen_at = excluded.last_seen_at
        "#,
        params_from_iter(vec![
            Value::from(id),
            Value::from(feed_id),
            Value::from(episode.entry_id.clone()),
            Value::from(episode.title.clone()),
            Value::from(episode.description.clone()),
            Value::from(episode.article_html.clone()),
            Value::from(episode.published_ts),
            Value::from(episode.duration_secs),
            Value::from(episode.audio_url.clone()),
            Value::from(episode.image_url.clone()),
            Value::from(timestamp),
            Value::from(timestamp),
        ]),
    )
    .await
    .map_err(|e| format!("更新单集失败: {e}"))?;
    Ok(())
}

async fn app_state_value(conn: &turso::Connection, key: &str) -> Result<Option<String>, String> {
    let mut rows = conn
        .query("SELECT value FROM app_state WHERE key = ?1", (key,))
        .await
        .map_err(db_error("读取应用状态失败"))?;
    let Some(row) = rows.next().await.map_err(db_error("读取应用状态失败"))? else {
        return Ok(None);
    };
    row.get(0).map_err(db_error("读取应用状态失败"))
}

async fn set_app_state_value(
    conn: &turso::Connection,
    key: &str,
    value: &str,
) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT INTO app_state (key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params_from_iter(vec![Value::from(key), Value::from(value)]),
    )
    .await
    .map_err(db_error("保存应用状态失败"))?;
    Ok(())
}

async fn clear_app_state_value(conn: &turso::Connection, key: &str) -> Result<(), String> {
    conn.execute("DELETE FROM app_state WHERE key = ?1", (key,))
        .await
        .map_err(db_error("清理应用状态失败"))?;
    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

fn db_error(message: &'static str) -> impl Fn(turso::Error) -> String {
    move |error| format!("{message}: {error}")
}
