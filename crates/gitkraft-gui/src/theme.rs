//! Theme helpers for GitKraft's UI.
//!
//! Colours are now derived from the unified `gitkraft_core::AppTheme`
//! definitions so that both the GUI and TUI render the exact same palette for
//! every theme.  The old `from_theme()` constructor is kept as a convenience
//! fallback that maps an `iced::Theme` to the closest core theme index.

use iced::widget::{button, container, scrollable};
use iced::{Background, Color};
use std::cell::RefCell;

// ── ThemeColors ───────────────────────────────────────────────────────────────

/// A resolved set of colours derived from the active `iced::Theme`.
///
/// Create one at the top of each view function with
/// `let c = ThemeColors::from_theme(&state.theme);` and then reference
/// `c.accent`, `c.green`, etc. instead of the old hard-coded constants.
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub accent: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub muted: Color,
    pub bg: Color,
    pub surface: Color,
    pub surface_highlight: Color,
    pub header_bg: Color,
    pub sidebar_bg: Color,
    pub border: Color,
    pub selection: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub diff_add_bg: Color,
    pub diff_del_bg: Color,
    pub diff_hunk_bg: Color,
    pub error_bg: Color,
    pub graph_colors: [Color; 8],
}

/// Clamp a single channel to `[0.0, 1.0]`.
fn clamp(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Shift every RGB channel of `base` by `delta` (positive = lighter, negative = darker).
fn shift(base: Color, delta: f32) -> Color {
    Color {
        r: clamp(base.r + delta),
        g: clamp(base.g + delta),
        b: clamp(base.b + delta),
        a: base.a,
    }
}

/// Scale every RGB channel of `base` by `factor`.
#[cfg(test)]
fn scale(base: Color, factor: f32) -> Color {
    Color {
        r: clamp(base.r * factor),
        g: clamp(base.g * factor),
        b: clamp(base.b * factor),
        a: base.a,
    }
}

/// Convert a core [`gitkraft_core::Rgb`] to an [`iced::Color`].
fn rgb_to_iced(rgb: gitkraft_core::Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

/// Mix `base` with `tint` at the given `amount` (0.0 = all base, 1.0 = all tint).
fn mix(base: Color, tint: Color, amount: f32) -> Color {
    let inv = 1.0 - amount;
    Color {
        r: clamp(base.r * inv + tint.r * amount),
        g: clamp(base.g * inv + tint.g * amount),
        b: clamp(base.b * inv + tint.b * amount),
        a: 1.0,
    }
}

thread_local! {
    static THEME_CACHE: RefCell<Option<(String, ThemeColors)>> = const { RefCell::new(None) };
}

impl ThemeColors {
    /// Build a complete GUI colour set from the core's platform-agnostic theme.
    ///
    /// This is the **primary** constructor — it guarantees that the GUI renders
    /// the exact same palette as the TUI for every theme index.
    pub fn from_core(t: &gitkraft_core::AppTheme) -> Self {
        let bg = rgb_to_iced(t.background);
        let surface = rgb_to_iced(t.surface);
        let success = rgb_to_iced(t.success);
        let error = rgb_to_iced(t.error);
        let hunk = rgb_to_iced(t.diff_hunk);

        let sign: f32 = if t.is_dark { 1.0 } else { -1.0 };
        let surface_highlight = shift(surface, sign * 0.04);
        let header_bg = shift(bg, sign * 0.02);
        let sidebar_bg = shift(bg, sign * 0.03);

        // Diff backgrounds — faint tint of the semantic colour over the bg
        let tint_amount = if t.is_dark { 0.18 } else { 0.12 };
        let diff_add_bg = mix(bg, success, tint_amount);
        let diff_del_bg = mix(bg, error, tint_amount);
        let diff_hunk_bg = mix(bg, hunk, tint_amount);

        // Error banner background — faint tint of the error colour over the bg
        let error_bg = mix(bg, error, tint_amount);

        // Graph lane colours — convert all eight from the core theme
        let graph_colors = {
            let gc = &t.graph_colors;
            [
                rgb_to_iced(gc[0]),
                rgb_to_iced(gc[1]),
                rgb_to_iced(gc[2]),
                rgb_to_iced(gc[3]),
                rgb_to_iced(gc[4]),
                rgb_to_iced(gc[5]),
                rgb_to_iced(gc[6]),
                rgb_to_iced(gc[7]),
            ]
        };

        Self {
            accent: rgb_to_iced(t.accent),
            text_primary: rgb_to_iced(t.text_primary),
            text_secondary: rgb_to_iced(t.text_secondary),
            muted: rgb_to_iced(t.text_muted),
            bg,
            surface,
            surface_highlight,
            header_bg,
            sidebar_bg,
            border: rgb_to_iced(t.border),
            selection: rgb_to_iced(t.selection),
            green: success,
            red: error,
            yellow: rgb_to_iced(t.warning),
            diff_add_bg,
            diff_del_bg,
            diff_hunk_bg,
            error_bg,
            graph_colors,
        }
    }

    /// Derive colours from an `iced::Theme` by mapping it to the closest core
    /// theme and then calling [`from_core`](Self::from_core).
    ///
    /// This keeps backward-compatibility for any code that still holds an
    /// `iced::Theme` value.
    pub fn from_theme(theme: &iced::Theme) -> Self {
        THEME_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let name = theme.to_string();
            if let Some((ref cached_name, cached_colors)) = *cache {
                if *cached_name == name {
                    return cached_colors;
                }
            }
            let index = gitkraft_core::theme_index_by_name(&name);
            let colors = Self::from_core(&gitkraft_core::theme_by_index(index));
            *cache = Some((name, colors));
            colors
        })
    }
}

