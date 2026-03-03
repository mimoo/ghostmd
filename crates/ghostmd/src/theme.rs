use gpui::{px, Hsla};
use gpui_component::theme::{Theme, ThemeMode};
use serde::{Serialize, Deserialize};

/// A color represented as an RGB tuple.
pub type Rgb = (u8, u8, u8);

/// Available theme names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    #[default]
    RosePine,
    Nord,
    Solarized,
    Dracula,
    Light,
}

/// The GhostMD color theme.
pub struct GhostTheme {
    pub bg: Rgb,
    pub fg: Rgb,
    pub selection: Rgb,
    pub cursor: Rgb,
    pub line_number: Rgb,
    pub sidebar_bg: Rgb,
    pub tab_active: Rgb,
    pub tab_inactive: Rgb,
    pub accent: Rgb,
    pub error: Rgb,
    pub border: Rgb,
    pub pane_title_bg: Rgb,
    pub pane_title_fg: Rgb,
}

impl GhostTheme {
    /// Returns the default dark theme (warm/rose-pine inspired).
    #[allow(dead_code)]
    pub fn default_dark() -> Self {
        Self::rose_pine()
    }

    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::RosePine => Self::rose_pine(),
            ThemeName::Nord => Self::nord(),
            ThemeName::Solarized => Self::solarized(),
            ThemeName::Dracula => Self::dracula(),
            ThemeName::Light => Self::light(),
        }
    }

    fn rose_pine() -> Self {
        GhostTheme {
            bg: (0x1a, 0x1a, 0x2e),
            fg: (0xe0, 0xde, 0xf4),
            selection: (0x2a, 0x2a, 0x4a),
            cursor: (0xeb, 0x6f, 0x92),
            line_number: (0x6e, 0x6a, 0x86),
            sidebar_bg: (0x16, 0x16, 0x2a),
            tab_active: (0x2a, 0x2a, 0x4a),
            tab_inactive: (0x1a, 0x1a, 0x2e),
            accent: (0x9c, 0xce, 0xf8),
            error: (0xeb, 0x6f, 0x92),
            border: (0x2a, 0x2a, 0x4a),
            pane_title_bg: (0x22, 0x22, 0x3a),
            pane_title_fg: (0x6e, 0x6a, 0x86),
        }
    }

    fn nord() -> Self {
        GhostTheme {
            bg: (0x2e, 0x34, 0x40),         // #2E3440
            fg: (0xec, 0xef, 0xf4),         // #ECEFF4
            selection: (0x3b, 0x42, 0x52),   // #3B4252
            cursor: (0x88, 0xc0, 0xd0),     // #88C0D0
            line_number: (0x4c, 0x56, 0x6a), // #4C566A
            sidebar_bg: (0x27, 0x2e, 0x3a),  // slightly darker
            tab_active: (0x3b, 0x42, 0x52),
            tab_inactive: (0x2e, 0x34, 0x40),
            accent: (0x88, 0xc0, 0xd0),     // #88C0D0
            error: (0xbf, 0x61, 0x6a),      // #BF616A
            border: (0x3b, 0x42, 0x52),
            pane_title_bg: (0x34, 0x3b, 0x48),
            pane_title_fg: (0x4c, 0x56, 0x6a),
        }
    }

    fn solarized() -> Self {
        GhostTheme {
            bg: (0x00, 0x2b, 0x36),         // #002B36
            fg: (0x83, 0x94, 0x96),         // #839496
            selection: (0x07, 0x36, 0x42),   // #073642
            cursor: (0x26, 0x8b, 0xd2),     // #268BD2
            line_number: (0x58, 0x6e, 0x75), // #586E75
            sidebar_bg: (0x00, 0x24, 0x2e),
            tab_active: (0x07, 0x36, 0x42),
            tab_inactive: (0x00, 0x2b, 0x36),
            accent: (0x26, 0x8b, 0xd2),     // #268BD2
            error: (0xdc, 0x32, 0x2f),      // #DC322F
            border: (0x07, 0x36, 0x42),
            pane_title_bg: (0x04, 0x30, 0x3c),
            pane_title_fg: (0x58, 0x6e, 0x75),
        }
    }

    fn dracula() -> Self {
        GhostTheme {
            bg: (0x28, 0x2a, 0x36),         // #282A36
            fg: (0xf8, 0xf8, 0xf2),         // #F8F8F2
            selection: (0x44, 0x47, 0x5a),   // #44475A
            cursor: (0xbd, 0x93, 0xf9),     // #BD93F9
            line_number: (0x62, 0x72, 0xa4), // #6272A4
            sidebar_bg: (0x21, 0x22, 0x2c),
            tab_active: (0x44, 0x47, 0x5a),
            tab_inactive: (0x28, 0x2a, 0x36),
            accent: (0xbd, 0x93, 0xf9),     // #BD93F9
            error: (0xff, 0x55, 0x55),      // #FF5555
            border: (0x44, 0x47, 0x5a),
            pane_title_bg: (0x34, 0x36, 0x46),
            pane_title_fg: (0x62, 0x72, 0xa4),
        }
    }

    fn light() -> Self {
        GhostTheme {
            bg: (0xfa, 0xfa, 0xfa),         // #FAFAFA
            fg: (0x38, 0x3a, 0x42),         // #383A42
            selection: (0xe5, 0xe5, 0xe6),   // #E5E5E6
            cursor: (0x40, 0x78, 0xf2),     // #4078F2
            line_number: (0x9d, 0xa5, 0xb4), // #9DA5B4
            sidebar_bg: (0xf0, 0xf0, 0xf0),
            tab_active: (0xe5, 0xe5, 0xe6),
            tab_inactive: (0xfa, 0xfa, 0xfa),
            accent: (0x40, 0x78, 0xf2),     // #4078F2
            error: (0xe4, 0x56, 0x49),      // #E45649
            border: (0xd3, 0xd3, 0xd4),
            pane_title_bg: (0xea, 0xea, 0xea),
            pane_title_fg: (0x9d, 0xa5, 0xb4),
        }
    }
}

