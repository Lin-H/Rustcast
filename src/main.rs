use std::collections::HashMap;
use std::time::Duration;

use iced::font::{Font, Weight};
use iced::widget::{button, column, container, image, row, scrollable, slider, svg, text};
use iced::{Alignment, Element, Length, Padding, Size, Task};
use rustcast::feed::{Episode, Feed};
use rustcast::player::{Command, PlayerHandle, Snapshot};
use rustcast::theme;
use rustcast::{feed, player};

fn boot() -> (App, Task<Message>) {
    App::new()
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    app.update(message)
}

fn theme(_app: &App) -> iced::Theme {
    iced::Theme::Dark
}

fn subscription(app: &App) -> iced::Subscription<Message> {
    app.subscription()
}

fn main() -> iced::Result {
    iced::application(boot, update, view_root)
        .title("Rustcast")
        .theme(theme)
        .centered()
        .window_size(Size::new(1220.0, 820.0))
        .window(iced::window::Settings {
            min_size: Some(Size::new(960.0, 620.0)),
            ..iced::window::Settings::default()
        })
        .exit_on_close_request(true)
        .subscription(subscription)
        .run()
}

// ---------- messages ----------

/// Free function wrapper so `view` satisfies the HRTB `ViewFn` trait.
fn view_root(app: &App) -> Element<'_, Message> {
    app.view()
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    FeedLoaded(Result<Feed, String>),
    LogoLoaded(Result<(String, Vec<u8>), String>),
    CoverLoaded(Result<(String, Vec<u8>), String>),
    EpisodePressed(usize),
    TogglePlay,
    ScrubChanged(f32),
    ScrubCommitted,
    VolumeSet(f32),
    ShowMore,
}

// ---------- icons ----------

fn icon_play() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/icons/play.svg").to_vec())
}
fn icon_pause() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/icons/pause.svg").to_vec())
}
fn icon_volume() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/icons/volume.svg").to_vec())
}
fn icon_brand() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../assets/icons/brand.svg").to_vec())
}

// ---------- tiny layout helpers (spacers) ----------

fn vgap(h: f32) -> Element<'static, Message> {
    container(column![]).height(h).into()
}

fn hfill() -> Element<'static, Message> {
    container(column![]).width(Length::Fill).into()
}

fn vfill() -> Element<'static, Message> {
    container(column![]).height(Length::Fill).into()
}

// ---------- app state ----------

struct App {
    player: PlayerHandle,
    snap: Snapshot,
    volume: f32,

    feed: Option<Feed>,
    episodes: Vec<Episode>,
    loading_feed: bool,
    load_error: Option<String>,

    selected: Option<usize>,
    playing_idx: Option<usize>,
    expanded: Option<usize>,

    covers: HashMap<String, image::Handle>,
    logo: Option<image::Handle>,

    scrubbing: bool,
    scrub_value: f32,

    /// How many episode cards are rendered; grows via `ShowMore`.
    /// Rendering 600+ complex cards at once starves the iced event loop
    /// (especially in debug builds), so we paginate aggressively.
    visible_count: usize,
}