// ── Container styles ──────────────────────────────────────────────────────────

/// Style for a container with the main window background.
pub fn bg_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.bg)),
        ..Default::default()
    }
}

/// Style for the error banner background (faint red tint).
pub fn error_banner_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.error_bg)),
        ..Default::default()
    }
}

/// Style for a container with the standard surface background.
pub fn surface_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.surface)),
        ..Default::default()
    }
}

/// Style for a container with the sidebar background.
pub fn sidebar_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.sidebar_bg)),
        ..Default::default()
    }
}

/// Style for a container with the header / toolbar background.
pub fn header_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.header_bg)),
        ..Default::default()
    }
}

/// Style for the floating context-menu panel.
pub fn context_menu_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.surface_highlight)),
        border: iced::Border {
            color: c.border,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow {
            color: iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// Style for the semi-transparent backdrop behind an open context menu.
pub fn backdrop_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(iced::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.15,
        })),
        ..Default::default()
    }
}

/// Style for a selected / highlighted row.
pub fn selected_row_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.surface_highlight)),
        ..Default::default()
    }
}

/// Style for a diff addition line.
pub fn diff_add_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.diff_add_bg)),
        ..Default::default()
    }
}

/// Style for a diff deletion line.
pub fn diff_del_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.diff_del_bg)),
        ..Default::default()
    }
}

/// Style for a diff hunk header line.
pub fn diff_hunk_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(Background::Color(c.diff_hunk_bg)),
        ..Default::default()
    }
}

// ── Button styles ─────────────────────────────────────────────────────────────

/// Completely transparent button — no background, no border.  Used for
/// clickable rows in the commit log, branch list, staging area, etc.
pub fn ghost_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let c = ThemeColors::from_theme(theme);
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: c.text_primary,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(c.surface_highlight)),
            text_color: c.text_primary,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(c.border)),
            text_color: c.text_primary,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: c.muted,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
    }
}

/// Active tab button — has a visible bottom accent border to indicate selection.
pub fn active_tab_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let c = ThemeColors::from_theme(theme);
    let active_border = iced::Border {
        color: c.accent,
        width: 0.0,
        radius: 0.0.into(),
    };
    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(c.surface)),
            text_color: c.text_primary,
            border: active_border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(c.surface_highlight)),
            text_color: c.text_primary,
            border: active_border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(c.border)),
            text_color: c.text_primary,
            border: active_border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(c.surface)),
            text_color: c.muted,
            border: active_border,
            shadow: iced::Shadow::default(),
        },
    }
}

/// Context menu item button — transparent at rest, accent-tinted on hover.
pub fn context_menu_item(theme: &iced::Theme, status: button::Status) -> button::Style {
    let c = ThemeColors::from_theme(theme);
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: c.text_primary,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(iced::Color {
                r: c.accent.r,
                g: c.accent.g,
                b: c.accent.b,
                a: 0.15,
            })),
            text_color: c.text_primary,
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            shadow: iced::Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(iced::Color {
                r: c.accent.r,
                g: c.accent.g,
                b: c.accent.b,
                a: 0.28,
            })),
            text_color: c.text_primary,
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            shadow: iced::Shadow::default(),
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: c.muted,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
    }
}

