//! Theme helpers and color constants for GitKraft's dark UI.

use iced::color;
use iced::Color;

// ── Brand / accent ────────────────────────────────────────────────────────────

/// Primary accent (blue-ish).
pub const ACCENT: Color = color!(0x58, 0xA6, 0xFF);

/// Success / addition green.
pub const GREEN: Color = color!(0x3F, 0xB9, 0x50);

/// Danger / deletion red.
pub const RED: Color = color!(0xF8, 0x53, 0x49);

/// Warning / modified yellow-orange.
pub const YELLOW: Color = color!(0xD2, 0x9A, 0x22);

/// Muted text / context lines.
pub const MUTED: Color = color!(0x8B, 0x94, 0x9E);

/// Separator / border color.
pub const BORDER: Color = color!(0x30, 0x36, 0x3D);

// ── Surface colors ────────────────────────────────────────────────────────────

/// Main background.
pub const BG: Color = color!(0x0D, 0x11, 0x17);

/// Slightly lighter surface for panels / cards.
pub const SURFACE: Color = color!(0x16, 0x1B, 0x22);

/// Even lighter, used for hovered / selected rows.
pub const SURFACE_HIGHLIGHT: Color = color!(0x21, 0x26, 0x2D);

/// Header / toolbar background.
pub const HEADER_BG: Color = color!(0x10, 0x14, 0x1A);

/// Sidebar background.
pub const SIDEBAR_BG: Color = color!(0x12, 0x17, 0x1E);

/// Diff addition line background (faint green).
pub const DIFF_ADD_BG: Color = Color {
    r: 0.15,
    g: 0.30,
    b: 0.15,
    a: 1.0,
};

/// Diff deletion line background (faint red).
pub const DIFF_DEL_BG: Color = Color {
    r: 0.30,
    g: 0.12,
    b: 0.12,
    a: 1.0,
};

/// Diff hunk header background.
pub const DIFF_HUNK_BG: Color = Color {
    r: 0.15,
    g: 0.18,
    b: 0.28,
    a: 1.0,
};

// ── Text colors ───────────────────────────────────────────────────────────────

/// Primary text on dark backgrounds.
pub const TEXT_PRIMARY: Color = color!(0xE6, 0xED, 0xF3);

/// Secondary / dimmed text.
pub const TEXT_SECONDARY: Color = color!(0x8B, 0x94, 0x9E);

// ── Helpers ───────────────────────────────────────────────────────────────────

use iced::widget::container;
use iced::Background;

/// Style for a container with the standard surface background.
pub fn surface_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        ..Default::default()
    }
}

/// Style for a container with the sidebar background.
pub fn sidebar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SIDEBAR_BG)),
        ..Default::default()
    }
}

/// Style for a container with the header / toolbar background.
pub fn header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(HEADER_BG)),
        ..Default::default()
    }
}

/// Style for a selected / highlighted row.
pub fn selected_row_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE_HIGHLIGHT)),
        ..Default::default()
    }
}

/// Style for a diff addition line.
pub fn diff_add_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DIFF_ADD_BG)),
        ..Default::default()
    }
}

/// Style for a diff deletion line.
pub fn diff_del_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DIFF_DEL_BG)),
        ..Default::default()
    }
}

/// Style for a diff hunk header line.
pub fn diff_hunk_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(DIFF_HUNK_BG)),
        ..Default::default()
    }
}

/// Return the color corresponding to a file-status badge.
pub fn status_color(status: &gitkraft_core::FileStatus) -> Color {
    match status {
        gitkraft_core::FileStatus::New | gitkraft_core::FileStatus::Untracked => GREEN,
        gitkraft_core::FileStatus::Modified | gitkraft_core::FileStatus::Typechange => YELLOW,
        gitkraft_core::FileStatus::Deleted => RED,
        gitkraft_core::FileStatus::Renamed | gitkraft_core::FileStatus::Copied => ACCENT,
    }
}
