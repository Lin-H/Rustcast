use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme, border::Radius};

pub const BG_ROOT: Color = Color::from_rgb(0x0E as f32 / 255.0, 0x10 as f32 / 255.0, 0x14 as f32 / 255.0);
pub const BG_PANEL: Color = Color::from_rgb(0x16 as f32 / 255.0, 0x19 as f32 / 255.0, 0x1F as f32 / 255.0);
pub const BG_CARD: Color = Color::from_rgb(0x1D as f32 / 255.0, 0x21 as f32 / 255.0, 0x29 as f32 / 255.0);
pub const BG_CARD_HOVER: Color = Color::from_rgb(0x24 as f32 / 255.0, 0x2A as f32 / 255.0, 0x34 as f32 / 255.0);
pub const BG_ELEVATED: Color = Color::from_rgb(0x22 as f32 / 255.0, 0x26 as f32 / 255.0, 0x30 as f32 / 255.0);

pub const ACCENT: Color = Color::from_rgb(0xFF as f32 / 255.0, 0xB4 as f32 / 255.0, 0x54 as f32 / 255.0);
pub const ACCENT_DIM: Color = Color::from_rgb(0x8A as f32 / 255.0, 0x62 as f32 / 255.0, 0x2E as f32 / 255.0);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0xEC as f32 / 255.0, 0xEF as f32 / 255.0, 0xF3 as f32 / 255.0);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0xA8 as f32 / 255.0, 0xAF as f32 / 255.0, 0xBA as f32 / 255.0);
pub const TEXT_FAINT: Color = Color::from_rgb(0x62 as f32 / 255.0, 0x6A as f32 / 255.0, 0x76 as f32 / 255.0);

fn radius(px: f32) -> Radius {
    Radius {
        top_left: px,
        top_right: px,
        bottom_right: px,
        bottom_left: px,
    }
}

pub fn radius_card() -> Radius {
    radius(12.0)
}

pub fn radius_pill() -> Radius {
    radius(999.0)
}

fn solid(color: Color) -> Option<Background> {
    Some(Background::Color(color))
}

pub fn root() -> impl Fn(&Theme) -> container::Style {
    |_theme| container::Style {
        background: solid(BG_ROOT),
        ..container::Style::default()
    }
}

pub fn panel() -> impl Fn(&Theme) -> container::Style {
    |_theme| container::Style {
        background: solid(BG_PANEL),
        ..container::Style::default()
    }
}

pub fn card() -> impl Fn(&Theme) -> container::Style {
    |_theme| container::Style {
        background: solid(BG_CARD),
        border: Border {
            radius: radius_card(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Card-style button used for episode rows.
pub fn episode_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (selected, status) {
            (_, button::Status::Pressed) => BG_ELEVATED,
            (true, _) => BG_ELEVATED,
            (false, button::Status::Hovered) => BG_CARD_HOVER,
            (false, _) => BG_CARD,
        };
        button::Style {
            background: solid(bg),
            text_color: TEXT_PRIMARY,
            border: Border {
                radius: radius_card(),
                width: if selected { 1.0 } else { 0.0 },
                color: if selected { ACCENT } else { Color::TRANSPARENT },
            },
            shadow: Default::default(),
            snap: false,
        }
    }
}

/// Circular accent play/pause button.
pub fn play_button() -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let scale = match status {
            button::Status::Hovered => 1.06,
            button::Status::Pressed => 0.96,
            _ => 1.0,
        };
        button::Style {
            background: solid(if matches!(status, button::Status::Disabled) {
                ACCENT_DIM
            } else {
                ACCENT
            }),
            text_color: BG_ROOT,
            border: Border {
                radius: radius(999.0),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: iced::Shadow {
                color: Color::from_rgba(
                    0xFF as f32 / 255.0,
                    0xB4 as f32 / 255.0,
                    0x54 as f32 / 255.0,
                    if scale > 1.0 { 0.35 } else { 0.18 },
                ),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 14.0 * scale,
            },
            snap: false,
        }
    }
}

pub fn ghost_button() -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, fg) = match status {
            button::Status::Hovered => (BG_CARD_HOVER, TEXT_PRIMARY),
            button::Status::Pressed => (BG_ELEVATED, TEXT_PRIMARY),
            button::Status::Disabled => (Color::TRANSPARENT, TEXT_FAINT),
            button::Status::Active => (Color::TRANSPARENT, TEXT_SECONDARY),
        };
        button::Style {
            background: solid(bg),
            text_color: fg,
            border: Border {
                radius: radius(8.0),
                ..Border::default()
            },
            shadow: Default::default(),
            snap: false,
        }
    }
}

/// Flat colored surface with rounded corners (image backdrops etc).
pub fn surface(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: solid(color),
        border: Border {
            radius: radius(10.0),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Rounded-corner mask so images blend into the dark UI.
pub fn rounded_clip() -> impl Fn(&Theme) -> container::Style {
    |_theme| container::Style {
        border: Border {
            radius: radius(10.0),
            ..Border::default()
        },
        ..container::Style::default()
    }
}