/// Convert an RGB tuple to GPUI's Hsla color space.
pub fn rgb_to_hsla(r: u8, g: u8, b: u8) -> Hsla {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f32::EPSILON {
        return gpui::hsla(0.0, 0.0, l, 1.0);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f32::EPSILON {
        let mut h = (g - b) / d;
        if g < b {
            h += 6.0;
        }
        h / 6.0
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    gpui::hsla(h, s, l, 1.0)
}

/// Apply GhostTheme colors to the gpui-component Theme global.
fn apply_theme_colors(ghost: &GhostTheme, theme: &mut Theme) {
    theme.colors.background = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
    theme.colors.selection = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.caret = rgb_to_hsla(ghost.cursor.0, ghost.cursor.1, ghost.cursor.2);
    theme.colors.accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);
    theme.colors.danger = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);
    theme.colors.danger_foreground = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);

    theme.colors.sidebar = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
    theme.colors.sidebar_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.sidebar_border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

    theme.colors.tab = rgb_to_hsla(ghost.tab_inactive.0, ghost.tab_inactive.1, ghost.tab_inactive.2);
    theme.colors.tab_active = rgb_to_hsla(ghost.tab_active.0, ghost.tab_active.1, ghost.tab_active.2);
    theme.colors.tab_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.tab_active_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.tab_bar = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);

    theme.colors.popover = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
    theme.colors.popover_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);

    theme.colors.list = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.list_active = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.list_hover = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);

    theme.colors.muted = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.muted_foreground = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

    theme.colors.title_bar = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.title_bar_border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_family = "JetBrains Mono".into();
    theme.font_size = px(14.0);
    theme.mono_font_size = px(14.0);
}

/// Initialize gpui-component and apply GhostMD's default theme colors.
pub fn apply_ghost_theme(cx: &mut gpui::App) {
    gpui_component::init(cx);
    apply_theme(ThemeName::default(), cx);
}

/// Switch to a named theme at runtime.
pub fn apply_theme(name: ThemeName, cx: &mut gpui::App) {
    let ghost = GhostTheme::from_name(name);
    let mode = if matches!(name, ThemeName::Light) { ThemeMode::Light } else { ThemeMode::Dark };
    Theme::change(mode, None, cx);
    let theme = Theme::global_mut(cx);
    apply_theme_colors(&ghost, theme);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dark_has_dark_background() {
        let theme = GhostTheme::default_dark();
        assert!(theme.bg.0 < 50);
        assert!(theme.bg.1 < 50);
        assert!(theme.bg.2 < 80);
    }

    #[test]
    fn default_dark_has_light_foreground() {
        let theme = GhostTheme::default_dark();
        assert!(theme.fg.0 > 200);
        assert!(theme.fg.1 > 200);
        assert!(theme.fg.2 > 200);
    }

    #[test]
    fn sidebar_darker_than_bg() {
        let theme = GhostTheme::default_dark();
        let sidebar_lum = theme.sidebar_bg.0 as u16
            + theme.sidebar_bg.1 as u16
            + theme.sidebar_bg.2 as u16;
        let bg_lum =
            theme.bg.0 as u16 + theme.bg.1 as u16 + theme.bg.2 as u16;
        assert!(sidebar_lum <= bg_lum);
    }

    #[test]
    fn all_themes_have_distinct_bg() {
        let themes: Vec<_> = [
            ThemeName::RosePine, ThemeName::Nord, ThemeName::Solarized,
            ThemeName::Dracula, ThemeName::Light,
        ].iter().map(|n| GhostTheme::from_name(*n).bg).collect();
        for i in 0..themes.len() {
            for j in (i+1)..themes.len() {
                assert_ne!(themes[i], themes[j], "themes {i} and {j} have same bg");
            }
        }
    }

    #[test]
    fn light_theme_has_light_bg() {
        let theme = GhostTheme::from_name(ThemeName::Light);
        assert!(theme.bg.0 > 200);
        assert!(theme.bg.1 > 200);
        assert!(theme.bg.2 > 200);
    }

    #[test]
    fn from_name_matches_default_dark() {
        let default = GhostTheme::default_dark();
        let rose = GhostTheme::from_name(ThemeName::RosePine);
        assert_eq!(default.bg, rose.bg);
        assert_eq!(default.fg, rose.fg);
    }
}
