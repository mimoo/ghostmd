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
    Gruvbox,
    Catppuccin,
    TokyoNight,
    Kanagawa,
    Everforest,
    OneDark,
    Moonlight,
    AyuDark,
    Palenight,
    Vesper,
    SolarizedLight,
    CatppuccinLatte,
    RosePineDawn,
    GithubLight,
    AyuLight,
    GruvboxLight,
    EverforestLight,
    NordLight,
    TokyoNightDay,
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
            ThemeName::Gruvbox => Self::gruvbox(),
            ThemeName::Catppuccin => Self::catppuccin(),
            ThemeName::TokyoNight => Self::tokyo_night(),
            ThemeName::Kanagawa => Self::kanagawa(),
            ThemeName::Everforest => Self::everforest(),
            ThemeName::OneDark => Self::one_dark(),
            ThemeName::Moonlight => Self::moonlight(),
            ThemeName::AyuDark => Self::ayu_dark(),
            ThemeName::Palenight => Self::palenight(),
            ThemeName::Vesper => Self::vesper(),
            ThemeName::SolarizedLight => Self::solarized_light(),
            ThemeName::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeName::RosePineDawn => Self::rose_pine_dawn(),
            ThemeName::GithubLight => Self::github_light(),
            ThemeName::AyuLight => Self::ayu_light(),
            ThemeName::GruvboxLight => Self::gruvbox_light(),
            ThemeName::EverforestLight => Self::everforest_light(),
            ThemeName::NordLight => Self::nord_light(),
            ThemeName::TokyoNightDay => Self::tokyo_night_day(),
        }
    }

    fn rose_pine() -> Self {
        GhostTheme {
            bg: (0x19, 0x17, 0x24),          // base
            fg: (0xe0, 0xde, 0xf4),          // text
            selection: (0x26, 0x23, 0x3a),    // highlight low
            cursor: (0xeb, 0x6f, 0x92),      // love
            line_number: (0x6e, 0x6a, 0x86),  // muted
            sidebar_bg: (0x14, 0x12, 0x1f),
            tab_active: (0x26, 0x23, 0x3a),
            tab_inactive: (0x19, 0x17, 0x24),
            accent: (0xc4, 0xa7, 0xe7),      // iris
            error: (0xeb, 0x6f, 0x92),
            border: (0x26, 0x23, 0x3a),
            pane_title_bg: (0x1f, 0x1d, 0x2e),  // surface
            pane_title_fg: (0x6e, 0x6a, 0x86),
        }
    }

    fn nord() -> Self {
        GhostTheme {
            bg: (0x2e, 0x34, 0x40),         // polar night
            fg: (0xec, 0xef, 0xf4),         // snow storm
            selection: (0x3b, 0x42, 0x52),
            cursor: (0x88, 0xc0, 0xd0),     // frost
            line_number: (0x4c, 0x56, 0x6a),
            sidebar_bg: (0x27, 0x2e, 0x3a),
            tab_active: (0x3b, 0x42, 0x52),
            tab_inactive: (0x2e, 0x34, 0x40),
            accent: (0x81, 0xa1, 0xc1),     // frost blue
            error: (0xbf, 0x61, 0x6a),      // aurora red
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
            bg: (0xfa, 0xfa, 0xfa),
            fg: (0x1a, 0x1a, 0x1a),
            selection: (0xd7, 0xda, 0xe0),
            cursor: (0x40, 0x78, 0xf2),
            line_number: (0x9d, 0xa5, 0xb4),
            sidebar_bg: (0xf0, 0xf0, 0xf0),
            tab_active: (0xe5, 0xe5, 0xe6),
            tab_inactive: (0xfa, 0xfa, 0xfa),
            accent: (0x40, 0x78, 0xf2),
            error: (0xe4, 0x56, 0x49),
            border: (0xd3, 0xd3, 0xd4),
            pane_title_bg: (0xea, 0xea, 0xea),
            pane_title_fg: (0x6b, 0x72, 0x80),
        }
    }

    fn gruvbox() -> Self {
        GhostTheme {
            bg: (0x28, 0x28, 0x28),         // #282828
            fg: (0xeb, 0xdb, 0xb2),         // #EBDBB2
            selection: (0x3c, 0x38, 0x36),   // #3C3836
            cursor: (0xfe, 0x80, 0x19),     // #FE8019
            line_number: (0x66, 0x5c, 0x54), // #665C54
            sidebar_bg: (0x1d, 0x20, 0x21),  // #1D2021
            tab_active: (0x3c, 0x38, 0x36),
            tab_inactive: (0x28, 0x28, 0x28),
            accent: (0xfa, 0xbd, 0x2f),     // #FABD2F
            error: (0xfb, 0x49, 0x34),      // #FB4934
            border: (0x3c, 0x38, 0x36),
            pane_title_bg: (0x32, 0x30, 0x2f),
            pane_title_fg: (0x66, 0x5c, 0x54),
        }
    }

    fn catppuccin() -> Self {
        GhostTheme {
            bg: (0x1e, 0x1e, 0x2e),         // mocha base
            fg: (0xcd, 0xd6, 0xf4),         // text
            selection: (0x31, 0x32, 0x44),   // surface0
            cursor: (0xf5, 0xc2, 0xe7),     // pink
            line_number: (0x58, 0x5b, 0x70), // overlay0
            sidebar_bg: (0x18, 0x18, 0x25),  // mantle
            tab_active: (0x31, 0x32, 0x44),
            tab_inactive: (0x1e, 0x1e, 0x2e),
            accent: (0x89, 0xb4, 0xfa),     // blue
            error: (0xf3, 0x8b, 0xa8),      // red
            border: (0x31, 0x32, 0x44),
            pane_title_bg: (0x24, 0x24, 0x38),
            pane_title_fg: (0x58, 0x5b, 0x70),
        }
    }

    fn tokyo_night() -> Self {
        GhostTheme {
            bg: (0x1a, 0x1b, 0x26),         // bg_dark
            fg: (0xc0, 0xca, 0xf5),         // fg
            selection: (0x28, 0x2d, 0x42),
            cursor: (0x7a, 0xa2, 0xf7),     // blue
            line_number: (0x3b, 0x40, 0x61), // comment
            sidebar_bg: (0x16, 0x16, 0x1e),
            tab_active: (0x28, 0x2d, 0x42),
            tab_inactive: (0x1a, 0x1b, 0x26),
            accent: (0x7d, 0xcf, 0xff),     // cyan
            error: (0xf7, 0x76, 0x8e),      // red
            border: (0x28, 0x2d, 0x42),
            pane_title_bg: (0x20, 0x22, 0x34),
            pane_title_fg: (0x3b, 0x40, 0x61),
        }
    }

    fn kanagawa() -> Self {
        GhostTheme {
            bg: (0x1f, 0x1f, 0x28),         // sumiInk1
            fg: (0xdc, 0xd7, 0xba),         // fujiWhite
            selection: (0x2a, 0x2a, 0x37),   // sumiInk4
            cursor: (0xe6, 0xc3, 0x84),     // carpYellow
            line_number: (0x54, 0x54, 0x6d), // sumiInk6
            sidebar_bg: (0x1a, 0x1a, 0x22),
            tab_active: (0x2a, 0x2a, 0x37),
            tab_inactive: (0x1f, 0x1f, 0x28),
            accent: (0x7e, 0x9c, 0xd8),     // crystalBlue
            error: (0xe8, 0x2a, 0x2a),      // samuraiRed
            border: (0x2a, 0x2a, 0x37),
            pane_title_bg: (0x25, 0x25, 0x30),
            pane_title_fg: (0x54, 0x54, 0x6d),
        }
    }

    fn everforest() -> Self {
        GhostTheme {
            bg: (0x2d, 0x35, 0x3b),         // bg_dim
            fg: (0xd3, 0xc6, 0xaa),         // fg
            selection: (0x3d, 0x48, 0x4d),   // bg3
            cursor: (0xa7, 0xc0, 0x80),     // green
            line_number: (0x60, 0x72, 0x6a), // grey1
            sidebar_bg: (0x27, 0x2e, 0x33),  // bg0
            tab_active: (0x3d, 0x48, 0x4d),
            tab_inactive: (0x2d, 0x35, 0x3b),
            accent: (0x83, 0xc0, 0x92),     // aqua
            error: (0xe6, 0x7e, 0x80),      // red
            border: (0x3d, 0x48, 0x4d),
            pane_title_bg: (0x34, 0x3f, 0x44),  // bg1
            pane_title_fg: (0x60, 0x72, 0x6a),
        }
    }

    fn one_dark() -> Self {
        GhostTheme {
            bg: (0x28, 0x2c, 0x34),         // bg
            fg: (0xab, 0xb2, 0xbf),         // fg
            selection: (0x3e, 0x44, 0x51),
            cursor: (0x61, 0xaf, 0xef),     // blue
            line_number: (0x4b, 0x52, 0x63), // comment
            sidebar_bg: (0x21, 0x25, 0x2b),
            tab_active: (0x3e, 0x44, 0x51),
            tab_inactive: (0x28, 0x2c, 0x34),
            accent: (0x61, 0xaf, 0xef),     // blue
            error: (0xe0, 0x6c, 0x75),      // red
            border: (0x3e, 0x44, 0x51),
            pane_title_bg: (0x2e, 0x33, 0x3e),
            pane_title_fg: (0x4b, 0x52, 0x63),
        }
    }

    fn moonlight() -> Self {
        GhostTheme {
            bg: (0x1e, 0x20, 0x30),
            fg: (0xc8, 0xd3, 0xf5),
            selection: (0x2f, 0x33, 0x4d),
            cursor: (0xff, 0x75, 0x7f),     // red
            line_number: (0x44, 0x49, 0x6d),
            sidebar_bg: (0x19, 0x1a, 0x2a),
            tab_active: (0x2f, 0x33, 0x4d),
            tab_inactive: (0x1e, 0x20, 0x30),
            accent: (0x82, 0xaa, 0xff),     // blue
            error: (0xff, 0x75, 0x7f),
            border: (0x2f, 0x33, 0x4d),
            pane_title_bg: (0x26, 0x29, 0x3e),
            pane_title_fg: (0x44, 0x49, 0x6d),
        }
    }

    fn ayu_dark() -> Self {
        GhostTheme {
            bg: (0x0b, 0x0e, 0x14),         // bg
            fg: (0xbf, 0xbd, 0xb6),         // fg
            selection: (0x1a, 0x1e, 0x2b),
            cursor: (0xe6, 0xb4, 0x50),     // accent
            line_number: (0x46, 0x4d, 0x56),
            sidebar_bg: (0x07, 0x0a, 0x0f),
            tab_active: (0x1a, 0x1e, 0x2b),
            tab_inactive: (0x0b, 0x0e, 0x14),
            accent: (0xe6, 0xb4, 0x50),     // orange accent
            error: (0xd9, 0x57, 0x57),
            border: (0x1a, 0x1e, 0x2b),
            pane_title_bg: (0x12, 0x15, 0x1e),
            pane_title_fg: (0x46, 0x4d, 0x56),
        }
    }

    fn palenight() -> Self {
        GhostTheme {
            bg: (0x29, 0x2d, 0x3e),
            fg: (0xa6, 0xac, 0xcd),
            selection: (0x3a, 0x3f, 0x58),
            cursor: (0xff, 0xcb, 0x6b),     // yellow
            line_number: (0x4e, 0x55, 0x79),
            sidebar_bg: (0x22, 0x26, 0x36),
            tab_active: (0x3a, 0x3f, 0x58),
            tab_inactive: (0x29, 0x2d, 0x3e),
            accent: (0xc7, 0x92, 0xea),     // purple
            error: (0xf0, 0x71, 0x78),
            border: (0x3a, 0x3f, 0x58),
            pane_title_bg: (0x30, 0x35, 0x4a),
            pane_title_fg: (0x4e, 0x55, 0x79),
        }
    }

    fn vesper() -> Self {
        GhostTheme {
            bg: (0x10, 0x10, 0x10),         // deep black
            fg: (0xb0, 0xb0, 0xb0),         // muted gray
            selection: (0x22, 0x22, 0x22),
            cursor: (0xff, 0xc7, 0x99),     // warm peach
            line_number: (0x40, 0x40, 0x40),
            sidebar_bg: (0x0a, 0x0a, 0x0a),
            tab_active: (0x22, 0x22, 0x22),
            tab_inactive: (0x10, 0x10, 0x10),
            accent: (0xff, 0xc7, 0x99),     // warm peach
            error: (0xf5, 0x6e, 0x6e),
            border: (0x22, 0x22, 0x22),
            pane_title_bg: (0x18, 0x18, 0x18),
            pane_title_fg: (0x40, 0x40, 0x40),
        }
    }

    fn solarized_light() -> Self {
        GhostTheme {
            bg: (0xfd, 0xf6, 0xe3),         // #FDF6E3 base3
            fg: (0x65, 0x7b, 0x83),         // #657B83 base00
            selection: (0xd6, 0xd0, 0xbe),   // darkened base2 for visible palette highlight
            cursor: (0x26, 0x8b, 0xd2),     // #268BD2 blue
            line_number: (0x93, 0xa1, 0xa1), // #93A1A1 base1
            sidebar_bg: (0xee, 0xe8, 0xd5),  // base2
            tab_active: (0xee, 0xe8, 0xd5),
            tab_inactive: (0xfd, 0xf6, 0xe3),
            accent: (0x26, 0x8b, 0xd2),     // blue
            error: (0xdc, 0x32, 0x2f),      // #DC322F red
            border: (0xe0, 0xdb, 0xc9),
            pane_title_bg: (0xf3, 0xee, 0xdc),
            pane_title_fg: (0x93, 0xa1, 0xa1),
        }
    }

    fn catppuccin_latte() -> Self {
        GhostTheme {
            bg: (0xef, 0xf1, 0xf5),         // #EFF1F5 base
            fg: (0x4c, 0x4f, 0x69),         // #4C4F69 text
            selection: (0xbc, 0xc0, 0xcc),   // #BCC0CC surface1 for visible palette
            cursor: (0xfe, 0x64, 0x0b),     // #FE640B peach
            line_number: (0x8c, 0x8f, 0xa1), // #8C8FA1 overlay0
            sidebar_bg: (0xe6, 0xe9, 0xef),  // #E6E9EF mantle
            tab_active: (0xcc, 0xd0, 0xda),
            tab_inactive: (0xef, 0xf1, 0xf5),
            accent: (0x12, 0x87, 0xa8),     // #1287A8 teal
            error: (0xd2, 0x00, 0x42),      // #D20042 red (maroon)
            border: (0xbc, 0xc0, 0xcc),      // #BCC0CC surface1
            pane_title_bg: (0xdc, 0xe0, 0xe8), // #DCE0E8 crust
            pane_title_fg: (0x8c, 0x8f, 0xa1),
        }
    }

    fn rose_pine_dawn() -> Self {
        GhostTheme {
            bg: (0xfa, 0xf4, 0xed),         // #FAF4ED base
            fg: (0x57, 0x52, 0x79),         // #575279 text
            selection: (0xdf, 0xd8, 0xd0),   // #DFD8D0 highlight med for visible palette
            cursor: (0xb4, 0x63, 0x7a),     // #B4637A love
            line_number: (0x9e, 0x93, 0x86), // #9E9386 muted (subtle)
            sidebar_bg: (0xf2, 0xe9, 0xde),  // #F2E9DE surface
            tab_active: (0xf2, 0xe9, 0xe1),
            tab_inactive: (0xfa, 0xf4, 0xed),
            accent: (0x90, 0x7a, 0xa9),     // #907AA9 iris
            error: (0xb4, 0x63, 0x7a),      // love
            border: (0xe4, 0xdf, 0xd7),
            pane_title_bg: (0xf4, 0xed, 0xe5),
            pane_title_fg: (0x9e, 0x93, 0x86),
        }
    }

    fn github_light() -> Self {
        GhostTheme {
            bg: (0xff, 0xff, 0xff),         // #FFFFFF
            fg: (0x1f, 0x23, 0x28),         // #1F2328 fg.default
            selection: (0xd1, 0xd9, 0xe0),   // accent.muted
            cursor: (0x03, 0x6f, 0xfc),     // #036FFC accent.fg
            line_number: (0x8b, 0x94, 0x9e), // #8B949E fg.muted
            sidebar_bg: (0xf6, 0xf8, 0xfa),  // #F6F8FA canvas.subtle
            tab_active: (0xff, 0xff, 0xff),
            tab_inactive: (0xf6, 0xf8, 0xfa),
            accent: (0x03, 0x69, 0xd6),     // #0369D6 accent.fg
            error: (0xcf, 0x22, 0x2e),      // #CF222E danger.fg
            border: (0xd1, 0xd9, 0xe0),     // #D1D9E0 border.default
            pane_title_bg: (0xf6, 0xf8, 0xfa),
            pane_title_fg: (0x8b, 0x94, 0x9e),
        }
    }

    fn ayu_light() -> Self {
        GhostTheme {
            bg: (0xfc, 0xfc, 0xf0),         // #FCFCF0
            fg: (0x5c, 0x61, 0x66),         // #5C6166
            selection: (0xd1, 0xdc, 0xe4),
            cursor: (0xff, 0x9d, 0x33),     // #FF9D33 accent
            line_number: (0x8a, 0x91, 0x99), // #8A9199
            sidebar_bg: (0xf3, 0xf3, 0xe4),
            tab_active: (0xe8, 0xe8, 0xd8),
            tab_inactive: (0xfc, 0xfc, 0xf0),
            accent: (0xff, 0x9d, 0x33),     // orange
            error: (0xf0, 0x71, 0x71),      // #F07171
            border: (0xd8, 0xda, 0xcd),
            pane_title_bg: (0xf0, 0xf0, 0xe2),
            pane_title_fg: (0x8a, 0x91, 0x99),
        }
    }

    fn gruvbox_light() -> Self {
        GhostTheme {
            bg: (0xfb, 0xf1, 0xc7),         // #FBF1C7 bg0
            fg: (0x3c, 0x38, 0x36),         // #3C3836 fg1
            selection: (0xd5, 0xc4, 0xa1),   // #D5C4A1 bg2
            cursor: (0xd6, 0x5d, 0x0e),     // #D65D0E orange
            line_number: (0x92, 0x83, 0x74), // #928374 gray
            sidebar_bg: (0xf2, 0xe5, 0xbc),  // #F2E5BC bg1
            tab_active: (0xe2, 0xd5, 0xae),
            tab_inactive: (0xfb, 0xf1, 0xc7),
            accent: (0xb5, 0x76, 0x14),     // #B57614 yellow
            error: (0xcc, 0x24, 0x1d),      // #CC241D red
            border: (0xd5, 0xc4, 0xa1),
            pane_title_bg: (0xeb, 0xdb, 0xb2),
            pane_title_fg: (0x92, 0x83, 0x74),
        }
    }

    fn everforest_light() -> Self {
        GhostTheme {
            bg: (0xfe, 0xf6, 0xe4),         // slightly shifted from Solarized Light's #FDF6E3
            fg: (0x5c, 0x6a, 0x72),         // #5C6A72 fg
            selection: (0xe0, 0xda, 0xc6),   // #E0DAC6 bg3
            cursor: (0x8d, 0xa1, 0x01),     // #8DA101 green
            line_number: (0x93, 0x9f, 0x91), // #939F91 grey0
            sidebar_bg: (0xf4, 0xf0, 0xd9),  // #F4F0D9 bg1
            tab_active: (0xe6, 0xe2, 0xcc),
            tab_inactive: (0xfd, 0xf6, 0xe3),
            accent: (0x35, 0xa7, 0x7c),     // #35A77C aqua
            error: (0xf8, 0x55, 0x52),      // #F85552 red
            border: (0xdd, 0xd8, 0xc4),
            pane_title_bg: (0xef, 0xea, 0xd5),
            pane_title_fg: (0x93, 0x9f, 0x91),
        }
    }

    fn nord_light() -> Self {
        GhostTheme {
            bg: (0xec, 0xef, 0xf4),         // #ECEFF4 snow storm 3
            fg: (0x2e, 0x34, 0x40),         // #2E3440 polar night 0
            selection: (0xd0, 0xd6, 0xe1),
            cursor: (0x5e, 0x81, 0xac),     // #5E81AC frost 3
            line_number: (0x7b, 0x88, 0x9e),
            sidebar_bg: (0xe5, 0xe9, 0xf0),  // #E5E9F0 snow storm 2
            tab_active: (0xd8, 0xde, 0xe9),  // #D8DEE9 snow storm 1
            tab_inactive: (0xec, 0xef, 0xf4),
            accent: (0x5e, 0x81, 0xac),     // #5E81AC frost
            error: (0xbf, 0x61, 0x6a),      // #BF616A aurora red
            border: (0xd8, 0xde, 0xe9),
            pane_title_bg: (0xe0, 0xe5, 0xed),
            pane_title_fg: (0x7b, 0x88, 0x9e),
        }
    }

    fn tokyo_night_day() -> Self {
        GhostTheme {
            bg: (0xe1, 0xe2, 0xe7),         // #E1E2E7 bg
            fg: (0x34, 0x37, 0x64),         // #343764 fg
            selection: (0xc4, 0xc8, 0xda),   // #C4C8DA bg_visual
            cursor: (0x2e, 0x7d, 0xe9),     // #2E7DE9 blue
            line_number: (0x84, 0x8c, 0xb5), // #848CB5 dark3
            sidebar_bg: (0xd5, 0xd6, 0xdb),  // #D5D6DB bg_sidebar
            tab_active: (0xc9, 0xcc, 0xd7),
            tab_inactive: (0xe1, 0xe2, 0xe7),
            accent: (0x2e, 0x7d, 0xe9),     // blue
            error: (0xf5, 0x2a, 0x65),      // #F52A65 red
            border: (0xc4, 0xc8, 0xda),
            pane_title_bg: (0xd5, 0xd8, 0xe2),
            pane_title_fg: (0x84, 0x8c, 0xb5),
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

/// Pre-converted HSLA colors for use in rendering (avoids 50+ rgb_to_hsla calls per frame).
pub struct ResolvedTheme {
    pub bg: Hsla,
    pub fg: Hsla,
    pub selection: Hsla,
    #[allow(dead_code)]
    pub cursor: Hsla,
    pub hint: Hsla,
    pub sidebar_bg: Hsla,
    #[allow(dead_code)]
    pub tab_active: Hsla,
    #[allow(dead_code)]
    pub tab_inactive: Hsla,
    pub accent: Hsla,
    pub error: Hsla,
    pub border: Hsla,
    pub pane_title_bg: Hsla,
    pub pane_title_fg: Hsla,
}

impl ResolvedTheme {
    pub fn from_name(name: ThemeName) -> Self {
        let g = GhostTheme::from_name(name);
        let c = |rgb: Rgb| rgb_to_hsla(rgb.0, rgb.1, rgb.2);
        Self {
            bg: c(g.bg),
            fg: c(g.fg),
            selection: c(g.selection),
            cursor: c(g.cursor),
            hint: c(g.line_number),
            sidebar_bg: c(g.sidebar_bg),
            tab_active: c(g.tab_active),
            tab_inactive: c(g.tab_inactive),
            accent: c(g.accent),
            error: c(g.error),
            border: c(g.border),
            pane_title_bg: c(g.pane_title_bg),
            pane_title_fg: c(g.pane_title_fg),
        }
    }
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
    let mode = if matches!(name, ThemeName::Light | ThemeName::SolarizedLight | ThemeName::CatppuccinLatte | ThemeName::RosePineDawn | ThemeName::GithubLight | ThemeName::AyuLight | ThemeName::GruvboxLight | ThemeName::EverforestLight | ThemeName::NordLight | ThemeName::TokyoNightDay) { ThemeMode::Light } else { ThemeMode::Dark };
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
            ThemeName::Dracula, ThemeName::Light, ThemeName::Gruvbox,
            ThemeName::Catppuccin, ThemeName::TokyoNight, ThemeName::Kanagawa,
            ThemeName::Everforest, ThemeName::OneDark, ThemeName::Moonlight,
            ThemeName::AyuDark, ThemeName::Palenight, ThemeName::Vesper,
            ThemeName::SolarizedLight, ThemeName::CatppuccinLatte,
            ThemeName::RosePineDawn, ThemeName::GithubLight,
            ThemeName::AyuLight, ThemeName::GruvboxLight,
            ThemeName::EverforestLight, ThemeName::NordLight,
            ThemeName::TokyoNightDay,
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
