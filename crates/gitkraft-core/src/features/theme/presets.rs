//! All 27 preset themes — the **single source of truth** for every colour
//! used by both the GUI and TUI front-ends.
//!
//! Each public function returns a fully-populated [`AppTheme`] with concrete
//! RGB values. The [`theme_by_index`] dispatcher maps a `0..=26` index to the
//! matching constructor; out-of-range indices silently fall back to `default()`.

use super::types::{AppTheme, Rgb};

// ── Theme catalogue ───────────────────────────────────────────────────────────

/// Ordered theme names. The position in this slice **is** the canonical index.
pub const THEME_NAMES: &[&str] = &[
    "Default",
    "Grape",
    "Ocean",
    "Sunset",
    "Forest",
    "Rose",
    "Mono",
    "Neon",
    "Dracula",
    "Nord",
    "Solarized Dark",
    "Solarized Light",
    "Gruvbox Dark",
    "Gruvbox Light",
    "Catppuccin Latte",
    "Catppuccin Frappé",
    "Catppuccin Macchiato",
    "Catppuccin Mocha",
    "Tokyo Night",
    "Tokyo Night Storm",
    "Tokyo Night Light",
    "Kanagawa Wave",
    "Kanagawa Dragon",
    "Kanagawa Lotus",
    "Moonfly",
    "Nightfly",
    "Oxocarbon",
    "Cyberpunk",
];

/// Total number of themes.
pub const THEME_COUNT: usize = 28;

/// Get a theme by index (0-based). Returns `default()` for out-of-range.
pub fn theme_by_index(index: usize) -> AppTheme {
    match index {
        0 => default(),
        1 => grape(),
        2 => ocean(),
        3 => sunset(),
        4 => forest(),
        5 => rose(),
        6 => mono(),
        7 => neon(),
        8 => dracula(),
        9 => nord(),
        10 => solarized_dark(),
        11 => solarized_light(),
        12 => gruvbox_dark(),
        13 => gruvbox_light(),
        14 => catppuccin_latte(),
        15 => catppuccin_frappe(),
        16 => catppuccin_macchiato(),
        17 => catppuccin_mocha(),
        18 => tokyo_night(),
        19 => tokyo_night_storm(),
        20 => tokyo_night_light(),
        21 => kanagawa_wave(),
        22 => kanagawa_dragon(),
        23 => kanagawa_lotus(),
        24 => moonfly(),
        25 => nightfly(),
        26 => oxocarbon(),
        27 => cyberpunk(),
        _ => default(),
    }
}

/// Find theme index by name (case-insensitive ASCII). Returns `0` (Default) if
/// no match is found.
pub fn theme_index_by_name(name: &str) -> usize {
    THEME_NAMES
        .iter()
        .position(|n| n.eq_ignore_ascii_case(name))
        .unwrap_or(0)
}

// ── Theme definitions ─────────────────────────────────────────────────────────

pub fn default() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(13, 17, 23),
        surface: Rgb::new(22, 27, 34),
        border: Rgb::new(48, 54, 61),
        selection: Rgb::new(40, 60, 80),
        text_primary: Rgb::new(230, 237, 243),
        text_secondary: Rgb::new(139, 148, 158),
        text_muted: Rgb::new(72, 79, 88),
        accent: Rgb::new(88, 166, 255),
        success: Rgb::new(63, 185, 80),
        warning: Rgb::new(210, 154, 34),
        error: Rgb::new(248, 83, 73),
        diff_add: Rgb::new(63, 185, 80),
        diff_del: Rgb::new(248, 83, 73),
        diff_context: Rgb::new(139, 148, 158),
        diff_hunk: Rgb::new(88, 166, 255),
        graph_colors: [
            Rgb::new(88, 166, 255),
            Rgb::new(63, 185, 80),
            Rgb::new(248, 83, 73),
            Rgb::new(210, 154, 34),
            Rgb::new(188, 140, 255),
            Rgb::new(0, 200, 180),
            Rgb::new(255, 140, 60),
            Rgb::new(255, 130, 160),
        ],
    }
}

pub fn grape() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(24, 18, 36),
        surface: Rgb::new(35, 28, 50),
        border: Rgb::new(60, 50, 80),
        selection: Rgb::new(50, 40, 70),
        text_primary: Rgb::new(220, 210, 240),
        text_secondary: Rgb::new(160, 140, 200),
        text_muted: Rgb::new(100, 85, 130),
        accent: Rgb::new(180, 130, 255),
        success: Rgb::new(130, 230, 160),
        warning: Rgb::new(240, 200, 100),
        error: Rgb::new(255, 100, 120),
        diff_add: Rgb::new(130, 230, 160),
        diff_del: Rgb::new(255, 100, 120),
        diff_context: Rgb::new(100, 85, 130),
        diff_hunk: Rgb::new(150, 160, 255),
        graph_colors: [
            Rgb::new(180, 130, 255),
            Rgb::new(130, 230, 160),
            Rgb::new(255, 100, 120),
            Rgb::new(240, 200, 100),
            Rgb::new(100, 180, 255),
            Rgb::new(255, 160, 200),
            Rgb::new(120, 220, 220),
            Rgb::new(255, 170, 100),
        ],
    }
}

