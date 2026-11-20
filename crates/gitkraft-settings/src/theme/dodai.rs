//! The Dodai design system as GitKraft themes.
//!
//! [Dodai UI](https://github.com/Dodai-Tech/dodai-ui) ships 62 themes (Dodai, Cyberpunk,
//! Material 3, Neo-Brutalism). Each one becomes a regular theme here, listed after the 43
//! built-ins, so every front-end (TUI, GUI, web) and every saved setting works unchanged: the
//! colours map onto [`AppTheme`], the *shape* (corner radius, chamfer, border width, hard
//! shadow, glow) comes through [`Design`] for the front-ends that can draw it.

use super::types::{AppTheme, Rgb};
use dodai_core::{Color, Mode, Theme};

/// The Dodai theme ids, in the order of the themes after the built-ins.
pub const DODAI_IDS: &[&str] = &[
    "dodai",
    "dodai-dark",
    "dodai-ocean",
    "dodai-ocean-dark",
    "dodai-forest",
    "dodai-forest-dark",
    "dodai-rose",
    "dodai-rose-dark",
    "dodai-sunset",
    "dodai-sunset-dark",
    "cyberpunk",
    "cyberpunk-cyan",
    "cyberpunk-red",
    "cyberpunk-light",
    "cyberpunk-cyan-light",
    "cyberpunk-red-light",
    "cyberpunk-magenta",
    "cyberpunk-acid",
    "cyberpunk-sunset",
    "cyberpunk-violet",
    "cyberpunk-ice",
    "cyberpunk-toxic",
    "cyberpunk-phantom",
    "cyberpunk-magenta-light",
    "cyberpunk-acid-light",
    "cyberpunk-sunset-light",
    "cyberpunk-violet-light",
    "cyberpunk-ice-light",
    "cyberpunk-toxic-light",
    "cyberpunk-phantom-light",
    "material",
    "material-dark",
    "material-pastel-lavender-light",
    "material-pastel-lavender-dark",
    "material-pastel-mint-light",
    "material-pastel-mint-dark",
    "material-pastel-peach-light",
    "material-pastel-peach-dark",
    "material-pastel-sky-light",
    "material-pastel-sky-dark",
    "material-ocean-light",
    "material-ocean-dark",
    "material-forest-light",
    "material-forest-dark",
    "material-rose-light",
    "material-rose-dark",
    "material-amber-light",
    "material-amber-dark",
    "neobrutalism",
    "neobrutalism-dark",
    "neobrutalism-yellow",
    "neobrutalism-yellow-dark",
    "neobrutalism-pink",
    "neobrutalism-pink-dark",
    "neobrutalism-green",
    "neobrutalism-green-dark",
    "neobrutalism-red",
    "neobrutalism-red-dark",
    "neobrutalism-violet",
    "neobrutalism-violet-dark",
    "neobrutalism-paper",
    "neobrutalism-paper-dark",
];

/// How strong a Dodai glow is drawn (1 = as the design defines it). Kept subtle for now; the
/// GUI and the web both read it, so they agree.
pub const GLOW: f32 = 0.15;

/// The look of a theme beyond its colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Design {
    /// Corner radius in px (0 = square).
    pub radius: f32,
    /// Cut-corner size in px (0 = none); the web draws it, the GUI falls back to a square corner.
    pub chamfer: f32,
    /// Border width in px.
    pub border_width: f32,
    /// Hard offset shadow in px (Neo-Brutalism), 0 = none.
    pub hard_shadow: f32,
    /// Glow strength: 0 = off, 1 = as designed.
    pub glow: f32,
    /// Labels and buttons in UPPERCASE.
    pub uppercase: bool,
    /// Is this a Dodai theme (rather than one of the built-ins)?
    pub dodai: bool,
}

impl Default for Design {
    /// What the built-in themes always looked like.
    fn default() -> Self {
        Self {
            radius: 6.0,
            chamfer: 0.0,
            border_width: 1.0,
            hard_shadow: 0.0,
            glow: 0.0,
            uppercase: false,
            dodai: false,
        }
    }
}

/// The Dodai theme at `n` (0-based, after the built-ins).
fn theme(n: usize) -> Option<Theme> {
    dodai_core::themes::by_id(DODAI_IDS.get(n)?)
}

/// `over` drawn on `under`, as opaque RGB.
fn flatten(over: &Color, under: &Color) -> Rgb {
    let a = f32::from(over.a) / 255.0;
    let mix = |o: u8, u: u8| (f32::from(o) * a + f32::from(u) * (1.0 - a)).round() as u8;
    Rgb::new(
        mix(over.r, under.r),
        mix(over.g, under.g),
        mix(over.b, under.b),
    )
}

fn rgb(c: &Color) -> Rgb {
    flatten(c, c)
}

fn blend(a: Rgb, b: Rgb) -> Rgb {
    let m = |x: u8, y: u8| ((u16::from(x) + u16::from(y)) / 2) as u8;
    Rgb::new(m(a.r, b.r), m(a.g, b.g), m(a.b, b.b))
}

