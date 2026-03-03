use gpui::{px, Hsla};
use gpui_component::theme::{Theme, ThemeMode};

/// A color represented as an RGB tuple.
pub type Rgb = (u8, u8, u8);

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
}

impl GhostTheme {
    /// Returns the default dark theme (warm/rose-pine inspired).
    pub fn default_dark() -> Self {
        GhostTheme {
            bg: (0x1a, 0x1a, 0x2e),         // #1a1a2e
            fg: (0xe0, 0xde, 0xf4),         // #e0def4
            selection: (0x2a, 0x2a, 0x4a),   // #2a2a4a
            cursor: (0xeb, 0x6f, 0x92),      // #eb6f92
            line_number: (0x6e, 0x6a, 0x86), // #6e6a86
            sidebar_bg: (0x16, 0x16, 0x2a),  // #16162a
            tab_active: (0x2a, 0x2a, 0x4a),  // #2a2a4a
            tab_inactive: (0x1a, 0x1a, 0x2e),// #1a1a2e
            accent: (0x9c, 0xce, 0xf8),      // #9ccef8
            error: (0xeb, 0x6f, 0x92),       // #eb6f92
            border: (0x2a, 0x2a, 0x4a),      // #2a2a4a
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

/// Initialize gpui-component and apply GhostMD's dark theme colors.
pub fn apply_ghost_theme(cx: &mut gpui::App) {
    gpui_component::init(cx);

    // Force dark mode
    Theme::change(ThemeMode::Dark, None, cx);

    let ghost = GhostTheme::default_dark();
    let theme = Theme::global_mut(cx);

    // Override core colors
    theme.colors.background = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
    theme.colors.selection = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.caret = rgb_to_hsla(ghost.cursor.0, ghost.cursor.1, ghost.cursor.2);
    theme.colors.accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);
    theme.colors.danger = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);
    theme.colors.danger_foreground = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);

    // Sidebar
    theme.colors.sidebar = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
    theme.colors.sidebar_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.sidebar_border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

    // Tabs
    theme.colors.tab = rgb_to_hsla(ghost.tab_inactive.0, ghost.tab_inactive.1, ghost.tab_inactive.2);
    theme.colors.tab_active = rgb_to_hsla(ghost.tab_active.0, ghost.tab_active.1, ghost.tab_active.2);
    theme.colors.tab_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.tab_active_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
    theme.colors.tab_bar = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);

    // Popover / overlay
    theme.colors.popover = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
    theme.colors.popover_foreground = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);

    // List
    theme.colors.list = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.list_active = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.list_hover = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);

    // Muted
    theme.colors.muted = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
    theme.colors.muted_foreground = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

    // Title bar
    theme.colors.title_bar = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
    theme.colors.title_bar_border = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

    // Font settings
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_family = "JetBrains Mono".into();
    theme.font_size = px(14.0);
    theme.mono_font_size = px(14.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dark_has_dark_background() {
        let theme = GhostTheme::default_dark();
        // Background should be dark (all components < 50)
        assert!(theme.bg.0 < 50);
        assert!(theme.bg.1 < 50);
        assert!(theme.bg.2 < 80);
    }

    #[test]
    fn default_dark_has_light_foreground() {
        let theme = GhostTheme::default_dark();
        // Foreground should be light (all components > 200)
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
}