pub fn ocean() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(15, 25, 35),
        surface: Rgb::new(22, 35, 48),
        border: Rgb::new(40, 60, 80),
        selection: Rgb::new(30, 55, 75),
        text_primary: Rgb::new(200, 220, 240),
        text_secondary: Rgb::new(100, 170, 200),
        text_muted: Rgb::new(70, 100, 130),
        accent: Rgb::new(80, 200, 180),
        success: Rgb::new(80, 220, 120),
        warning: Rgb::new(220, 200, 80),
        error: Rgb::new(255, 100, 100),
        diff_add: Rgb::new(80, 220, 120),
        diff_del: Rgb::new(255, 100, 100),
        diff_context: Rgb::new(70, 100, 130),
        diff_hunk: Rgb::new(80, 200, 180),
        graph_colors: [
            Rgb::new(80, 200, 180),
            Rgb::new(80, 220, 120),
            Rgb::new(255, 100, 100),
            Rgb::new(220, 200, 80),
            Rgb::new(100, 160, 255),
            Rgb::new(200, 130, 255),
            Rgb::new(255, 180, 80),
            Rgb::new(255, 130, 180),
        ],
    }
}

pub fn sunset() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(35, 18, 18),
        surface: Rgb::new(50, 28, 28),
        border: Rgb::new(80, 45, 45),
        selection: Rgb::new(70, 35, 35),
        text_primary: Rgb::new(240, 220, 210),
        text_secondary: Rgb::new(200, 150, 120),
        text_muted: Rgb::new(140, 100, 80),
        accent: Rgb::new(255, 140, 60),
        success: Rgb::new(180, 220, 80),
        warning: Rgb::new(255, 200, 50),
        error: Rgb::new(255, 80, 80),
        diff_add: Rgb::new(180, 220, 80),
        diff_del: Rgb::new(255, 80, 80),
        diff_context: Rgb::new(140, 100, 80),
        diff_hunk: Rgb::new(255, 180, 100),
        graph_colors: [
            Rgb::new(255, 140, 60),
            Rgb::new(180, 220, 80),
            Rgb::new(255, 80, 80),
            Rgb::new(255, 200, 50),
            Rgb::new(200, 120, 255),
            Rgb::new(80, 200, 200),
            Rgb::new(255, 130, 160),
            Rgb::new(100, 180, 255),
        ],
    }
}

pub fn forest() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(18, 28, 18),
        surface: Rgb::new(28, 40, 28),
        border: Rgb::new(50, 70, 50),
        selection: Rgb::new(35, 55, 35),
        text_primary: Rgb::new(210, 230, 210),
        text_secondary: Rgb::new(140, 180, 140),
        text_muted: Rgb::new(80, 110, 80),
        accent: Rgb::new(80, 180, 80),
        success: Rgb::new(100, 220, 100),
        warning: Rgb::new(220, 200, 80),
        error: Rgb::new(220, 90, 70),
        diff_add: Rgb::new(100, 220, 100),
        diff_del: Rgb::new(220, 90, 70),
        diff_context: Rgb::new(80, 110, 80),
        diff_hunk: Rgb::new(120, 180, 120),
        graph_colors: [
            Rgb::new(80, 180, 80),
            Rgb::new(100, 220, 100),
            Rgb::new(220, 90, 70),
            Rgb::new(220, 200, 80),
            Rgb::new(100, 170, 220),
            Rgb::new(180, 130, 200),
            Rgb::new(200, 180, 100),
            Rgb::new(140, 210, 200),
        ],
    }
}

pub fn rose() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(30, 18, 25),
        surface: Rgb::new(45, 28, 38),
        border: Rgb::new(75, 45, 60),
        selection: Rgb::new(60, 35, 50),
        text_primary: Rgb::new(240, 220, 230),
        text_secondary: Rgb::new(200, 140, 170),
        text_muted: Rgb::new(130, 90, 110),
        accent: Rgb::new(255, 130, 160),
        success: Rgb::new(130, 220, 150),
        warning: Rgb::new(240, 200, 100),
        error: Rgb::new(255, 80, 100),
        diff_add: Rgb::new(130, 220, 150),
        diff_del: Rgb::new(255, 80, 100),
        diff_context: Rgb::new(130, 90, 110),
        diff_hunk: Rgb::new(220, 150, 200),
        graph_colors: [
            Rgb::new(255, 130, 160),
            Rgb::new(130, 220, 150),
            Rgb::new(255, 80, 100),
            Rgb::new(240, 200, 100),
            Rgb::new(130, 170, 255),
            Rgb::new(200, 140, 255),
            Rgb::new(80, 210, 210),
            Rgb::new(255, 170, 100),
        ],
    }
}