const PAGE_SIZE: usize = 60;
const PAGE_STEP: usize = 150;

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = Self {
            player: PlayerHandle::spawn(),
            snap: Snapshot {
                volume: 1.0,
                ..Snapshot::default()
            },
            volume: 1.0,
            feed: None,
            episodes: Vec::new(),
            loading_feed: true,
            load_error: None,
            selected: None,
            playing_idx: None,
            expanded: None,
            covers: HashMap::new(),
            logo: None,
            scrubbing: false,
            scrub_value: 0.0,
            visible_count: PAGE_SIZE,
        };
        let task = Task::perform(
            feed::fetch_feed(feed::SYNTAX_FEED_URL.to_owned()),
            Message::FeedLoaded,
        );
        (app, task)
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(250)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.snap = self.player.state.lock().unwrap().clone();
            }
            Message::FeedLoaded(result) => {
                return match result {
                    Ok(mut f) => {
                        let logo_task = f
                            .logo_url
                            .clone()
                            .map(|url| Task::perform(feed::fetch_image(url), Message::LogoLoaded))
                            .unwrap_or_else(Task::none);
                        self.episodes = std::mem::take(&mut f.episodes);
                        self.feed = Some(f);
                        self.loading_feed = false;
                        self.visible_count = PAGE_SIZE;
                        logo_task
                    }
                    Err(e) => {
                        self.loading_feed = false;
                        self.load_error = Some(e);
                        Task::none()
                    }
                };
            }
            Message::LogoLoaded(result) => {
                if let Ok((_, bytes)) = result {
                    self.logo = Some(image::Handle::from_bytes(bytes));
                }
            }
            Message::CoverLoaded(result) => {
                if let Ok((url, bytes)) = result {
                    self.covers.insert(url, image::Handle::from_bytes(bytes));
                }
            }
            Message::EpisodePressed(i) => {
                let Some(ep) = self.episodes.get(i) else {
                    return Task::none();
                };
                self.selected = Some(i);
                self.playing_idx = Some(i);
                if self.expanded != Some(i) {
                    self.expanded = Some(i);
                }
                self.scrubbing = false;
                self.player.send(Command::Load {
                    url: ep.audio_url.clone(),
                    duration_secs: ep.duration.map(|d| d.as_secs_f64()).unwrap_or(0.0),
                });
                if let Some(img) = ep.image_url.clone() {
                    if !self.covers.contains_key(&img) {
                        return Task::perform(feed::fetch_image(img), Message::CoverLoaded);
                    }
                }
            }
            Message::TogglePlay => {
                self.player.send(Command::TogglePlay);
            }
            Message::ScrubChanged(v) => {
                self.scrubbing = true;
                self.scrub_value = v;
            }
            Message::ScrubCommitted => {
                let target = if self.scrubbing {
                    self.scrub_value as f64
                } else {
                    self.snap.position.as_secs_f64()
                };
                self.scrubbing = false;
                self.player.send(Command::Seek { seconds: target });
            }
            Message::VolumeSet(v) => {
                self.volume = v;
                self.player.send(Command::SetVolume(v));
            }
            Message::ShowMore => {
                self.visible_count += PAGE_STEP;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            top_bar(),
            main_area(self),
            player_bar(self),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::root())
            .into()
    }
}

// ---------- formatting helpers ----------

fn bold() -> Font {
    Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    }
}

/// Hard character truncation (grapheme-perfect clamping is out of scope for M1).
fn clamp_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

fn fmt_time(d: Duration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn fmt_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn cover_view(handle: Option<&image::Handle>, side: f32) -> Element<'static, Message> {
    let inner: Element<'static, Message> = match handle {
        Some(h) => image(h.clone()).width(side).height(side).into(),
        None => container(svg(icon_brand()).width(side * 0.5).height(side * 0.5))
            .padding(side * 0.25)
            .style(theme::surface(theme::BG_ELEVATED))
            .into(),
    };
    container(inner)
        .width(side)
        .height(side)
        .style(theme::rounded_clip())
        .into()
}

// ---------- views ----------

fn top_bar() -> Element<'static, Message> {
    let brand = row![
        svg(icon_brand()).width(22).height(22),
        text("Rustcast").size(17).font(bold()).color(theme::TEXT_PRIMARY),
    ]
    .spacing(9)
    .align_y(Alignment::Center);

    let tagline = text("RSS 音频播放器 · M1")
        .size(12)
        .color(theme::TEXT_FAINT);

    container(
        row![brand, hfill(), tagline].align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 14.0,
        right: 22.0,
        bottom: 14.0,
        left: 22.0,
    })
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_PANEL)),
        ..iced::widget::container::Style::default()
    })
    .into()
}

