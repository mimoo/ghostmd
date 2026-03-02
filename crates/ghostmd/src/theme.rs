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