pub fn mono() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(20, 20, 20),
        surface: Rgb::new(35, 35, 35),
        border: Rgb::new(60, 60, 60),
        selection: Rgb::new(50, 50, 50),
        text_primary: Rgb::new(220, 220, 220),
        text_secondary: Rgb::new(160, 160, 160),
        text_muted: Rgb::new(100, 100, 100),
        accent: Rgb::new(180, 180, 180),
        success: Rgb::new(180, 220, 180),
        warning: Rgb::new(220, 200, 160),
        error: Rgb::new(220, 140, 140),
        diff_add: Rgb::new(180, 220, 180),
        diff_del: Rgb::new(220, 140, 140),
        diff_context: Rgb::new(100, 100, 100),
        diff_hunk: Rgb::new(180, 180, 180),
        graph_colors: [
            Rgb::new(180, 180, 180),
            Rgb::new(180, 220, 180),
            Rgb::new(220, 140, 140),
            Rgb::new(220, 200, 160),
            Rgb::new(140, 180, 220),
            Rgb::new(200, 160, 200),
            Rgb::new(200, 200, 140),
            Rgb::new(160, 210, 210),
        ],
    }
}

pub fn neon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(10, 10, 18),
        surface: Rgb::new(18, 18, 30),
        border: Rgb::new(30, 30, 60),
        selection: Rgb::new(25, 25, 50),
        text_primary: Rgb::new(220, 240, 255),
        text_secondary: Rgb::new(0, 200, 255),
        text_muted: Rgb::new(60, 80, 120),
        accent: Rgb::new(0, 255, 200),
        success: Rgb::new(0, 255, 100),
        warning: Rgb::new(255, 255, 0),
        error: Rgb::new(255, 0, 80),
        diff_add: Rgb::new(0, 255, 100),
        diff_del: Rgb::new(255, 0, 80),
        diff_context: Rgb::new(60, 80, 120),
        diff_hunk: Rgb::new(200, 0, 255),
        graph_colors: [
            Rgb::new(0, 255, 200),
            Rgb::new(0, 255, 100),
            Rgb::new(255, 0, 80),
            Rgb::new(255, 255, 0),
            Rgb::new(0, 200, 255),
            Rgb::new(200, 0, 255),
            Rgb::new(255, 100, 0),
            Rgb::new(255, 0, 200),
        ],
    }
}

pub fn dracula() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(40, 42, 54),
        surface: Rgb::new(68, 71, 90),
        border: Rgb::new(98, 114, 164),
        selection: Rgb::new(68, 71, 90),
        text_primary: Rgb::new(248, 248, 242),
        text_secondary: Rgb::new(189, 147, 249),
        text_muted: Rgb::new(98, 114, 164),
        accent: Rgb::new(189, 147, 249),
        success: Rgb::new(80, 250, 123),
        warning: Rgb::new(241, 250, 140),
        error: Rgb::new(255, 85, 85),
        diff_add: Rgb::new(80, 250, 123),
        diff_del: Rgb::new(255, 85, 85),
        diff_context: Rgb::new(98, 114, 164),
        diff_hunk: Rgb::new(139, 233, 253),
        graph_colors: [
            Rgb::new(189, 147, 249),
            Rgb::new(80, 250, 123),
            Rgb::new(255, 85, 85),
            Rgb::new(241, 250, 140),
            Rgb::new(139, 233, 253),
            Rgb::new(255, 121, 198),
            Rgb::new(255, 184, 108),
            Rgb::new(98, 114, 164),
        ],
    }
}

pub fn nord() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(46, 52, 64),
        surface: Rgb::new(59, 66, 82),
        border: Rgb::new(76, 86, 106),
        selection: Rgb::new(59, 66, 82),
        text_primary: Rgb::new(216, 222, 233),
        text_secondary: Rgb::new(129, 161, 193),
        text_muted: Rgb::new(76, 86, 106),
        accent: Rgb::new(136, 192, 208),
        success: Rgb::new(163, 190, 140),
        warning: Rgb::new(235, 203, 139),
        error: Rgb::new(191, 97, 106),
        diff_add: Rgb::new(163, 190, 140),
        diff_del: Rgb::new(191, 97, 106),
        diff_context: Rgb::new(76, 86, 106),
        diff_hunk: Rgb::new(129, 161, 193),
        graph_colors: [
            Rgb::new(136, 192, 208),
            Rgb::new(163, 190, 140),
            Rgb::new(191, 97, 106),
            Rgb::new(235, 203, 139),
            Rgb::new(129, 161, 193),
            Rgb::new(180, 142, 173),
            Rgb::new(208, 135, 112),
            Rgb::new(143, 188, 187),
        ],
    }
}