fn main_area(app: &App) -> Element<'_, Message> {
    row![sidebar(app), episode_list(app)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn sidebar(app: &App) -> Element<'_, Message> {
    let body: Element<'_, Message> = match (&app.feed, &app.load_error) {
        (_, Some(err)) => column![
            text("订阅源加载失败")
                .size(15)
                .font(bold())
                .color(theme::TEXT_PRIMARY),
            vgap(6.0),
            text(clamp_chars(err, 160))
                .size(12)
                .color(theme::TEXT_SECONDARY),
        ]
        .spacing(2)
        .into(),
        (Some(f), _) => {
            let desc = f.description.as_deref().map(|d| clamp_chars(d, 90));
            let mut col = column![
                cover_view(app.logo.as_ref(), 76.0),
                vgap(10.0),
                text(f.title.clone())
                    .size(18)
                    .font(bold())
                    .color(theme::TEXT_PRIMARY),
                vgap(4.0),
                text(format!("{} 集", app.episodes.len()))
                    .size(12)
                    .color(theme::ACCENT),
            ]
            .spacing(2);
            if let Some(d) = desc {
                col = col.push(text(d).size(13).color(theme::TEXT_FAINT));
            }
            col.push(vfill()).spacing(4).into()
        }
        _ => column![text("正在加载订阅源…")
            .size(13)
            .color(theme::TEXT_SECONDARY)]
        .into(),
    };

    let add_hint = button(
        row![
            text("+").size(13).font(bold()).color(theme::TEXT_FAINT),
            text("添加订阅源（即将推出）")
                .size(12)
                .color(theme::TEXT_FAINT),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding(7)
    .style(theme::ghost_button());

    container(
        container(
            column![body, vfill(), container(add_hint).width(Length::Fill)]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(16)
        .style(theme::card()),
    )
    .width(272.0)
    .height(Length::Fill)
    .padding(Padding {
        top: 12.0,
        right: 12.0,
        bottom: 12.0,
        left: 16.0,
    })
    .into()
}

fn episode_list(app: &App) -> Element<'_, Message> {
    let header = row![
        text("全部单集")
            .size(16)
            .font(bold())
            .color(theme::TEXT_PRIMARY),
        hfill(),
        text(if app.loading_feed {
            "加载中…".to_owned()
        } else {
            format!("{} 集", app.episodes.len())
        })
        .size(12)
        .color(theme::TEXT_FAINT),
    ]
    .align_y(Alignment::Center);

    if app.loading_feed && app.episodes.is_empty() {
        return container(
            column![
                vfill(),
                svg(icon_brand()).width(46).height(46),
                vgap(12.0),
                text("正在拉取 Syntax FM 订阅…")
                    .size(13)
                    .color(theme::TEXT_SECONDARY),
                vfill(),
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(24)
        .into();
    }

    let cards: Vec<Element<'_, Message>> = {
        let end = app.visible_count.min(app.episodes.len());
        let mut cards: Vec<Element<'_, Message>> = app.episodes[..end]
            .iter()
            .enumerate()
            .map(|(i, _)| episode_card(app, i))
            .collect();
        if end < app.episodes.len() {
            let remaining = app.episodes.len() - end;
            cards.push(
                container(
                    button(
                        text(format!("显示更多单集（还有 {remaining} 集）"))
                            .size(13)
                            .color(theme::TEXT_SECONDARY),
                    )
                    .padding(Padding {
                        top: 10.0,
                        right: 18.0,
                        bottom: 10.0,
                        left: 18.0,
                    })
                    .style(theme::ghost_button())
                    .on_press(Message::ShowMore),
                )
                .width(Length::Fill)
                .center_x(Length::Fill)
                .into(),
            );
        }
        cards
    };

    let list = scrollable(
        column(cards)
            .spacing(10)
            .padding(Padding {
                top: 4.0,
                right: 18.0,
                bottom: 24.0,
                left: 4.0,
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    container(
        column![header, list]
            .spacing(12)
            .padding(Padding {
                top: 16.0,
                right: 20.0,
                bottom: 0.0,
                left: 8.0,
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn episode_card(app: &App, idx: usize) -> Element<'_, Message> {
    let Some(ep) = app.episodes.get(idx) else {
        return column![].into();
    };
    let is_playing_row = app.playing_idx == Some(idx);
    let is_expanded = app.expanded == Some(idx);

    let thumb: Element<'_, Message> = match &ep.image_url {
        Some(url) => {
            let handle = app.covers.get(url).or(app.logo.as_ref());
            cover_view(handle, 58.0)
        }
        None => cover_view(app.logo.as_ref(), 58.0),
    };

    let title_text = clamp_chars(&ep.title, 80);

    let mut info = column![
        text(title_text)
            .size(15)
            .font(bold())
            .color(if is_playing_row {
                theme::ACCENT
            } else {
                theme::TEXT_PRIMARY
            }),
        row![
            text(fmt_date(ep.published_ts))
                .size(11.5)
                .color(theme::TEXT_FAINT),
            text("·").size(11.5).color(theme::TEXT_FAINT),
            text(match ep.duration {
                Some(d) => fmt_time(d),
                None => "时长未知".into(),
            })
            .size(11.5)
            .color(theme::TEXT_SECONDARY),
        ]
        .spacing(6),
    ]
    .spacing(5);

    info = info.push(
        text(clamp_chars(
            &ep.description,
            if is_expanded { 600 } else { 96 },
        ))
        .size(13.5)
        .color(theme::TEXT_SECONDARY),
    );

    // Full show notes (content:encoded) for the episode being played back.
    if is_playing_row && !ep.article.is_empty() {
        info = info.push(vgap(6.0));
        info = info.push(
            container(
                scrollable(
                    text(clamp_chars(&ep.article, 8000))
                        .size(13.5)
                        .color(theme::TEXT_SECONDARY),
                )
                .width(Length::Fill)
                .height(210.0),
            )
            .width(Length::Fill)
            .padding(12)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::BG_ROOT)),
                border: iced::Border {
                    radius: theme::radius_card(),
                    width: 1.0,
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                },
                ..iced::widget::container::Style::default()
            }),
        );
    }

    let card_body = row![
        thumb,
        info.width(Length::Fill).spacing(6),
        play_state_badge(is_playing_row, app.snap.playing),
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    button(container(card_body).width(Length::Fill).padding(13))
        .width(Length::Fill)
        .style(theme::episode_button(is_playing_row))
        .on_press(Message::EpisodePressed(idx))
        .into()
}

fn play_state_badge(on_row: bool, playing: bool) -> Element<'static, Message> {
    if on_row {
        container(
            text(if playing { "播放中" } else { "已暂停" })
                .size(10.5)
                .color(theme::ACCENT),
        )
        .padding(Padding {
            top: 4.0,
            right: 9.0,
            bottom: 4.0,
            left: 9.0,
        })
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(
                theme::ACCENT.scale_alpha(if playing { 0.14 } else { 0.08 }),
            )),
            border: iced::Border {
                radius: theme::radius_pill(),
                width: 1.0,
                color: theme::ACCENT_DIM,
            },
            ..iced::widget::container::Style::default()
        })
        .into()
    } else {
        container(column![]).width(0.0).into()
    }
}

