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
];

/// Total number of themes.
pub const THEME_COUNT: usize = 27;

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
    }
}

pub fn catppuccin_latte() -> AppTheme {
    AppTheme {
        is_dark: false,
        background: Rgb::new(239, 241, 245),
        surface: Rgb::new(204, 208, 218),
        border: Rgb::new(172, 176, 190),
        selection: Rgb::new(204, 208, 218),
        text_primary: Rgb::new(76, 79, 105),
        text_secondary: Rgb::new(30, 102, 245),
        text_muted: Rgb::new(156, 160, 176),
        accent: Rgb::new(136, 57, 239),
        success: Rgb::new(64, 160, 43),
        warning: Rgb::new(223, 142, 29),
        error: Rgb::new(210, 15, 57),
        diff_add: Rgb::new(64, 160, 43),
        diff_del: Rgb::new(210, 15, 57),
        diff_context: Rgb::new(156, 160, 176),
        diff_hunk: Rgb::new(23, 146, 153),
    }
}

pub fn catppuccin_frappe() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(48, 52, 70),
        surface: Rgb::new(65, 69, 89),
        border: Rgb::new(81, 87, 109),
        selection: Rgb::new(65, 69, 89),
        text_primary: Rgb::new(198, 208, 245),
        text_secondary: Rgb::new(140, 170, 238),
        text_muted: Rgb::new(115, 121, 148),
        accent: Rgb::new(202, 158, 230),
        success: Rgb::new(166, 209, 137),
        warning: Rgb::new(229, 200, 144),
        error: Rgb::new(231, 130, 132),
        diff_add: Rgb::new(166, 209, 137),
        diff_del: Rgb::new(231, 130, 132),
        diff_context: Rgb::new(115, 121, 148),
        diff_hunk: Rgb::new(129, 200, 190),
    }
}

pub fn catppuccin_macchiato() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(36, 39, 58),
        surface: Rgb::new(54, 58, 79),
        border: Rgb::new(73, 77, 100),
        selection: Rgb::new(54, 58, 79),
        text_primary: Rgb::new(202, 211, 245),
        text_secondary: Rgb::new(138, 173, 244),
        text_muted: Rgb::new(110, 115, 141),
        accent: Rgb::new(198, 160, 246),
        success: Rgb::new(166, 218, 149),
        warning: Rgb::new(238, 212, 159),
        error: Rgb::new(237, 135, 150),
        diff_add: Rgb::new(166, 218, 149),
        diff_del: Rgb::new(237, 135, 150),
        diff_context: Rgb::new(110, 115, 141),
        diff_hunk: Rgb::new(139, 213, 202),
    }
}

pub fn catppuccin_mocha() -> AppTheme {
    AppTheme {
        is_dark: true,
        background: Rgb::new(30, 30, 46),
        surface: Rgb::new(49, 50, 68),
        border: Rgb::new(69, 71, 90),
        selection: Rgb::new(49, 50, 68),
        text_primary: Rgb::new(205, 214, 244),
        text_secondary: Rgb::new(137, 180, 250),
        text_muted: Rgb::new(108, 112, 134),
        accent: Rgb::new(203, 166, 247),
        success: Rgb::new(166, 227, 161),
        warning: Rgb::new(249, 226, 175),
        error: Rgb::new(243, 139, 168),
        diff_add: Rgb::new(166, 227, 161),
        diff_del: Rgb::new(243, 139, 168),
        diff_context: Rgb::new(108, 112, 134),
        diff_hunk: Rgb::new(137, 180, 250),
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
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
}