pub fn solarized_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(0, 43, 54),
        surface: Rgb::new(7, 54, 66),
        border: Rgb::new(88, 110, 117),
        selection: Rgb::new(7, 54, 66),
        text_primary: Rgb::new(131, 148, 150),
        text_secondary: Rgb::new(42, 161, 152),
        text_muted: Rgb::new(88, 110, 117),
        accent: Rgb::new(38, 139, 210),
        success: Rgb::new(133, 153, 0),
        warning: Rgb::new(181, 137, 0),
        error: Rgb::new(220, 50, 47),
        diff_add: Rgb::new(133, 153, 0),
        diff_del: Rgb::new(220, 50, 47),
        diff_context: Rgb::new(88, 110, 117),
        diff_hunk: Rgb::new(42, 161, 152),
        graph_colors: [
            Rgb::new(38, 139, 210),
            Rgb::new(133, 153, 0),
            Rgb::new(220, 50, 47),
            Rgb::new(181, 137, 0),
            Rgb::new(42, 161, 152),
            Rgb::new(108, 113, 196),
            Rgb::new(203, 75, 22),
            Rgb::new(211, 54, 130),
        ],
    }
}

pub fn solarized_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: Rgb::new(253, 246, 227),
        surface: Rgb::new(238, 232, 213),
        border: Rgb::new(147, 161, 161),
        selection: Rgb::new(238, 232, 213),
        text_primary: Rgb::new(101, 123, 131),
        text_secondary: Rgb::new(42, 161, 152),
        text_muted: Rgb::new(147, 161, 161),
        accent: Rgb::new(38, 139, 210),
        success: Rgb::new(133, 153, 0),
        warning: Rgb::new(181, 137, 0),
        error: Rgb::new(220, 50, 47),
        diff_add: Rgb::new(133, 153, 0),
        diff_del: Rgb::new(220, 50, 47),
        diff_context: Rgb::new(147, 161, 161),
        diff_hunk: Rgb::new(42, 161, 152),
        graph_colors: [
            Rgb::new(38, 139, 210),
            Rgb::new(133, 153, 0),
            Rgb::new(220, 50, 47),
            Rgb::new(181, 137, 0),
            Rgb::new(42, 161, 152),
            Rgb::new(108, 113, 196),
            Rgb::new(203, 75, 22),
            Rgb::new(211, 54, 130),
        ],
    }
}

pub fn gruvbox_dark() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(40, 40, 40),
        surface: Rgb::new(60, 56, 54),
        border: Rgb::new(80, 73, 69),
        selection: Rgb::new(60, 56, 54),
        text_primary: Rgb::new(235, 219, 178),
        text_secondary: Rgb::new(250, 189, 47),
        text_muted: Rgb::new(146, 131, 116),
        accent: Rgb::new(254, 128, 25),
        success: Rgb::new(184, 187, 38),
        warning: Rgb::new(250, 189, 47),
        error: Rgb::new(251, 73, 52),
        diff_add: Rgb::new(184, 187, 38),
        diff_del: Rgb::new(251, 73, 52),
        diff_context: Rgb::new(146, 131, 116),
        diff_hunk: Rgb::new(142, 192, 124),
        graph_colors: [
            Rgb::new(254, 128, 25),
            Rgb::new(184, 187, 38),
            Rgb::new(251, 73, 52),
            Rgb::new(250, 189, 47),
            Rgb::new(131, 165, 152),
            Rgb::new(211, 134, 155),
            Rgb::new(142, 192, 124),
            Rgb::new(69, 133, 136),
        ],
    }
}

pub fn gruvbox_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: Rgb::new(251, 241, 199),
        surface: Rgb::new(213, 196, 161),
        border: Rgb::new(168, 153, 132),
        selection: Rgb::new(213, 196, 161),
        text_primary: Rgb::new(60, 56, 54),
        text_secondary: Rgb::new(215, 153, 33),
        text_muted: Rgb::new(146, 131, 116),
        accent: Rgb::new(214, 93, 14),
        success: Rgb::new(121, 116, 14),
        warning: Rgb::new(215, 153, 33),
        error: Rgb::new(157, 0, 6),
        diff_add: Rgb::new(121, 116, 14),
        diff_del: Rgb::new(157, 0, 6),
        diff_context: Rgb::new(146, 131, 116),
        diff_hunk: Rgb::new(104, 157, 106),
        graph_colors: [
            Rgb::new(214, 93, 14),
            Rgb::new(121, 116, 14),
            Rgb::new(157, 0, 6),
            Rgb::new(215, 153, 33),
            Rgb::new(69, 133, 136),
            Rgb::new(177, 98, 134),
            Rgb::new(104, 157, 106),
            Rgb::new(7, 102, 120),
        ],
    }
}

