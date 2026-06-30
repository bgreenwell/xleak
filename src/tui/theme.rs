use crate::workbook::CellValue;
use ratatui::style::Color;

/// Available themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Default,
    Dracula,
    SolarizedDark,
    SolarizedLight,
    GitHubDark,
    Nord,
}

impl Theme {
    /// Get all available themes
    pub fn all() -> &'static [Theme] {
        &[
            Theme::Default,
            Theme::Dracula,
            Theme::SolarizedDark,
            Theme::SolarizedLight,
            Theme::GitHubDark,
            Theme::Nord,
        ]
    }

    /// Get the next theme in the cycle
    pub fn next(&self) -> Theme {
        let themes = Self::all();
        let current_idx = themes.iter().position(|t| t == self).unwrap_or(0);
        themes[(current_idx + 1) % themes.len()]
    }

    /// Get theme name for display
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Default => "Default",
            Theme::Dracula => "Dracula",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::SolarizedLight => "Solarized Light",
            Theme::GitHubDark => "GitHub Dark",
            Theme::Nord => "Nord",
        }
    }

    /// Get the color scheme for this theme
    pub fn colors(&self) -> ColorScheme {
        match self {
            Theme::Default => ColorScheme::default_theme(),
            Theme::Dracula => ColorScheme::dracula(),
            Theme::SolarizedDark => ColorScheme::solarized_dark(),
            Theme::SolarizedLight => ColorScheme::solarized_light(),
            Theme::GitHubDark => ColorScheme::github_dark(),
            Theme::Nord => ColorScheme::nord(),
        }
    }
}

/// Color scheme for the TUI
#[derive(Debug, Clone)]
pub struct ColorScheme {
    // Cell type colors
    pub string_fg: Color,
    pub number_fg: Color,
    pub bool_fg: Color,
    pub datetime_fg: Color,
    pub error_fg: Color,
    pub empty_fg: Color,

    // UI element colors
    pub header_fg: Color,
    pub header_bg: Option<Color>,
    pub current_cell_fg: Color,
    pub current_cell_bg: Color,
    pub current_row_bg: Color,
    pub current_col_fg: Color,
    pub alternating_row_bg: Option<Color>,

    // Search colors
    pub search_match_fg: Color,
    pub search_match_bg: Color,
    pub current_search_fg: Color,
    pub current_search_bg: Color,

    // Border and status bar
    pub border_fg: Color,
    pub status_bar_fg: Color,
    pub status_bar_bg: Option<Color>,
}

impl ColorScheme {
    /// Default theme (current behavior with enhancements)
    pub fn default_theme() -> Self {
        Self {
            // Cell types
            string_fg: Color::White,
            number_fg: Color::Cyan,
            bool_fg: Color::Magenta,
            datetime_fg: Color::Green,
            error_fg: Color::Red,
            empty_fg: Color::DarkGray,

            // UI elements
            header_fg: Color::Yellow,
            header_bg: None,
            current_cell_fg: Color::White,
            current_cell_bg: Color::Blue,
            current_row_bg: Color::DarkGray,
            current_col_fg: Color::Cyan,
            alternating_row_bg: Some(Color::Rgb(25, 25, 28)),

            // Search
            search_match_fg: Color::Black,
            search_match_bg: Color::LightYellow,
            current_search_fg: Color::Black,
            current_search_bg: Color::Yellow,

            // Borders/status
            border_fg: Color::White,
            status_bar_fg: Color::White,
            status_bar_bg: None,
        }
    }