/// WCAG relative luminance.
fn luminance(c: Rgb) -> f32 {
    let lin = |v: u8| {
        let s = f32::from(v) / 255.0;
        if s <= 0.039_28 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio of two colours (1 to 21).
#[must_use]
pub fn contrast(a: Rgb, b: Rgb) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The colour to use as GitKraft's accent: the first of the brand colours that reads on the
/// background (links, the active tab and the brand name are drawn in it), else the one that
/// reads best. A bright primary on a light page (Cyberpunk · Toxic Light) loses to the accent.
fn pick_accent(p: &dodai_core::Palette) -> Rgb {
    let bg = rgb(&p.background);
    let candidates = [rgb(&p.primary), rgb(&p.accent), rgb(&p.secondary)];
    let text = rgb(&p.text);
    candidates
        .iter()
        .find(|c| contrast(**c, bg) >= 3.0 && **c != text)
        .copied()
        .unwrap_or_else(|| {
            // Nothing reads (pastel designs): take the best and pull it towards the text colour
            // until it does.
            let mut best = candidates
                .into_iter()
                .max_by(|a, b| contrast(*a, bg).total_cmp(&contrast(*b, bg)))
                .unwrap_or(text);
            for _ in 0..10 {
                if contrast(best, bg) >= 3.0 {
                    break;
                }
                best = blend(best, text);
            }
            best
        })
}

/// The colours of the Dodai theme at `n`.
pub(super) fn app_theme(n: usize) -> Option<AppTheme> {
    let t = theme(n)?;
    let p = &t.palette;
    let bg = &p.background;
    let brand = pick_accent(p);
    let series = |i: usize| rgb(&p.series[i]);
    Some(AppTheme {
        is_dark: t.meta.mode == Mode::Dark,
        background: rgb(bg),
        surface: flatten(&p.surface, bg),
        border: rgb(&p.border),
        selection: flatten(&p.selection, bg),
        text_primary: rgb(&p.text),
        text_secondary: blend(rgb(&p.text), rgb(&p.text_muted)),
        text_muted: rgb(&p.text_muted),
        accent: brand,
        success: rgb(&p.success),
        warning: rgb(&p.warning),
        error: rgb(&p.danger),
        diff_add: rgb(&p.success),
        diff_del: rgb(&p.danger),
        diff_context: rgb(&p.text_muted),
        diff_hunk: rgb(&p.info),
        graph_colors: [
            brand,
            series(1),
            series(2),
            series(3),
            series(5),
            series(6),
            rgb(&p.secondary),
            series(0),
        ],
    })
}

/// The look of theme `index` (a built-in gets [`Design::default`]).
#[must_use]
pub fn design(index: usize) -> Design {
    let Some(t) = index.checked_sub(super::DODAI_FIRST).and_then(theme) else {
        return Design::default();
    };
    Design {
        radius: t.shape.radius,
        chamfer: t.shape.chamfer,
        border_width: t.shape.border_width,
        // A chamfered design cuts its corners instead; a shadow would be clipped to the cut.
        hard_shadow: if t.shape.chamfer > 0.0 {
            0.0
        } else {
            t.effects.hard_shadow
        },
        glow: if t.effects.glow_blur > 0.0 { GLOW } else { 0.0 },
        uppercase: t.typography.uppercase_labels,
        dodai: true,
    }
}

/// The Dodai id of theme `index`, for the web layer's `data-theme` (None for a built-in).
#[must_use]
pub fn dodai_id(index: usize) -> Option<&'static str> {
    DODAI_IDS
        .get(index.checked_sub(super::DODAI_FIRST)?)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{DODAI_FIRST, THEME_COUNT, THEME_NAMES, theme_by_index};

    #[test]
    fn the_list_matches_the_design_system_theme_for_theme() {
        let all = dodai_core::themes::all();
        assert_eq!(
            all.len(),
            DODAI_IDS.len(),
            "a theme was added to or dropped from Dodai UI"
        );
        assert_eq!(THEME_COUNT, DODAI_FIRST + all.len());
        assert_eq!(THEME_NAMES.len(), THEME_COUNT);
        for (i, t) in all.iter().enumerate() {
            assert_eq!(DODAI_IDS[i], t.meta.id);
            let name = t.meta.name;
            let expected = if name.starts_with("Dodai") {
                name.to_string()
            } else {
                format!("Dodai · {name}")
            };
            assert_eq!(THEME_NAMES[DODAI_FIRST + i], expected);
        }
    }

    #[test]
    fn every_dodai_theme_has_readable_text_and_a_matching_mode() {
        let lum =
            |c: Rgb| 0.2126 * f32::from(c.r) + 0.7152 * f32::from(c.g) + 0.0722 * f32::from(c.b);
        for (i, name) in THEME_NAMES.iter().enumerate().skip(DODAI_FIRST) {
            let t = theme_by_index(i);
            assert_eq!(lum(t.background) < 128.0, t.is_dark, "{name}");
            assert!(
                (lum(t.text_primary) - lum(t.background)).abs() > 100.0,
                "{name}"
            );
            assert!(
                contrast(t.accent, t.background) >= 3.0,
                "{name} {:.2}",
                contrast(t.accent, t.background)
            );
        }
    }

    #[test]
    fn the_shape_follows_the_design() {
        let cyber = design(DODAI_FIRST + DODAI_IDS.iter().position(|i| *i == "cyberpunk").unwrap());
        assert!(cyber.dodai && cyber.chamfer > 0.0 && cyber.radius == 0.0 && cyber.uppercase);
        assert_eq!(cyber.hard_shadow, 0.0, "cut corners, no hard shadow");
        assert!(cyber.glow > 0.0);
        let brut =
            design(DODAI_FIRST + DODAI_IDS.iter().position(|i| *i == "neobrutalism").unwrap());
        assert!(brut.hard_shadow > 0.0 && brut.border_width >= 2.0);
        assert_eq!(design(0), Design::default());
        assert_eq!(dodai_id(0), None);
        assert_eq!(dodai_id(DODAI_FIRST), Some("dodai"));
    }

    #[test]
    fn names_are_unique_and_found_again_by_name() {
        for (i, n) in THEME_NAMES.iter().enumerate() {
            assert_eq!(crate::theme::theme_index_by_name(n), i, "{n}");
        }
    }
}