pub fn catppuccin_latte() -> AppTheme {
    AppTheme {
        is_dark: false,
        // Base
        background: Rgb::new(239, 241, 245),
        // Surface 0
        surface: Rgb::new(204, 208, 218),
        // Surface 2
        border: Rgb::new(172, 176, 190),
        // Surface 1
        selection: Rgb::new(188, 192, 204),
        // Text
        text_primary: Rgb::new(76, 79, 105),
        // Subtext 1
        text_secondary: Rgb::new(92, 95, 119),
        // Overlay 0
        text_muted: Rgb::new(156, 160, 176),
        // Mauve
        accent: Rgb::new(136, 57, 239),
        // Green
        success: Rgb::new(64, 160, 43),
        // Yellow
        warning: Rgb::new(223, 142, 29),
        // Red
        error: Rgb::new(210, 15, 57),
        diff_add: Rgb::new(64, 160, 43),
        diff_del: Rgb::new(210, 15, 57),
        // Overlay 0
        diff_context: Rgb::new(156, 160, 176),
        // Teal
        diff_hunk: Rgb::new(23, 146, 153),
        graph_colors: [
            Rgb::new(136, 57, 239), // Mauve
            Rgb::new(64, 160, 43),  // Green
            Rgb::new(210, 15, 57),  // Red
            Rgb::new(223, 142, 29), // Yellow
            Rgb::new(23, 146, 153), // Teal
            Rgb::new(30, 102, 245), // Blue
            Rgb::new(230, 69, 83),  // Maroon
            Rgb::new(32, 159, 181), // Sapphire
        ],
    }
}

pub fn catppuccin_frappe() -> AppTheme {
    AppTheme {
        is_dark: true,
        // Base
        background: Rgb::new(48, 52, 70),
        // Surface 0
        surface: Rgb::new(65, 69, 89),
        // Surface 2
        border: Rgb::new(98, 104, 128),
        // Surface 1
        selection: Rgb::new(81, 87, 109),
        // Text
        text_primary: Rgb::new(198, 208, 245),
        // Subtext 1
        text_secondary: Rgb::new(181, 191, 226),
        // Overlay 0
        text_muted: Rgb::new(115, 121, 148),
        // Mauve
        accent: Rgb::new(202, 158, 230),
        // Green
        success: Rgb::new(166, 209, 137),
        // Yellow
        warning: Rgb::new(229, 200, 144),
        // Red
        error: Rgb::new(231, 130, 132),
        diff_add: Rgb::new(166, 209, 137),
        diff_del: Rgb::new(231, 130, 132),
        // Overlay 0
        diff_context: Rgb::new(115, 121, 148),
        // Teal
        diff_hunk: Rgb::new(129, 200, 190),
        graph_colors: [
            Rgb::new(202, 158, 230), // Mauve
            Rgb::new(166, 209, 137), // Green
            Rgb::new(231, 130, 132), // Red
            Rgb::new(229, 200, 144), // Yellow
            Rgb::new(140, 170, 238), // Blue
            Rgb::new(244, 184, 228), // Pink
            Rgb::new(129, 200, 190), // Teal
            Rgb::new(239, 159, 118), // Peach
        ],
    }
}

pub fn catppuccin_macchiato() -> AppTheme {
    AppTheme {
        is_dark: true,
        // Base
        background: Rgb::new(36, 39, 58),
        // Surface 0
        surface: Rgb::new(54, 58, 79),
        // Surface 2
        border: Rgb::new(91, 96, 120),
        // Surface 1
        selection: Rgb::new(73, 77, 100),
        // Text
        text_primary: Rgb::new(202, 211, 245),
        // Subtext 1
        text_secondary: Rgb::new(184, 192, 224),
        // Overlay 0
        text_muted: Rgb::new(110, 115, 141),
        // Mauve
        accent: Rgb::new(198, 160, 246),
        // Green
        success: Rgb::new(166, 218, 149),
        // Yellow
        warning: Rgb::new(238, 212, 159),
        // Red
        error: Rgb::new(237, 135, 150),
        diff_add: Rgb::new(166, 218, 149),
        diff_del: Rgb::new(237, 135, 150),
        // Overlay 0
        diff_context: Rgb::new(110, 115, 141),
        // Teal
        diff_hunk: Rgb::new(139, 213, 202),
        graph_colors: [
            Rgb::new(198, 160, 246), // Mauve
            Rgb::new(166, 218, 149), // Green
            Rgb::new(237, 135, 150), // Red
            Rgb::new(238, 212, 159), // Yellow
            Rgb::new(138, 173, 244), // Blue
            Rgb::new(245, 189, 230), // Pink
            Rgb::new(139, 213, 202), // Teal
            Rgb::new(245, 169, 127), // Peach
        ],
    }
}