/// Overlay scrollbar — invisible at rest, thin rounded thumb on hover/drag.
///
/// - **Active** / not hovered: completely invisible — the scrollbar takes no
///   visual space and the content fills the full width.
/// - **Hovered** (cursor anywhere over the scrollable, not just the rail):
///   a thin, semi-transparent rounded thumb floats over the right edge.
/// - **Dragged**: same thumb, slightly more opaque for feedback.
///
/// Apply with a 6 px `Direction::Vertical` width so there is a small grab
/// target even though the rendered thumb is only 4 px wide.
pub fn overlay_scrollbar(theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let c = ThemeColors::from_theme(theme);

    let hidden = scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            color: Color::TRANSPARENT,
            border: iced::Border::default(),
        },
    };

    let thumb = |alpha: f32| scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            color: Color {
                r: c.muted.r,
                g: c.muted.g,
                b: c.muted.b,
                a: alpha,
            },
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
        },
    };

    let v_rail = match status {
        scrollable::Status::Active => hidden,
        scrollable::Status::Hovered { .. } => thumb(0.45),
        scrollable::Status::Dragged { .. } => thumb(0.70),
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: v_rail,
        horizontal_rail: hidden,
        gap: None,
    }
}

/// Subtle toolbar button — transparent at rest, light surface on hover.
pub fn toolbar_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let c = ThemeColors::from_theme(theme);
    let border = iced::Border {
        color: c.border,
        width: 1.0,
        radius: 4.0.into(),
    };
    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(c.surface)),
            text_color: c.text_primary,
            border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(c.surface_highlight)),
            text_color: c.text_primary,
            border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(c.border)),
            text_color: c.text_primary,
            border,
            shadow: iced::Shadow::default(),
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(c.surface)),
            text_color: c.muted,
            border,
            shadow: iced::Shadow::default(),
        },
    }
}

/// Small icon-only action button (stage, unstage, delete, etc.)
pub fn icon_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let c = ThemeColors::from_theme(theme);
    match status {
        button::Status::Active => button::Style {
            background: None,
            text_color: c.text_secondary,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(c.surface_highlight)),
            text_color: c.text_primary,
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(c.border)),
            text_color: c.text_primary,
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow::default(),
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: c.muted,
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
        },
    }
}

// ── Semantic colour helpers ───────────────────────────────────────────────────