    /// Dracula theme (purple/pink aesthetic)
    pub fn dracula() -> Self {
        Self {
            // Cell types - Dracula palette
            string_fg: Color::Rgb(248, 248, 242),  // Foreground
            number_fg: Color::Rgb(189, 147, 249),  // Purple
            bool_fg: Color::Rgb(255, 121, 198),    // Pink
            datetime_fg: Color::Rgb(80, 250, 123), // Green
            error_fg: Color::Rgb(255, 85, 85),     // Red
            empty_fg: Color::Rgb(98, 114, 164),    // Comment

            // UI elements
            header_fg: Color::Rgb(139, 233, 253),    // Cyan
            header_bg: Some(Color::Rgb(68, 71, 90)), // Current line
            current_cell_fg: Color::Rgb(248, 248, 242),
            current_cell_bg: Color::Rgb(98, 114, 164), // Comment (darker)
            current_row_bg: Color::Rgb(68, 71, 90),    // Current line
            current_col_fg: Color::Rgb(139, 233, 253), // Cyan
            alternating_row_bg: Some(Color::Rgb(50, 52, 65)),

            // Search
            search_match_fg: Color::Rgb(40, 42, 54), // Background
            search_match_bg: Color::Rgb(241, 250, 140), // Yellow
            current_search_fg: Color::Rgb(40, 42, 54),
            current_search_bg: Color::Rgb(255, 184, 108), // Orange

            // Borders/status
            border_fg: Color::Rgb(98, 114, 164), // Comment
            status_bar_fg: Color::Rgb(248, 248, 242),
            status_bar_bg: Some(Color::Rgb(68, 71, 90)),
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            // Cell types - Solarized Dark
            string_fg: Color::Rgb(131, 148, 150), // Base0
            number_fg: Color::Rgb(38, 139, 210),  // Blue
            bool_fg: Color::Rgb(211, 54, 130),    // Magenta
            datetime_fg: Color::Rgb(133, 153, 0), // Green
            error_fg: Color::Rgb(220, 50, 47),    // Red
            empty_fg: Color::Rgb(88, 110, 117),   // Base01

            // UI elements
            header_fg: Color::Rgb(181, 137, 0),     // Yellow
            header_bg: Some(Color::Rgb(7, 54, 66)), // Base02
            current_cell_fg: Color::Rgb(253, 246, 227),
            current_cell_bg: Color::Rgb(88, 110, 117), // Base01
            current_row_bg: Color::Rgb(7, 54, 66),     // Base02
            current_col_fg: Color::Rgb(42, 161, 152),  // Cyan
            alternating_row_bg: Some(Color::Rgb(0, 43, 54)),

            // Search
            search_match_fg: Color::Rgb(0, 43, 54),
            search_match_bg: Color::Rgb(181, 137, 0), // Yellow
            current_search_fg: Color::Rgb(0, 43, 54),
            current_search_bg: Color::Rgb(203, 75, 22), // Orange

            // Borders/status
            border_fg: Color::Rgb(88, 110, 117),
            status_bar_fg: Color::Rgb(131, 148, 150),
            status_bar_bg: Some(Color::Rgb(7, 54, 66)),
        }
    }

    /// Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            // Cell types - Solarized Light
            string_fg: Color::Rgb(101, 123, 131), // Base00
            number_fg: Color::Rgb(38, 139, 210),  // Blue
            bool_fg: Color::Rgb(211, 54, 130),    // Magenta
            datetime_fg: Color::Rgb(133, 153, 0), // Green
            error_fg: Color::Rgb(220, 50, 47),    // Red
            empty_fg: Color::Rgb(147, 161, 161),  // Base1

            // UI elements
            header_fg: Color::Rgb(181, 137, 0),         // Yellow
            header_bg: Some(Color::Rgb(238, 232, 213)), // Base2
            current_cell_fg: Color::Rgb(0, 43, 54),     // Base02
            current_cell_bg: Color::Rgb(147, 161, 161), // Base1
            current_row_bg: Color::Rgb(238, 232, 213),  // Base2
            current_col_fg: Color::Rgb(42, 161, 152),   // Cyan
            alternating_row_bg: Some(Color::Rgb(253, 246, 227)),

            // Search
            search_match_fg: Color::Rgb(0, 43, 54),
            search_match_bg: Color::Rgb(181, 137, 0), // Yellow
            current_search_fg: Color::Rgb(253, 246, 227),
            current_search_bg: Color::Rgb(203, 75, 22), // Orange

