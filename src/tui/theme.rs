//! Theme support for the TUI
//!
//! Provides multiple color themes including Catppuccin, Dracula, and Nord.

use ratatui::style::Color;

/// A complete color theme for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    // Base colors
    pub base: Color,     // Main background
    pub surface0: Color, // Slightly elevated surface
    pub surface1: Color, // Borders, separators
    // Text colors
    pub text: Color,     // Primary text
    pub subtext0: Color, // Secondary/dimmed text
    // Accent colors
    pub blue: Color,   // Links, highlights
    pub green: Color,  // Success, installed
    pub yellow: Color, // Warnings, stars
    pub red: Color,    // Errors, destructive
    pub mauve: Color,  // Categories
    pub peach: Color,  // Source badges
    pub teal: Color,   // Sparklines, metrics
}

/// Available theme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeVariant {
    #[default]
    CatppuccinMocha,
    CatppuccinLatte,
    Dracula,
    Nord,
    TokyoNight,
    Gruvbox,
}

impl ThemeVariant {
    /// Get the theme for this variant
    pub fn theme(&self) -> Theme {
        match self {
            Self::CatppuccinMocha => CATPPUCCIN_MOCHA,
            Self::CatppuccinLatte => CATPPUCCIN_LATTE,
            Self::Dracula => DRACULA,
            Self::Nord => NORD,
            Self::TokyoNight => TOKYO_NIGHT,
            Self::Gruvbox => GRUVBOX,
        }
    }

    /// Cycle to the next theme
    pub fn next(&self) -> Self {
        match self {
            Self::CatppuccinMocha => Self::CatppuccinLatte,
            Self::CatppuccinLatte => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::TokyoNight,
            Self::TokyoNight => Self::Gruvbox,
            Self::Gruvbox => Self::CatppuccinMocha,
        }
    }

    /// Get all available variants
    pub fn all() -> &'static [ThemeVariant] {
        &[
            Self::CatppuccinMocha,
            Self::CatppuccinLatte,
            Self::Dracula,
            Self::Nord,
            Self::TokyoNight,
            Self::Gruvbox,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::TokyoNight => "Tokyo Night",
            Self::Gruvbox => "Gruvbox",
        }
    }
}

// ============================================================================
// Theme Definitions
// ============================================================================

/// Catppuccin Mocha - Dark theme with warm pastels
pub const CATPPUCCIN_MOCHA: Theme = Theme {
    name: "Catppuccin Mocha",
    base: Color::Rgb(30, 30, 46),
    surface0: Color::Rgb(49, 50, 68),
    surface1: Color::Rgb(69, 71, 90),
    text: Color::Rgb(205, 214, 244),
    subtext0: Color::Rgb(166, 173, 200),
    blue: Color::Rgb(137, 180, 250),
    green: Color::Rgb(166, 227, 161),
    yellow: Color::Rgb(249, 226, 175),
    red: Color::Rgb(243, 139, 168),
    mauve: Color::Rgb(203, 166, 247),
    peach: Color::Rgb(250, 179, 135),
    teal: Color::Rgb(148, 226, 213),
};

/// Catppuccin Latte - Light theme with warm pastels
pub const CATPPUCCIN_LATTE: Theme = Theme {
    name: "Catppuccin Latte",
    base: Color::Rgb(239, 241, 245),
    surface0: Color::Rgb(220, 224, 232),
    surface1: Color::Rgb(188, 192, 204),
    text: Color::Rgb(76, 79, 105),
    subtext0: Color::Rgb(108, 111, 133),
    blue: Color::Rgb(30, 102, 245),
    green: Color::Rgb(64, 160, 43),
    yellow: Color::Rgb(223, 142, 29),
    red: Color::Rgb(210, 15, 57),
    mauve: Color::Rgb(136, 57, 239),
    peach: Color::Rgb(254, 100, 11),
    teal: Color::Rgb(23, 146, 153),
};

/// Dracula - Dark theme with vibrant colors
pub const DRACULA: Theme = Theme {
    name: "Dracula",
    base: Color::Rgb(40, 42, 54),
    surface0: Color::Rgb(68, 71, 90),
    surface1: Color::Rgb(98, 114, 164),
    text: Color::Rgb(248, 248, 242),
    subtext0: Color::Rgb(189, 147, 249),
    blue: Color::Rgb(139, 233, 253),
    green: Color::Rgb(80, 250, 123),
    yellow: Color::Rgb(241, 250, 140),
    red: Color::Rgb(255, 85, 85),
    mauve: Color::Rgb(189, 147, 249),
    peach: Color::Rgb(255, 184, 108),
    teal: Color::Rgb(139, 233, 253),
};

/// Nord - Arctic, bluish color palette
pub const NORD: Theme = Theme {
    name: "Nord",
    base: Color::Rgb(46, 52, 64),
    surface0: Color::Rgb(59, 66, 82),
    surface1: Color::Rgb(76, 86, 106),
    text: Color::Rgb(236, 239, 244),
    subtext0: Color::Rgb(216, 222, 233),
    blue: Color::Rgb(136, 192, 208),
    green: Color::Rgb(163, 190, 140),
    yellow: Color::Rgb(235, 203, 139),
    red: Color::Rgb(191, 97, 106),
    mauve: Color::Rgb(180, 142, 173),
    peach: Color::Rgb(208, 135, 112),
    teal: Color::Rgb(143, 188, 187),
};

/// Tokyo Night - Dark theme inspired by Tokyo's night
pub const TOKYO_NIGHT: Theme = Theme {
    name: "Tokyo Night",
    base: Color::Rgb(26, 27, 38),
    surface0: Color::Rgb(36, 40, 59),
    surface1: Color::Rgb(65, 72, 104),
    text: Color::Rgb(192, 202, 245),
    subtext0: Color::Rgb(139, 147, 175),
    blue: Color::Rgb(122, 162, 247),
    green: Color::Rgb(158, 206, 106),
    yellow: Color::Rgb(224, 175, 104),
    red: Color::Rgb(247, 118, 142),
    mauve: Color::Rgb(187, 154, 247),
    peach: Color::Rgb(255, 158, 100),
    teal: Color::Rgb(115, 218, 202),
};

/// Gruvbox - Retro groove color scheme
pub const GRUVBOX: Theme = Theme {
    name: "Gruvbox",
    base: Color::Rgb(40, 40, 40),
    surface0: Color::Rgb(60, 56, 54),
    surface1: Color::Rgb(80, 73, 69),
    text: Color::Rgb(235, 219, 178),
    subtext0: Color::Rgb(189, 174, 147),
    blue: Color::Rgb(131, 165, 152),
    green: Color::Rgb(184, 187, 38),
    yellow: Color::Rgb(250, 189, 47),
    red: Color::Rgb(251, 73, 52),
    mauve: Color::Rgb(211, 134, 155),
    peach: Color::Rgb(254, 128, 25),
    teal: Color::Rgb(142, 192, 124),
};