pub fn catppuccin_mocha() -> AppTheme {
    AppTheme {
        is_dark: true,
        // Base
        background: Rgb::new(30, 30, 46),
        // Surface 0
        surface: Rgb::new(49, 50, 68),
        // Surface 2
        border: Rgb::new(88, 91, 112),
        // Surface 1
        selection: Rgb::new(69, 71, 90),
        // Text
        text_primary: Rgb::new(205, 214, 244),
        // Subtext 1
        text_secondary: Rgb::new(186, 194, 222),
        // Overlay 0
        text_muted: Rgb::new(108, 112, 134),
        // Mauve
        accent: Rgb::new(203, 166, 247),
        // Green
        success: Rgb::new(166, 227, 161),
        // Yellow
        warning: Rgb::new(249, 226, 175),
        // Red
        error: Rgb::new(243, 139, 168),
        diff_add: Rgb::new(166, 227, 161),
        diff_del: Rgb::new(243, 139, 168),
        // Overlay 0
        diff_context: Rgb::new(108, 112, 134),
        // Blue
        diff_hunk: Rgb::new(137, 180, 250),
        graph_colors: [
            Rgb::new(203, 166, 247), // Mauve
            Rgb::new(166, 227, 161), // Green
            Rgb::new(243, 139, 168), // Red
            Rgb::new(249, 226, 175), // Yellow
            Rgb::new(137, 180, 250), // Blue
            Rgb::new(245, 194, 231), // Pink
            Rgb::new(148, 226, 213), // Teal
            Rgb::new(250, 179, 135), // Peach
        ],
    }
}

pub fn tokyo_night() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(26, 27, 38),
        surface: Rgb::new(41, 46, 66),
        border: Rgb::new(59, 66, 97),
        selection: Rgb::new(41, 46, 66),
        text_primary: Rgb::new(192, 202, 245),
        text_secondary: Rgb::new(122, 162, 247),
        text_muted: Rgb::new(86, 95, 137),
        accent: Rgb::new(122, 162, 247),
        success: Rgb::new(158, 206, 106),
        warning: Rgb::new(224, 175, 104),
        error: Rgb::new(247, 118, 142),
        diff_add: Rgb::new(158, 206, 106),
        diff_del: Rgb::new(247, 118, 142),
        diff_context: Rgb::new(86, 95, 137),
        diff_hunk: Rgb::new(187, 154, 247),
        graph_colors: [
            Rgb::new(122, 162, 247),
            Rgb::new(158, 206, 106),
            Rgb::new(247, 118, 142),
            Rgb::new(224, 175, 104),
            Rgb::new(187, 154, 247),
            Rgb::new(255, 117, 127),
            Rgb::new(115, 218, 202),
            Rgb::new(255, 158, 100),
        ],
    }
}

pub fn tokyo_night_storm() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(36, 40, 59),
        surface: Rgb::new(45, 49, 75),
        border: Rgb::new(59, 66, 97),
        selection: Rgb::new(45, 49, 75),
        text_primary: Rgb::new(192, 202, 245),
        text_secondary: Rgb::new(122, 162, 247),
        text_muted: Rgb::new(86, 95, 137),
        accent: Rgb::new(122, 162, 247),
        success: Rgb::new(158, 206, 106),
        warning: Rgb::new(224, 175, 104),
        error: Rgb::new(247, 118, 142),
        diff_add: Rgb::new(158, 206, 106),
        diff_del: Rgb::new(247, 118, 142),
        diff_context: Rgb::new(86, 95, 137),
        diff_hunk: Rgb::new(187, 154, 247),
        graph_colors: [
            Rgb::new(122, 162, 247),
            Rgb::new(158, 206, 106),
            Rgb::new(247, 118, 142),
            Rgb::new(224, 175, 104),
            Rgb::new(187, 154, 247),
            Rgb::new(255, 117, 127),
            Rgb::new(115, 218, 202),
            Rgb::new(255, 158, 100),
        ],
    }
}

pub fn tokyo_night_light() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: Rgb::new(213, 214, 219),
        surface: Rgb::new(208, 213, 227),
        border: Rgb::new(132, 140, 176),
        selection: Rgb::new(208, 213, 227),
        text_primary: Rgb::new(52, 59, 88),
        text_secondary: Rgb::new(46, 126, 233),
        text_muted: Rgb::new(132, 140, 176),
        accent: Rgb::new(46, 126, 233),
        success: Rgb::new(72, 94, 48),
        warning: Rgb::new(140, 108, 62),
        error: Rgb::new(143, 57, 85),
        diff_add: Rgb::new(72, 94, 48),
        diff_del: Rgb::new(143, 57, 85),
        diff_context: Rgb::new(132, 140, 176),
        diff_hunk: Rgb::new(90, 74, 120),
        graph_colors: [
            Rgb::new(46, 126, 233),
            Rgb::new(72, 94, 48),
            Rgb::new(143, 57, 85),
            Rgb::new(140, 108, 62),
            Rgb::new(90, 74, 120),
            Rgb::new(166, 77, 121),
            Rgb::new(15, 130, 130),
            Rgb::new(180, 90, 50),
        ],
    }
}