            // Borders/status
            border_fg: Color::Rgb(147, 161, 161),
            status_bar_fg: Color::Rgb(101, 123, 131),
            status_bar_bg: Some(Color::Rgb(238, 232, 213)),
        }
    }

    /// GitHub Dark theme
    pub fn github_dark() -> Self {
        Self {
            // Cell types - GitHub Dark
            string_fg: Color::Rgb(201, 209, 217),   // fgDefault
            number_fg: Color::Rgb(121, 192, 255),   // prettylights-syntax-constant
            bool_fg: Color::Rgb(255, 125, 163),     // prettylights-syntax-entity
            datetime_fg: Color::Rgb(127, 219, 202), // prettylights-syntax-string
            error_fg: Color::Rgb(248, 81, 73),      // danger-fg
            empty_fg: Color::Rgb(110, 118, 129),    // fgMuted

            // UI elements
            header_fg: Color::Rgb(255, 199, 119), // prettylights-syntax-entity-tag
            header_bg: Some(Color::Rgb(33, 38, 45)), // canvas-subtle
            current_cell_fg: Color::Rgb(201, 209, 217),
            current_cell_bg: Color::Rgb(56, 139, 253), // accent-emphasis
            current_row_bg: Color::Rgb(33, 38, 45),    // canvas-subtle
            current_col_fg: Color::Rgb(121, 192, 255),
            alternating_row_bg: Some(Color::Rgb(22, 27, 34)),

            // Search
            search_match_fg: Color::Rgb(13, 17, 23),
            search_match_bg: Color::Rgb(187, 128, 9), // attention-emphasis
            current_search_fg: Color::Rgb(13, 17, 23),
            current_search_bg: Color::Rgb(242, 130, 33), // severe-emphasis

            // Borders/status
            border_fg: Color::Rgb(48, 54, 61), // border-default
            status_bar_fg: Color::Rgb(201, 209, 217),
            status_bar_bg: Some(Color::Rgb(33, 38, 45)),
        }
    }

    /// Nord theme (cool blue/cyan palette)
    pub fn nord() -> Self {
        Self {
            // Cell types - Nord
            string_fg: Color::Rgb(216, 222, 233),   // nord4
            number_fg: Color::Rgb(136, 192, 208),   // nord8
            bool_fg: Color::Rgb(180, 142, 173),     // nord15
            datetime_fg: Color::Rgb(163, 190, 140), // nord14
            error_fg: Color::Rgb(191, 97, 106),     // nord11
            empty_fg: Color::Rgb(76, 86, 106),      // nord3

            // UI elements
            header_fg: Color::Rgb(235, 203, 139),    // nord13
            header_bg: Some(Color::Rgb(59, 66, 82)), // nord1
            current_cell_fg: Color::Rgb(236, 239, 244),
            current_cell_bg: Color::Rgb(94, 129, 172), // nord9
            current_row_bg: Color::Rgb(59, 66, 82),    // nord1
            current_col_fg: Color::Rgb(136, 192, 208), // nord8
            alternating_row_bg: Some(Color::Rgb(46, 52, 64)),

            // Search
            search_match_fg: Color::Rgb(46, 52, 64),
            search_match_bg: Color::Rgb(235, 203, 139), // nord13
            current_search_fg: Color::Rgb(46, 52, 64),
            current_search_bg: Color::Rgb(208, 135, 112), // nord12

            // Borders/status
            border_fg: Color::Rgb(76, 86, 106), // nord3
            status_bar_fg: Color::Rgb(216, 222, 233),
            status_bar_bg: Some(Color::Rgb(59, 66, 82)),
        }
    }

    /// Get foreground color for a cell based on its value type
    pub fn cell_color(&self, cell: &CellValue) -> Color {
        match cell {
            CellValue::Empty => self.empty_fg,
            CellValue::String(_) => self.string_fg,
            CellValue::Int(_) | CellValue::Float(_) => self.number_fg,
            CellValue::Bool(_) => self.bool_fg,
            CellValue::Error(_) => self.error_fg,
            CellValue::DateTime(_) => self.datetime_fg,
        }
    }
}