/// Return the colour corresponding to a file-status badge.
pub fn status_color(status: &gitkraft_core::FileStatus, c: &ThemeColors) -> Color {
    match status {
        gitkraft_core::FileStatus::New | gitkraft_core::FileStatus::Untracked => c.green,
        gitkraft_core::FileStatus::Modified | gitkraft_core::FileStatus::Typechange => c.yellow,
        gitkraft_core::FileStatus::Deleted => c.red,
        gitkraft_core::FileStatus::Renamed | gitkraft_core::FileStatus::Copied => c.accent,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_core_dark_theme() {
        let core = gitkraft_core::theme_by_index(0); // Default (dark)
        let colors = ThemeColors::from_core(&core);
        // Dark theme should have a dark background
        assert!(colors.bg.r < 0.5);
        // Accent, green, red should all be non-zero
        assert!(colors.accent.r > 0.0 || colors.accent.g > 0.0 || colors.accent.b > 0.0);
        assert!(colors.green.g > 0.0);
        assert!(colors.red.r > 0.0);
    }

    #[test]
    fn from_core_light_theme() {
        let core = gitkraft_core::theme_by_index(11); // Solarized Light
        let colors = ThemeColors::from_core(&core);
        // Light theme should have a light background
        assert!(colors.bg.r > 0.5);
    }

    #[test]
    fn from_theme_fallback_still_works() {
        let colors = ThemeColors::from_theme(&iced::Theme::Dark);
        // Should resolve to the Default core theme (dark bg)
        assert!(colors.bg.r < 0.5);
    }

    #[test]
    fn status_color_variants() {
        let core = gitkraft_core::theme_by_index(0);
        let c = ThemeColors::from_core(&core);
        // New / Untracked → green
        assert_eq!(status_color(&gitkraft_core::FileStatus::New, &c), c.green);
        assert_eq!(
            status_color(&gitkraft_core::FileStatus::Untracked, &c),
            c.green
        );
        // Modified → yellow
        assert_eq!(
            status_color(&gitkraft_core::FileStatus::Modified, &c),
            c.yellow
        );
        // Deleted → red
        assert_eq!(status_color(&gitkraft_core::FileStatus::Deleted, &c), c.red);
        // Renamed → accent
        assert_eq!(
            status_color(&gitkraft_core::FileStatus::Renamed, &c),
            c.accent
        );
    }

    #[test]
    fn clamp_stays_in_range() {
        assert_eq!(clamp(-0.1), 0.0);
        assert_eq!(clamp(1.5), 1.0);
        assert!((clamp(0.5) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn shift_and_scale_stay_in_range() {
        let base = Color {
            r: 0.9,
            g: 0.1,
            b: 0.5,
            a: 1.0,
        };
        let shifted = shift(base, 0.2);
        assert!(shifted.r <= 1.0 && shifted.g >= 0.0);

        let scaled = scale(base, 2.0);
        assert!(scaled.r <= 1.0);
    }

    #[test]
    fn all_27_core_themes_produce_valid_colors() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            // bg channels should be in [0, 1]
            assert!(
                c.bg.r >= 0.0 && c.bg.r <= 1.0,
                "theme {i} bg.r out of range"
            );
            assert!(
                c.bg.g >= 0.0 && c.bg.g <= 1.0,
                "theme {i} bg.g out of range"
            );
            assert!(
                c.bg.b >= 0.0 && c.bg.b <= 1.0,
                "theme {i} bg.b out of range"
            );
        }
    }

    #[test]
    fn graph_colors_populated_for_all_themes() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            // All 8 graph lane colours should be valid (channels in [0, 1])
            for (lane, color) in c.graph_colors.iter().enumerate() {
                assert!(
                    color.r >= 0.0 && color.r <= 1.0,
                    "theme {i} graph_colors[{lane}].r out of range"
                );
                assert!(
                    color.g >= 0.0 && color.g <= 1.0,
                    "theme {i} graph_colors[{lane}].g out of range"
                );
                assert!(
                    color.b >= 0.0 && color.b <= 1.0,
                    "theme {i} graph_colors[{lane}].b out of range"
                );
            }
        }
    }

    #[test]
    fn graph_colors_are_not_all_identical() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            // At least two distinct colours among the 8 lanes
            let first = c.graph_colors[0];
            let all_same = c.graph_colors.iter().all(|gc| {
                (gc.r - first.r).abs() < f32::EPSILON
                    && (gc.g - first.g).abs() < f32::EPSILON
                    && (gc.b - first.b).abs() < f32::EPSILON
            });
            assert!(!all_same, "theme {i} has all identical graph lane colours");
        }
    }

    #[test]
    fn error_bg_differs_from_plain_bg() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            // error_bg should be a tinted version of bg, not identical
            let same = (c.error_bg.r - c.bg.r).abs() < f32::EPSILON
                && (c.error_bg.g - c.bg.g).abs() < f32::EPSILON
                && (c.error_bg.b - c.bg.b).abs() < f32::EPSILON;
            assert!(
                !same,
                "theme {i} error_bg is identical to bg — tint not applied"
            );
        }
    }

    #[test]
    fn selection_is_valid_color() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            assert!(
                c.selection.r >= 0.0 && c.selection.r <= 1.0,
                "theme {i} selection.r out of range"
            );
            assert!(
                c.selection.g >= 0.0 && c.selection.g <= 1.0,
                "theme {i} selection.g out of range"
            );
            assert!(
                c.selection.b >= 0.0 && c.selection.b <= 1.0,
                "theme {i} selection.b out of range"
            );
        }
    }

    #[test]
    fn selection_differs_from_bg() {
        for i in 0..gitkraft_core::THEME_COUNT {
            let core = gitkraft_core::theme_by_index(i);
            let c = ThemeColors::from_core(&core);
            let same = (c.selection.r - c.bg.r).abs() < f32::EPSILON
                && (c.selection.g - c.bg.g).abs() < f32::EPSILON
                && (c.selection.b - c.bg.b).abs() < f32::EPSILON;
            assert!(
                !same,
                "theme {i} selection is identical to bg — should be distinguishable"
            );
        }
    }
}