pub fn kanagawa_wave() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(31, 31, 40),
        surface: Rgb::new(42, 42, 55),
        border: Rgb::new(84, 84, 88),
        selection: Rgb::new(42, 42, 55),
        text_primary: Rgb::new(220, 215, 186),
        text_secondary: Rgb::new(126, 156, 216),
        text_muted: Rgb::new(114, 113, 105),
        accent: Rgb::new(126, 156, 216),
        success: Rgb::new(118, 148, 106),
        warning: Rgb::new(220, 165, 97),
        error: Rgb::new(195, 64, 67),
        diff_add: Rgb::new(118, 148, 106),
        diff_del: Rgb::new(195, 64, 67),
        diff_context: Rgb::new(114, 113, 105),
        diff_hunk: Rgb::new(210, 126, 153),
        graph_colors: [
            Rgb::new(126, 156, 216),
            Rgb::new(118, 148, 106),
            Rgb::new(195, 64, 67),
            Rgb::new(220, 165, 97),
            Rgb::new(210, 126, 153),
            Rgb::new(160, 140, 200),
            Rgb::new(106, 149, 137),
            Rgb::new(228, 104, 118),
        ],
    }
}

pub fn kanagawa_dragon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(24, 21, 21),
        surface: Rgb::new(40, 39, 39),
        border: Rgb::new(80, 80, 78),
        selection: Rgb::new(40, 39, 39),
        text_primary: Rgb::new(197, 201, 197),
        text_secondary: Rgb::new(139, 164, 176),
        text_muted: Rgb::new(166, 166, 156),
        accent: Rgb::new(139, 164, 176),
        success: Rgb::new(135, 169, 135),
        warning: Rgb::new(200, 170, 109),
        error: Rgb::new(195, 64, 67),
        diff_add: Rgb::new(135, 169, 135),
        diff_del: Rgb::new(195, 64, 67),
        diff_context: Rgb::new(166, 166, 156),
        diff_hunk: Rgb::new(210, 126, 153),
        graph_colors: [
            Rgb::new(139, 164, 176),
            Rgb::new(135, 169, 135),
            Rgb::new(195, 64, 67),
            Rgb::new(200, 170, 109),
            Rgb::new(210, 126, 153),
            Rgb::new(160, 140, 200),
            Rgb::new(106, 149, 137),
            Rgb::new(228, 104, 118),
        ],
    }
}

pub fn kanagawa_lotus() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: Rgb::new(245, 240, 215),
        surface: Rgb::new(231, 219, 160),
        border: Rgb::new(196, 178, 138),
        selection: Rgb::new(231, 219, 160),
        text_primary: Rgb::new(84, 84, 100),
        text_secondary: Rgb::new(77, 105, 155),
        text_muted: Rgb::new(196, 178, 138),
        accent: Rgb::new(77, 105, 155),
        success: Rgb::new(111, 137, 78),
        warning: Rgb::new(119, 113, 63),
        error: Rgb::new(195, 64, 67),
        diff_add: Rgb::new(111, 137, 78),
        diff_del: Rgb::new(195, 64, 67),
        diff_context: Rgb::new(196, 178, 138),
        diff_hunk: Rgb::new(160, 154, 190),
        graph_colors: [
            Rgb::new(77, 105, 155),
            Rgb::new(111, 137, 78),
            Rgb::new(195, 64, 67),
            Rgb::new(119, 113, 63),
            Rgb::new(160, 154, 190),
            Rgb::new(155, 80, 117),
            Rgb::new(75, 130, 120),
            Rgb::new(180, 100, 55),
        ],
    }
}

pub fn moonfly() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(8, 8, 8),
        surface: Rgb::new(23, 23, 23),
        border: Rgb::new(50, 50, 50),
        selection: Rgb::new(23, 50, 80),
        text_primary: Rgb::new(189, 189, 189),
        text_secondary: Rgb::new(128, 160, 255),
        text_muted: Rgb::new(99, 99, 99),
        accent: Rgb::new(174, 129, 255),
        success: Rgb::new(130, 170, 60),
        warning: Rgb::new(230, 170, 50),
        error: Rgb::new(255, 83, 112),
        diff_add: Rgb::new(130, 170, 60),
        diff_del: Rgb::new(255, 83, 112),
        diff_context: Rgb::new(99, 99, 99),
        diff_hunk: Rgb::new(128, 160, 255),
        graph_colors: [
            Rgb::new(174, 129, 255),
            Rgb::new(130, 170, 60),
            Rgb::new(255, 83, 112),
            Rgb::new(230, 170, 50),
            Rgb::new(128, 160, 255),
            Rgb::new(255, 100, 180),
            Rgb::new(80, 200, 180),
            Rgb::new(255, 160, 80),
        ],
    }
}

pub fn nightfly() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(1, 13, 32),
        surface: Rgb::new(5, 22, 50),
        border: Rgb::new(30, 50, 80),
        selection: Rgb::new(10, 40, 80),
        text_primary: Rgb::new(195, 204, 219),
        text_secondary: Rgb::new(130, 170, 255),
        text_muted: Rgb::new(99, 117, 150),
        accent: Rgb::new(130, 170, 255),
        success: Rgb::new(161, 217, 147),
        warning: Rgb::new(236, 196, 100),
        error: Rgb::new(252, 57, 49),
        diff_add: Rgb::new(161, 217, 147),
        diff_del: Rgb::new(252, 57, 49),
        diff_context: Rgb::new(99, 117, 150),
        diff_hunk: Rgb::new(174, 129, 255),
        graph_colors: [
            Rgb::new(130, 170, 255),
            Rgb::new(161, 217, 147),
            Rgb::new(252, 57, 49),
            Rgb::new(236, 196, 100),
            Rgb::new(174, 129, 255),
            Rgb::new(255, 100, 180),
            Rgb::new(33, 200, 170),
            Rgb::new(255, 160, 90),
        ],
    }
}