fn player_bar(app: &App) -> Element<'_, Message> {
    let loaded = app.playing_idx.and_then(|i| app.episodes.get(i));

    let bar_style = |_: &_| iced::widget::container::Style {
        background: Some(iced::Background::Color(theme::BG_PANEL)),
        ..iced::widget::container::Style::default()
    };

    let Some(ep) = loaded else {
        return container(
            row![text("在上方选择一集，即可开始流式收听")
                .size(12.5)
                .color(theme::TEXT_FAINT)]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding {
            top: 16.0,
            right: 26.0,
            bottom: 16.0,
            left: 26.0,
        })
        .style(bar_style)
        .into();
    };

    // progress cluster -------------------------------------------------
    let duration_secs = app
        .snap
        .duration
        .or(ep.duration)
        .unwrap_or(Duration::ZERO)
        .as_secs_f32()
        .max(1.0);
    let current = if app.scrubbing {
        Duration::from_secs_f64(app.scrub_value.max(0.0) as f64)
    } else {
        app.snap.position
    };

    let progress = slider(
        0.0..=duration_secs,
        if app.scrubbing {
            app.scrub_value.clamp(0.0, duration_secs)
        } else {
            current.as_secs_f32().clamp(0.0, duration_secs)
        },
        Message::ScrubChanged,
    )
    .step(1.0)
    .on_release(Message::ScrubCommitted)
    .width(Length::Fill);

    let progress_cluster = column![
        progress,
        row![
            text(fmt_time(current)).size(11).color(theme::ACCENT),
            hfill(),
            text(fmt_time(Duration::from_secs_f64(duration_secs as f64)))
                .size(11)
                .color(theme::TEXT_FAINT),
        ]
        .width(Length::Fill),
    ]
    .width(Length::Fill)
    .spacing(3);

    // transport ---------------------------------------------------------
    let transport_glyph = if app.snap.playing {
        icon_pause()
    } else {
        icon_play()
    };

    let play_btn = button(container(svg(transport_glyph).width(19).height(19)).padding(13))
        .on_press(Message::TogglePlay)
        .style(theme::play_button());

    let volume_pct = format!("{}%", (app.volume * 100.0).round() as i32);
    let volume_ctl = row![
        svg(icon_volume()).width(17).height(17),
        slider(0.0..=1.0, app.volume, Message::VolumeSet)
            .step(0.01)
            .width(112.0),
        text(volume_pct)
            .size(11)
            .color(theme::TEXT_SECONDARY),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let status_text = if app.snap.buffering {
        "缓冲中…"
    } else if app.snap.finished {
        "已播完"
    } else if app.snap.playing {
        "正在播放"
    } else {
        "已暂停"
    };

    let meta = column![
        text(clamp_chars(&ep.title, 42))
            .size(13.5)
            .font(bold())
            .color(theme::TEXT_PRIMARY),
        text(format!(
            "{} · {}",
            app.feed
                .as_ref()
                .map(|f| f.title.as_str())
                .unwrap_or("播客"),
            status_text
        ))
        .size(11)
        .color(theme::TEXT_SECONDARY),
    ]
    .spacing(3);

    let bar = row![
        cover_view(player_cover(app, ep), 52.0),
        container(meta).width(230.0),
        container(progress_cluster).width(Length::Fill),
        play_btn,
        volume_ctl,
    ]
    .spacing(20)
    .align_y(Alignment::Center);

    let wrapped: Element<'_, Message> = match &app.snap.error {
        Some(err) => column![
            text(err)
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.45, 0.45)),
            bar,
        ]
        .spacing(6)
        .into(),
        None => bar.into(),
    };

    container(wrapped)
        .width(Length::Fill)
        .padding(if app.snap.error.is_some() {
            Padding {
                top: 8.0,
                right: 26.0,
                bottom: 12.0,
                left: 26.0,
            }
        } else {
            Padding {
                top: 12.0,
                right: 26.0,
                bottom: 12.0,
                left: 26.0,
            }
        })
        .style(bar_style)
        .into()
}

fn player_cover<'a>(app: &'a App, ep: &'a Episode) -> Option<&'a image::Handle> {
    ep.image_url
        .as_ref()
        .and_then(|u| app.covers.get(u))
        .or_else(|| app.logo.as_ref())
}