pub fn oxocarbon() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(22, 22, 22),
        surface: Rgb::new(38, 38, 38),
        border: Rgb::new(57, 57, 57),
        selection: Rgb::new(38, 38, 38),
        text_primary: Rgb::new(240, 240, 240),
        text_secondary: Rgb::new(78, 154, 232),
        text_muted: Rgb::new(82, 82, 82),
        accent: Rgb::new(78, 154, 232),
        success: Rgb::new(66, 190, 101),
        warning: Rgb::new(190, 149, 255),
        error: Rgb::new(238, 83, 120),
        diff_add: Rgb::new(66, 190, 101),
        diff_del: Rgb::new(238, 83, 120),
        diff_context: Rgb::new(82, 82, 82),
        diff_hunk: Rgb::new(51, 177, 255),
        graph_colors: [
            Rgb::new(78, 154, 232),
            Rgb::new(66, 190, 101),
            Rgb::new(238, 83, 120),
            Rgb::new(190, 149, 255),
            Rgb::new(51, 177, 255),
            Rgb::new(255, 104, 159),
            Rgb::new(8, 189, 186),
            Rgb::new(255, 164, 90),
        ],
    }
}

/// Cyberpunk — inspired by Cyberpunk 2077's signature electric yellow
/// accent with cyan neon highlights against a deep dark background.
pub fn cyberpunk() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(10, 10, 16),
        surface: Rgb::new(20, 20, 30),
        border: Rgb::new(45, 45, 55),
        selection: Rgb::new(50, 48, 20),
        text_primary: Rgb::new(230, 230, 220),
        text_secondary: Rgb::new(0, 210, 235),
        text_muted: Rgb::new(90, 90, 100),
        accent: Rgb::new(252, 238, 10),
        success: Rgb::new(0, 220, 180),
        warning: Rgb::new(255, 150, 0),
        error: Rgb::new(255, 50, 70),
        diff_add: Rgb::new(0, 220, 180),
        diff_del: Rgb::new(255, 50, 70),
        diff_context: Rgb::new(90, 90, 100),
        diff_hunk: Rgb::new(0, 210, 235),
        graph_colors: [
            Rgb::new(252, 238, 10),
            Rgb::new(0, 210, 235),
            Rgb::new(0, 220, 180),
            Rgb::new(255, 150, 0),
            Rgb::new(180, 60, 255),
            Rgb::new(255, 80, 120),
            Rgb::new(100, 255, 220),
            Rgb::new(255, 200, 60),
        ],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_themes_resolve() {
        for i in 0..THEME_COUNT {
            let t = theme_by_index(i);
            // Every theme should have non-zero text
            assert!(
                t.text_primary.r > 0 || t.text_primary.g > 0 || t.text_primary.b > 0,
                "theme index {i} has zero text_primary"
            );
        }
    }

    #[test]
    fn theme_count_matches_names() {
        assert_eq!(THEME_NAMES.len(), THEME_COUNT);
    }

    #[test]
    fn index_by_name_round_trips() {
        for (i, name) in THEME_NAMES.iter().enumerate() {
            assert_eq!(
                theme_index_by_name(name),
                i,
                "round-trip failed for '{name}'"
            );
        }
    }

    #[test]
    fn unknown_name_returns_zero() {
        assert_eq!(theme_index_by_name("nonexistent"), 0);
    }

    #[test]
    fn out_of_range_returns_default() {
        let d = default();
        let oob = theme_by_index(999);
        assert_eq!(d.background, oob.background);
        assert_eq!(d.accent, oob.accent);
    }

    #[test]
    fn graph_colors_populated_for_all_themes() {
        for i in 0..THEME_COUNT {
            let t = theme_by_index(i);
            // Every theme must supply exactly 8 graph lane colours
            assert_eq!(
                t.graph_colors.len(),
                8,
                "theme index {i} does not have 8 graph_colors"
            );
            // At least two distinct colours among the 8 lanes
            let first = t.graph_colors[0];
            let all_same = t.graph_colors.iter().all(|c| *c == first);
            assert!(
                !all_same,
                "theme index {i} has all identical graph lane colours"
            );
        }
    }

    #[test]
    fn graph_colors_channels_nonzero() {
        for i in 0..THEME_COUNT {
            let t = theme_by_index(i);
            for (lane, c) in t.graph_colors.iter().enumerate() {
                // Each lane colour should have at least one non-zero channel
                // (pure black would be invisible on dark themes)
                assert!(
                    c.r > 0 || c.g > 0 || c.b > 0,
                    "theme {i} graph_colors[{lane}] is pure black"
                );
            }
        }
    }
}
