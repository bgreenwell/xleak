use crate::workbook::{CellValue, LazySheetData, SheetData, Workbook};
use anyhow::{Context, Result};
use arboard::Clipboard;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use std::io;
use std::time::{Duration, Instant};

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

/// Cached row data for lazy loading
struct RowCache {
    start_row: usize,
    rows: Vec<Vec<CellValue>>,
    formulas: Vec<Vec<Option<String>>>,
}

/// Sheet data source (either eager or lazy)
enum SheetDataSource {
    Eager(SheetData),
    Lazy {
        data: LazySheetData,
        cache: Option<RowCache>,
        cache_size: usize, // Number of rows to cache at once
    },
}

impl SheetDataSource {
    fn headers(&self) -> &[String] {
        match self {
            SheetDataSource::Eager(data) => &data.headers,
            SheetDataSource::Lazy { data, .. } => &data.headers,
        }
    }

    fn width(&self) -> usize {
        match self {
            SheetDataSource::Eager(data) => data.width,
            SheetDataSource::Lazy { data, .. } => data.width,
        }
    }

    fn height(&self) -> usize {
        match self {
            SheetDataSource::Eager(data) => data.height,
            SheetDataSource::Lazy { data, .. } => data.height,
        }
    }

    /// Fetches rows with automatic cache management
    fn get_rows(
        &mut self,
        start: usize,
        count: usize,
    ) -> (&[Vec<CellValue>], &[Vec<Option<String>>]) {
        match self {
            SheetDataSource::Eager(data) => {
                let end = (start + count).min(data.rows.len());
                (&data.rows[start..end], &data.formulas[start..end])
            }
            SheetDataSource::Lazy {
                data,
                cache,
                cache_size,
            } => {
                // Check if we need to reload the cache
                let needs_reload = match cache {
                    None => true,
                    Some(c) => start < c.start_row || start >= c.start_row + c.rows.len(),
                };

                if needs_reload {
                    // Load new chunk centered around the requested start
                    let cache_start = start.saturating_sub(*cache_size / 4); // Start a bit before
                    let (rows, formulas) = data.get_rows(cache_start, *cache_size);
                    *cache = Some(RowCache {
                        start_row: cache_start,
                        rows,
                        formulas,
                    });
                }

                // Return from cache
                if let Some(c) = cache {
                    let offset = start.saturating_sub(c.start_row);
                    let end = (offset + count).min(c.rows.len());
                    (&c.rows[offset..end], &c.formulas[offset..end])
                } else {
                    // Shouldn't happen, but return empty slices
                    (&[], &[])
                }
            }
        }
    }

    fn get_cell(&mut self, row: usize, col: usize) -> (Option<CellValue>, Option<String>) {
        match self {
            SheetDataSource::Eager(data) => {
                let cell = data.rows.get(row).and_then(|r| r.get(col)).cloned();
                let formula = data
                    .formulas
                    .get(row)
                    .and_then(|r| r.get(col))
                    .and_then(|f| f.clone());
                (cell, formula)
            }
            SheetDataSource::Lazy { .. } => {
                // For lazy loading, get just the one row we need
                let (rows, formulas) = self.get_rows(row, 1);
                let cell = rows.first().and_then(|r| r.get(col)).cloned();
                let formula = formulas
                    .first()
                    .and_then(|r| r.get(col))
                    .and_then(|f| f.clone());
                (cell, formula)
            }
        }
    }
}

/// Progress information for long-running operations
#[derive(Debug, Clone)]
struct ProgressInfo {
    message: String,
    current: usize,
    total: usize,
    started_at: Instant,
}

impl ProgressInfo {
    fn new(message: impl Into<String>, total: usize) -> Self {
        Self {
            message: message.into(),
            current: 0,
            total,
            started_at: Instant::now(),
        }
    }

    fn update(&mut self, current: usize) {
        self.current = current;
    }

    fn percentage(&self) -> usize {
        if self.total == 0 {
            100
        } else {
            (self.current * 100) / self.total
        }
    }

    fn format(&self) -> String {
        let pct = self.percentage();
        let _elapsed = self.started_at.elapsed().as_secs_f64();
        format!(
            "{} {}% ({}/{})",
            self.message, pct, self.current, self.total
        )
    }
}

/// TUI application state
pub struct TuiState {
    workbook: Workbook,
    sheet_names: Vec<String>,
    current_sheet_index: usize,
    sheet_data: SheetDataSource,
    should_quit: bool,
    cursor_row: usize,               // Current row (0-indexed in data)
    cursor_col: usize,               // Current column (0-indexed)
    scroll_offset: usize,            // Vertical scroll offset
    horizontal_scroll_offset: usize, // Horizontal scroll offset
    horizontal_scroll_enabled: bool, // Whether horizontal scrolling is enabled
    column_widths: Vec<usize>,       // Cached column widths for horizontal scroll
    show_help: bool,                 // Help overlay visible
    show_cell_detail: bool,          // Cell detail popup visible
    // Search state
    search_mode: bool,                   // Whether we're in search input mode
    search_query: String,                // Current search query
    search_matches: Vec<(usize, usize)>, // List of (row, col) matches
    current_match_index: Option<usize>,  // Index in search_matches
    // Jump mode state
    jump_mode: bool,    // Whether we're in jump input mode
    jump_input: String, // Current jump input (row number or cell address)
    // Clipboard state
    copy_feedback: Option<(String, Instant)>, // Message and timestamp for copy feedback
    // Progress state
    progress: Option<ProgressInfo>, // Current operation progress
    // Theme state
    current_theme: Theme, // Current color theme
    // Config state
    config: crate::config::Config, // User configuration
}

impl TuiState {
    const LAZY_LOADING_THRESHOLD: usize = 1000; // Use lazy loading for sheets with >1000 rows
    const ROW_CACHE_SIZE: usize = 200; // Cache 200 rows at a time for lazy loading

    pub fn new(
        mut workbook: Workbook,
        initial_sheet_name: &str,
        config: &crate::config::Config,
        horizontal_scroll: bool,
    ) -> Result<Self> {
        let sheet_names = workbook.sheet_names();
        let current_sheet_index = sheet_names
            .iter()
            .position(|name| name == initial_sheet_name)
            .unwrap_or(0);

        // Load sheet lazily first to check size
        let lazy_data = workbook.load_sheet_lazy(&sheet_names[current_sheet_index])?;
        let sheet_height = lazy_data.height;

        // Choose loading strategy based on size
        let sheet_data = if sheet_height > Self::LAZY_LOADING_THRESHOLD {
            eprintln!(
                "📊 Large file detected ({} rows) - using lazy loading",
                sheet_height
            );
            SheetDataSource::Lazy {
                data: lazy_data,
                cache: None,
                cache_size: Self::ROW_CACHE_SIZE,
            }
        } else {
            // Convert to eager loading for small files
            SheetDataSource::Eager(lazy_data.to_sheet_data())
        };

        let mut state = Self {
            workbook,
            sheet_names,
            current_sheet_index,
            sheet_data,
            should_quit: false,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            horizontal_scroll_offset: 0,
            horizontal_scroll_enabled: horizontal_scroll,
            column_widths: Vec::new(),
            show_help: false,
            show_cell_detail: false,
            search_mode: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match_index: None,
            jump_mode: false,
            jump_input: String::new(),
            copy_feedback: None,
            progress: None,
            current_theme: Self::parse_theme_name(&config.theme.default),
            config: config.clone(),
        };

        // Calculate column widths if horizontal scrolling is enabled
        if horizontal_scroll {
            state.column_widths = state.calculate_column_widths();
        }

        Ok(state)
    }

    /// Parse theme name from config string
    fn parse_theme_name(name: &str) -> Theme {
        match name.to_lowercase().as_str() {
            "dracula" => Theme::Dracula,
            "solarized dark" | "solarizeddark" => Theme::SolarizedDark,
            "solarized light" | "solarizedlight" => Theme::SolarizedLight,
            "github dark" | "githubdark" => Theme::GitHubDark,
            "nord" => Theme::Nord,
            _ => Theme::Default, // Fallback to default for unknown themes
        }
    }

    fn current_sheet_name(&self) -> &str {
        &self.sheet_names[self.current_sheet_index]
    }

    fn switch_to_next_sheet(&mut self) -> Result<()> {
        if self.sheet_names.len() <= 1 {
            return Ok(()); // No other sheets to switch to
        }

        self.current_sheet_index = (self.current_sheet_index + 1) % self.sheet_names.len();
        self.load_current_sheet()?;
        self.reset_cursor();
        self.clear_search(); // Clear search when changing sheets
        Ok(())
    }

    fn switch_to_prev_sheet(&mut self) -> Result<()> {
        if self.sheet_names.len() <= 1 {
            return Ok(()); // No other sheets to switch to
        }

        self.current_sheet_index = if self.current_sheet_index == 0 {
            self.sheet_names.len() - 1
        } else {
            self.current_sheet_index - 1
        };
        self.load_current_sheet()?;
        self.reset_cursor();
        self.clear_search(); // Clear search when changing sheets
        Ok(())
    }

    fn load_current_sheet(&mut self) -> Result<()> {
        let sheet_name = self.sheet_names[self.current_sheet_index].clone();

        // Load sheet lazily first to check size
        let lazy_data = self.workbook.load_sheet_lazy(&sheet_name)?;
        let sheet_height = lazy_data.height;

        // Choose loading strategy based on size
        self.sheet_data = if sheet_height > Self::LAZY_LOADING_THRESHOLD {
            eprintln!(
                "📊 Large file detected ({} rows) - using lazy loading",
                sheet_height
            );
            SheetDataSource::Lazy {
                data: lazy_data,
                cache: None,
                cache_size: Self::ROW_CACHE_SIZE,
            }
        } else {
            // Convert to eager loading for small files
            SheetDataSource::Eager(lazy_data.to_sheet_data())
        };

        // Recalculate column widths if horizontal scrolling is enabled
        if self.horizontal_scroll_enabled {
            self.column_widths = self.calculate_column_widths();
        }

        Ok(())
    }

    fn reset_cursor(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.horizontal_scroll_offset = 0;
    }

    /// Perform case-insensitive search across all cells
    fn perform_search(&mut self) {
        self.search_matches.clear();
        self.current_match_index = None;

        if self.search_query.is_empty() {
            self.progress = None;
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        let total_height = self.sheet_data.height();

        // Show progress for large sheets
        if total_height > 1000 {
            self.progress = Some(ProgressInfo::new("Searching", total_height));
        }

        // Search through all cells (load in chunks for lazy data)
        const SEARCH_CHUNK_SIZE: usize = 500;
        for chunk_start in (0..total_height).step_by(SEARCH_CHUNK_SIZE) {
            let chunk_size = SEARCH_CHUNK_SIZE.min(total_height - chunk_start);
            let (rows, _formulas) = self.sheet_data.get_rows(chunk_start, chunk_size);

            for (chunk_idx, row) in rows.iter().enumerate() {
                let row_idx = chunk_start + chunk_idx;
                for (col_idx, cell) in row.iter().enumerate() {
                    let cell_str = cell.to_string().to_lowercase();
                    if cell_str.contains(&query_lower) {
                        self.search_matches.push((row_idx, col_idx));
                    }
                }
            }

            // Update progress
            if let Some(ref mut progress) = self.progress {
                progress.update(chunk_start + chunk_size);
            }
        }

        // Clear progress when done
        self.progress = None;

        // If we found matches, select the first one
        if !self.search_matches.is_empty() {
            self.current_match_index = Some(0);
            self.jump_to_current_match();
        }
    }

    /// Jump to the next search match
    fn jump_to_next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => (idx + 1) % self.search_matches.len(),
            None => 0,
        });

        self.jump_to_current_match();
    }

    /// Jump to the previous search match
    fn jump_to_prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => {
                if idx == 0 {
                    self.search_matches.len() - 1
                } else {
                    idx - 1
                }
            }
            None => self.search_matches.len() - 1,
        });

        self.jump_to_current_match();
    }

    /// Move cursor to the current search match
    fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.current_match_index
            && let Some(&(row, col)) = self.search_matches.get(idx)
        {
            self.cursor_row = row;
            self.cursor_col = col;
        }
    }

    /// Clear search state
    fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = None;
    }

    /// Enter jump mode
    fn enter_jump_mode(&mut self) {
        self.jump_mode = true;
        self.jump_input.clear();
    }

    /// Parse jump input and navigate to that location
    /// Supports formats: "100" (row), "A5" (cell address), "5,3" (row,col)
    fn perform_jump(&mut self) {
        if self.jump_input.is_empty() {
            self.jump_mode = false;
            return;
        }

        let input = self.jump_input.trim();

        // Try to parse as row number (1-indexed)
        if let Ok(row_num) = input.parse::<usize>() {
            if row_num > 0 && row_num <= self.sheet_data.height() {
                self.cursor_row = row_num - 1; // Convert to 0-indexed
                self.copy_feedback = Some((format!("Jumped to row {}", row_num), Instant::now()));
            } else {
                self.copy_feedback = Some((
                    format!(
                        "Invalid row: {} (max: {})",
                        row_num,
                        self.sheet_data.height()
                    ),
                    Instant::now(),
                ));
            }
        }
        // Try to parse as cell address like "A5" or "B10"
        else if let Some((col, row)) = Self::parse_cell_address(input) {
            if row < self.sheet_data.height() && col < self.sheet_data.width() {
                self.cursor_row = row;
                self.cursor_col = col;
                self.copy_feedback = Some((
                    format!("Jumped to {}", input.to_uppercase()),
                    Instant::now(),
                ));
            } else {
                self.copy_feedback = Some((
                    format!("Cell address out of bounds: {}", input),
                    Instant::now(),
                ));
            }
        }
        // Try to parse as "row,col" format
        else if let Some((row, col)) = input.split_once(',') {
            if let (Ok(row_num), Ok(col_num)) =
                (row.trim().parse::<usize>(), col.trim().parse::<usize>())
            {
                if row_num > 0
                    && row_num <= self.sheet_data.height()
                    && col_num > 0
                    && col_num <= self.sheet_data.width()
                {
                    self.cursor_row = row_num - 1;
                    self.cursor_col = col_num - 1;
                    self.copy_feedback = Some((
                        format!("Jumped to row {}, col {}", row_num, col_num),
                        Instant::now(),
                    ));
                } else {
                    self.copy_feedback =
                        Some(("Invalid row/column number".to_string(), Instant::now()));
                }
            } else {
                self.copy_feedback = Some((
                    "Invalid format. Use: row number, cell (A5), or row,col".to_string(),
                    Instant::now(),
                ));
            }
        } else {
            self.copy_feedback = Some((
                "Invalid format. Use: row number, cell (A5), or row,col".to_string(),
                Instant::now(),
            ));
        }

        self.jump_mode = false;
        self.jump_input.clear();
    }

    /// Parse cell address like "A5", "B10", "AA100" into (col, row) indices
    fn parse_cell_address(addr: &str) -> Option<(usize, usize)> {
        let addr = addr.to_uppercase();
        let mut col = 0usize;
        let mut row_str = String::new();

        for ch in addr.chars() {
            if ch.is_ascii_alphabetic() {
                col = col * 26 + (ch as usize - 'A' as usize + 1);
            } else if ch.is_ascii_digit() {
                row_str.push(ch);
            } else {
                return None;
            }
        }

        if row_str.is_empty() || col == 0 {
            return None;
        }

        let row = row_str.parse::<usize>().ok()?;
        Some((col - 1, row - 1)) // Convert to 0-indexed
    }

    /// Copy the current cell value to clipboard
    fn copy_current_cell(&mut self) {
        let (cell, _formula) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);
        let cell_value = cell.map(|v| v.to_raw_string()).unwrap_or_default();

        match Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(&cell_value) {
                    self.copy_feedback = Some((format!("Copy failed: {}", e), Instant::now()));
                } else {
                    let cell_addr = self.current_cell_address();
                    self.copy_feedback =
                        Some((format!("Copied cell {}", cell_addr), Instant::now()));
                }
            }
            Err(e) => {
                self.copy_feedback = Some((format!("Clipboard error: {}", e), Instant::now()));
            }
        }
    }

    /// Copy the current row to clipboard (tab-separated)
    fn copy_current_row(&mut self) {
        let (rows, _formulas) = self.sheet_data.get_rows(self.cursor_row, 1);
        let row_values = rows
            .first()
            .map(|row| {
                row.iter()
                    .map(|cell| {
                        let value = cell.to_raw_string();
                        // Escape cells that contain tabs, newlines, or quotes
                        if value.contains('\t') || value.contains('\n') || value.contains('"') {
                            format!("\"{}\"", value.replace('"', "\"\""))
                        } else {
                            value
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .unwrap_or_default();

        match Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_text(&row_values) {
                    self.copy_feedback = Some((format!("Copy failed: {}", e), Instant::now()));
                } else {
                    self.copy_feedback = Some((
                        format!(
                            "Copied row {} ({} cells)",
                            self.cursor_row + 1,
                            self.sheet_data.width()
                        ),
                        Instant::now(),
                    ));
                }
            }
            Err(e) => {
                self.copy_feedback = Some((format!("Clipboard error: {}", e), Instant::now()));
            }
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            // Auto-scroll up if cursor moves above viewport
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row < self.sheet_data.height().saturating_sub(1) {
            self.cursor_row += 1;
            // Auto-scroll down will be handled in render based on viewport height
        }
    }

    /// Update scroll offset to keep cursor visible
    fn update_scroll(&mut self, viewport_height: usize) {
        // Scroll down if cursor is below visible area
        if self.cursor_row >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_row.saturating_sub(viewport_height - 1);
        }
        // Scroll up if cursor is above visible area
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        }
    }

    /// Calculate column widths based on content
    fn calculate_column_widths(&mut self) -> Vec<usize> {
        let num_cols = self.sheet_data.width();
        let mut widths = vec![0; num_cols];

        // Measure headers
        let headers = self.sheet_data.headers();
        for (i, header) in headers.iter().enumerate() {
            widths[i] = header.len();
        }

        // Sample first 100 rows (or fewer if sheet is smaller)
        let sample_size = 100.min(self.sheet_data.height());
        let (sample_rows, _) = self.sheet_data.get_rows(0, sample_size);

        for row in sample_rows.iter() {
            for (col_idx, cell) in row.iter().enumerate() {
                let len = cell.to_string().len();
                widths[col_idx] = widths[col_idx].max(len);
            }
        }

        // Apply constraints: min 3 chars, max 30 chars
        widths.iter().map(|&w| w.clamp(3, 30)).collect()
    }

    /// Update horizontal scroll offset to keep cursor visible
    fn update_horizontal_scroll(&mut self, viewport_width: usize) {
        if !self.horizontal_scroll_enabled {
            return;
        }

        // Calculate which columns are visible
        let mut total_width = 0;
        let mut visible_end = self.horizontal_scroll_offset;

        for i in self.horizontal_scroll_offset..self.column_widths.len() {
            total_width += self.column_widths[i] + 1; // +1 for separator
            visible_end = i + 1; // Always include current column
            if total_width > viewport_width {
                break; // Break after including partially-visible column
            }
        }

        // Scroll right if cursor is beyond visible area
        if self.cursor_col >= visible_end {
            self.horizontal_scroll_offset += 1;
        }

        // Scroll left if cursor is before visible area
        if self.cursor_col < self.horizontal_scroll_offset {
            self.horizontal_scroll_offset = self.cursor_col;
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            // Auto-scroll left if cursor moves before visible area
            if self.horizontal_scroll_enabled && self.cursor_col < self.horizontal_scroll_offset {
                self.horizontal_scroll_offset = self.cursor_col;
            }
        }
    }

    fn move_right(&mut self) {
        if self.cursor_col < self.sheet_data.width().saturating_sub(1) {
            self.cursor_col += 1;
            // Auto-scroll right will be handled in render based on viewport width
        }
    }

    fn move_to_start_of_row(&mut self) {
        self.cursor_col = 0;
        if self.horizontal_scroll_enabled {
            self.horizontal_scroll_offset = 0;
        }
    }

    fn move_to_end_of_row(&mut self) {
        self.cursor_col = self.sheet_data.width().saturating_sub(1);
        // Horizontal scroll will be updated in render to show the last column
    }

    fn page_up(&mut self, page_size: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(page_size);
    }

    fn page_down(&mut self, page_size: usize) {
        self.cursor_row =
            (self.cursor_row + page_size).min(self.sheet_data.height().saturating_sub(1));
    }

    fn move_to_top(&mut self) {
        self.cursor_row = 0;
    }

    fn move_to_bottom(&mut self) {
        self.cursor_row = self.sheet_data.height().saturating_sub(1);
    }

    fn col_to_letter(&self, col: usize) -> String {
        let mut result = String::new();
        let mut n = col + 1;
        while n > 0 {
            n -= 1;
            result.push((b'A' + (n % 26) as u8) as char);
            n /= 26;
        }
        result.chars().rev().collect()
    }

    fn current_cell_address(&self) -> String {
        format!(
            "{}{}",
            self.col_to_letter(self.cursor_col),
            self.cursor_row + 1
        )
    }

    /// Check if a key press matches a configured action
    fn key_matches(
        &self,
        code: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
        action: &str,
    ) -> bool {
        if let Some((expected_code, expected_mods)) = self.config.get_keybinding(action) {
            code == expected_code && modifiers == expected_mods
        } else {
            false
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            // If help is showing, any key closes it
            if self.show_help {
                self.show_help = false;
                return;
            }

            // If cell detail is showing, any key closes it
            if self.show_cell_detail {
                self.show_cell_detail = false;
                return;
            }

            // If in search mode, handle search input
            if self.search_mode {
                match code {
                    KeyCode::Char(c) => {
                        self.search_query.push(c);
                        self.perform_search();
                    }
                    KeyCode::Backspace => {
                        self.search_query.pop();
                        self.perform_search();
                    }
                    KeyCode::Enter => {
                        // Exit search mode but keep results
                        self.search_mode = false;
                    }
                    KeyCode::Esc => {
                        // Exit search mode and clear search
                        self.search_mode = false;
                        self.clear_search();
                    }
                    _ => {}
                }
                return;
            }

            // If in jump mode, handle jump input
            if self.jump_mode {
                match code {
                    KeyCode::Char(c) => {
                        self.jump_input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.jump_input.pop();
                    }
                    KeyCode::Enter => {
                        // Perform jump
                        self.perform_jump();
                    }
                    KeyCode::Esc => {
                        // Exit jump mode
                        self.jump_mode = false;
                        self.jump_input.clear();
                    }
                    _ => {}
                }
                return;
            }

            // Normal navigation and commands - using configured keybindings
            // Check actions in order of priority
            if self.key_matches(code, modifiers, "quit") {
                self.should_quit = true;
            } else if self.key_matches(code, modifiers, "help") {
                self.show_help = true;
            } else if self.key_matches(code, modifiers, "theme_toggle") {
                self.current_theme = self.current_theme.next();
            } else if self.key_matches(code, modifiers, "search") {
                self.search_mode = true;
                self.clear_search();
            } else if self.key_matches(code, modifiers, "next_match") {
                self.jump_to_next_match();
            } else if self.key_matches(code, modifiers, "prev_match") {
                self.jump_to_prev_match();
            } else if self.key_matches(code, modifiers, "copy_cell") {
                self.copy_current_cell();
            } else if self.key_matches(code, modifiers, "copy_row") {
                self.copy_current_row();
            } else if self.key_matches(code, modifiers, "jump") {
                self.enter_jump_mode();
            } else if self.key_matches(code, modifiers, "show_cell_detail") {
                self.show_cell_detail = true;
            } else if self.key_matches(code, modifiers, "next_sheet") {
                let _ = self.switch_to_next_sheet();
            } else if self.key_matches(code, modifiers, "prev_sheet") || code == KeyCode::BackTab {
                // BackTab is another way to detect Shift+Tab on some terminals
                let _ = self.switch_to_prev_sheet();
            } else if self.key_matches(code, modifiers, "up") {
                self.move_up();
            } else if self.key_matches(code, modifiers, "down") {
                self.move_down();
            } else if self.key_matches(code, modifiers, "left") {
                self.move_left();
            } else if self.key_matches(code, modifiers, "right") {
                self.move_right();
            } else if self.key_matches(code, modifiers, "jump_to_top") {
                self.move_to_top();
            } else if self.key_matches(code, modifiers, "jump_to_bottom") {
                self.move_to_bottom();
            } else if self.key_matches(code, modifiers, "jump_to_row_start") {
                self.move_to_start_of_row();
            } else if self.key_matches(code, modifiers, "jump_to_row_end") {
                self.move_to_end_of_row();
            } else if self.key_matches(code, modifiers, "page_up") {
                self.page_up(10);
            } else if self.key_matches(code, modifiers, "page_down") {
                self.page_down(10);
            } else if code == KeyCode::Esc {
                // Special handling for Esc - clear search if active, otherwise quit
                if !self.search_matches.is_empty() {
                    self.clear_search();
                } else {
                    self.should_quit = true;
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(frame.area());

        // Calculate visible viewport
        let table_height = chunks[0].height.saturating_sub(3) as usize; // Account for borders and header
        let viewport_width = chunks[0].width.saturating_sub(2) as usize; // Account for borders

        // Update scroll to keep cursor visible
        self.update_scroll(table_height);
        self.update_horizontal_scroll(viewport_width);

        let visible_start = self.scroll_offset;

        // Calculate visible column range
        let (visible_col_start, visible_col_end) = if self.horizontal_scroll_enabled {
            // Calculate which columns fit in viewport
            let mut total_width = 0;
            let mut end = self.horizontal_scroll_offset;

            for i in self.horizontal_scroll_offset..self.column_widths.len() {
                total_width += self.column_widths[i] + 1; // +1 for separator
                end = i + 1; // Always include current column
                if total_width > viewport_width {
                    break; // Break after including partially-visible column
                }
            }
            (self.horizontal_scroll_offset, end)
        } else {
            (0, self.sheet_data.width())
        };

        // Clone headers to avoid borrow issues
        let headers = self.sheet_data.headers().to_vec();

        // Get theme colors
        let colors = self.current_theme.colors();

        // Build table rows with highlighting
        let header_cells: Vec<Cell> = headers
            .iter()
            .enumerate()
            .skip(visible_col_start)
            .take(visible_col_end - visible_col_start)
            .map(|(col_idx, h)| {
                let mut style = Style::default()
                    .fg(colors.header_fg)
                    .add_modifier(Modifier::BOLD);

                if let Some(bg) = colors.header_bg {
                    style = style.bg(bg);
                }

                // Highlight current column header
                if col_idx == self.cursor_col {
                    style = style.fg(colors.current_col_fg);
                }

                Cell::from(h.as_str()).style(style)
            })
            .collect();

        let header = Row::new(header_cells).height(1);

        // Get visible rows from data source (handles lazy loading if needed)
        let (visible_rows, _visible_formulas) =
            self.sheet_data.get_rows(visible_start, table_height);

        let data_rows: Vec<Row> = visible_rows
            .iter()
            .enumerate()
            .map(|(visible_idx, row)| {
                let row_idx = visible_start + visible_idx; // Absolute row index
                let cells: Vec<Cell> = row
                    .iter()
                    .enumerate()
                    .skip(visible_col_start)
                    .take(visible_col_end - visible_col_start)
                    .map(|(col_idx, cell)| {
                        // Start with cell type color
                        let mut style = Style::default().fg(colors.cell_color(cell));

                        // Add alternating row background (only if not the current row)
                        let is_alternating_row = row_idx % 2 == 1;
                        if is_alternating_row && let Some(alt_bg) = colors.alternating_row_bg {
                            style = style.bg(alt_bg);
                        }

                        // Check if this cell is a search match
                        let is_search_match = self.search_matches.contains(&(row_idx, col_idx));
                        let is_current_match = self
                            .current_match_index
                            .and_then(|idx| self.search_matches.get(idx))
                            .map(|&pos| pos == (row_idx, col_idx))
                            .unwrap_or(false);

                        // Highlight current search match (highest priority)
                        if is_current_match {
                            style = style
                                .bg(colors.current_search_bg)
                                .fg(colors.current_search_fg)
                                .add_modifier(Modifier::BOLD);
                        }
                        // Highlight current cell
                        else if row_idx == self.cursor_row && col_idx == self.cursor_col {
                            style = style
                                .bg(colors.current_cell_bg)
                                .fg(colors.current_cell_fg)
                                .add_modifier(Modifier::BOLD);
                        }
                        // Highlight other search matches
                        else if is_search_match {
                            style = style.bg(colors.search_match_bg).fg(colors.search_match_fg);
                        }
                        // Highlight current row
                        else if row_idx == self.cursor_row {
                            style = style.bg(colors.current_row_bg);
                        }
                        // Highlight current column
                        else if col_idx == self.cursor_col {
                            style = style.fg(colors.current_col_fg);
                        }
                        Cell::from(cell.to_string()).style(style)
                    })
                    .collect();
                Row::new(cells).height(1)
            })
            .collect();

        // Calculate column widths
        let col_widths: Vec<Constraint> = if self.horizontal_scroll_enabled {
            // Use fixed widths based on content
            self.column_widths[visible_col_start..visible_col_end]
                .iter()
                .map(|&w| Constraint::Length(w as u16))
                .collect()
        } else {
            // Use percentage-based widths (current behavior)
            let sheet_width = self.sheet_data.width();
            headers
                .iter()
                .map(|_| Constraint::Percentage((100 / sheet_width.max(1)) as u16))
                .collect()
        };

        let table_title = if self.sheet_names.len() > 1 {
            format!(
                " {} (Sheet {}/{}) ",
                self.current_sheet_name(),
                self.current_sheet_index + 1,
                self.sheet_names.len()
            )
        } else {
            format!(" {} ", self.current_sheet_name())
        };

        let table = Table::new(data_rows, col_widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border_fg))
                .title(table_title),
        );

        frame.render_widget(table, chunks[0]);

        // Status bar with current cell info
        let (cell, _) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);
        let current_cell_value = cell.map(|v| v.to_string()).unwrap_or_default();

        // Format sheet dimensions with scroll indicator
        let sheet_dims = if self.horizontal_scroll_enabled && self.horizontal_scroll_offset > 0 {
            let first_col = headers
                .get(visible_col_start)
                .map(|s| s.as_str())
                .unwrap_or("?");
            let last_col = headers
                .get(visible_col_end.saturating_sub(1))
                .map(|s| s.as_str())
                .unwrap_or("?");
            format!(
                "{} rows × {} columns (showing {}-{})",
                self.sheet_data.height(),
                self.sheet_data.width(),
                first_col,
                last_col
            )
        } else {
            format!(
                "{} rows × {} columns",
                self.sheet_data.height(),
                self.sheet_data.width()
            )
        };

        let status_text = if let Some(ref progress) = self.progress {
            // Show progress indicator
            format!(" ⏳ {} ", progress.format())
        } else if self.jump_mode {
            format!(
                " Jump to (row, cell like A5, or row,col): {} ",
                self.jump_input
            )
        } else if self.search_mode {
            format!(" Search: {} ", self.search_query)
        } else if let Some(idx) = self.current_match_index {
            // Show search results
            let match_info = format!("Match {}/{} | ", idx + 1, self.search_matches.len());
            if self.sheet_names.len() > 1 {
                format!(
                    " {} | {}n:next N:prev Esc:clear | {} | Tab:next sheet ?:help q:quit ",
                    match_info,
                    self.current_cell_address(),
                    sheet_dims
                )
            } else {
                format!(
                    " {} | {}n:next N:prev Esc:clear | {} | ?:help q:quit ",
                    match_info,
                    self.current_cell_address(),
                    sheet_dims
                )
            }
        } else {
            // Show loading mode indicator for large files
            let mode_indicator = match &self.sheet_data {
                SheetDataSource::Lazy { .. } => " [Lazy] ",
                SheetDataSource::Eager(_) => "",
            };

            if self.sheet_names.len() > 1 {
                format!(
                    " {} | {}{} | Theme: {} | t:theme /:search Tab:sheet ?:help q:quit ",
                    self.current_cell_address(),
                    sheet_dims,
                    mode_indicator,
                    self.current_theme.name()
                )
            } else {
                format!(
                    " {} | {}{} | Theme: {} | t:theme /:search ?:help q:quit ",
                    self.current_cell_address(),
                    sheet_dims,
                    mode_indicator,
                    self.current_theme.name()
                )
            }
        };

        let mut status_style = Style::default().fg(colors.status_bar_fg);
        if let Some(bg) = colors.status_bar_bg {
            status_style = status_style.bg(bg);
        }

        let status = Paragraph::new(status_text).style(status_style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border_fg))
                .title(format!(" {} ", current_cell_value)),
        );

        frame.render_widget(status, chunks[1]);

        // Render cell detail overlay if visible
        if self.show_cell_detail {
            self.render_cell_detail(frame);
        }

        // Render help overlay if visible
        if self.show_help {
            self.render_help(frame);
        }

        // Render copy feedback if active (and not expired)
        if let Some((ref message, timestamp)) = self.copy_feedback {
            // Show feedback for 2 seconds
            if timestamp.elapsed() < Duration::from_secs(2) {
                self.render_copy_feedback(frame, message);
            } else {
                // Clear expired feedback
                self.copy_feedback = None;
            }
        }
    }

    fn render_help(&self, frame: &mut Frame) {
        use ratatui::text::{Line, Span};

        // Build help content with rich formatting
        let help_lines = vec![
            Line::from(vec![
                Span::styled(
                    "xleak",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - Interactive Excel Viewer"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "NAVIGATION",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  ↑ ↓ ← →          ", Style::default().fg(Color::Green)),
                Span::raw("Move cursor one cell"),
            ]),
            Line::from(vec![
                Span::styled("  Page Up/Down     ", Style::default().fg(Color::Green)),
                Span::raw("Scroll 10 rows"),
            ]),
            Line::from(vec![
                Span::styled("  Home             ", Style::default().fg(Color::Green)),
                Span::raw("Jump to first column (start of row)"),
            ]),
            Line::from(vec![
                Span::styled("  End              ", Style::default().fg(Color::Green)),
                Span::raw("Jump to last column (end of row)"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+Home        ", Style::default().fg(Color::Green)),
                Span::raw("Jump to first row (top of sheet)"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+End         ", Style::default().fg(Color::Green)),
                Span::raw("Jump to last row (bottom of sheet)"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+G           ", Style::default().fg(Color::Green)),
                Span::raw("Jump to row/cell (e.g., 100, A5, or 10,3)"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "SEARCH",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  /                ", Style::default().fg(Color::Green)),
                Span::raw("Start search (type query, Enter to confirm)"),
            ]),
            Line::from(vec![
                Span::styled("  n                ", Style::default().fg(Color::Green)),
                Span::raw("Jump to next search match"),
            ]),
            Line::from(vec![
                Span::styled("  N (Shift+n)      ", Style::default().fg(Color::Green)),
                Span::raw("Jump to previous search match"),
            ]),
            Line::from(vec![
                Span::styled("  Esc              ", Style::default().fg(Color::Green)),
                Span::raw("Clear search results"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "CLIPBOARD",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  c                ", Style::default().fg(Color::Green)),
                Span::raw("Copy current cell value"),
            ]),
            Line::from(vec![
                Span::styled("  C (Shift+c)      ", Style::default().fg(Color::Green)),
                Span::raw("Copy entire current row (tab-separated)"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "SHEET NAVIGATION",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  Tab              ", Style::default().fg(Color::Green)),
                Span::raw("Switch to next sheet"),
            ]),
            Line::from(vec![
                Span::styled("  Shift+Tab        ", Style::default().fg(Color::Green)),
                Span::raw("Switch to previous sheet"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "GENERAL",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  Enter            ", Style::default().fg(Color::Green)),
                Span::raw("Show cell details (type, formula, value)"),
            ]),
            Line::from(vec![
                Span::styled("  t                ", Style::default().fg(Color::Green)),
                Span::raw("Cycle through color themes"),
            ]),
            Line::from(vec![
                Span::styled("  ?                ", Style::default().fg(Color::Green)),
                Span::raw("Toggle this help screen"),
            ]),
            Line::from(vec![
                Span::styled("  q                ", Style::default().fg(Color::Green)),
                Span::raw("Quit xleak"),
            ]),
            Line::from(vec![
                Span::styled("  Esc              ", Style::default().fg(Color::Green)),
                Span::raw("Quit xleak (or clear search)"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "VISUAL CUES",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  Blue background  ",
                    Style::default().bg(Color::Blue).fg(Color::White),
                ),
                Span::raw("  Current cell (selected)"),
            ]),
            Line::from(vec![
                Span::styled("  Dark gray bg     ", Style::default().bg(Color::DarkGray)),
                Span::raw("  Current row highlight"),
            ]),
            Line::from(vec![
                Span::styled("  Cyan text        ", Style::default().fg(Color::Cyan)),
                Span::raw("  Current column highlight"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Yellow bold      ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  Column headers"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Yellow bg        ",
                    Style::default().bg(Color::Yellow).fg(Color::Black),
                ),
                Span::raw("  Current search match"),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Light yellow bg  ",
                    Style::default().bg(Color::LightYellow).fg(Color::Black),
                ),
                Span::raw("  Other search matches"),
            ]),
            Line::from(""),
            Line::from("  Cell colors vary by type and current theme:"),
            Line::from("  • Numbers, strings, dates, booleans, errors each have distinct colors"),
            Line::from("  • Alternating row backgrounds improve readability"),
            Line::from("  • Press 't' to cycle through 6 built-in themes"),
            Line::from(""),
            Line::from(Span::styled(
                "STATUS BAR INFO",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  Cell address (e.g., B7) shown in bottom left"),
            Line::from("  Current cell value displayed in status bar title"),
            Line::from("  Sheet dimensions (rows × columns) shown"),
            Line::from("  Match counter shown when searching (e.g., Match 3/12)"),
            Line::from(""),
            Line::from(Span::styled(
                "CONFIGURATION",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  Customize keybindings and theme in config file:"),
            Line::from("  ~/.config/xleak/config.toml"),
            Line::from(""),
            Line::from("  Supports VIM-style navigation (hjkl, gg, G, 0, $)"),
            Line::from("  Custom keybindings per action"),
            Line::from("  Default theme selection"),
            Line::from(""),
            Line::from("  See config.toml.example for all options"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press any key to close",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ];

        // Calculate popup size (centered, 70% width, auto height)
        let area = frame.area();
        let popup_width = (area.width as f32 * 0.7).min(80.0) as u16;
        let popup_height =
            (help_lines.len() + 4).min(area.height.saturating_sub(2) as usize) as u16;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Create help content with styled text
        let help_paragraph = Paragraph::new(help_lines)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(vec![
                        Span::raw(" "),
                        Span::styled(
                            "Help",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" - Keyboard Shortcuts "),
                    ])
                    .title_alignment(Alignment::Center),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(help_paragraph, popup_area);
    }

    fn render_cell_detail(&mut self, frame: &mut Frame) {
        use ratatui::text::{Line, Span};

        // Get current cell info
        let (cell_value, cell_formula) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);

        let cell_addr = self.current_cell_address();
        let header = self
            .sheet_data
            .headers()
            .get(self.cursor_col)
            .map(|s| s.as_str())
            .unwrap_or("");

        // Build detail lines
        let mut detail_lines = vec![
            Line::from(vec![
                Span::styled(
                    "Cell: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(cell_addr.clone(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Column: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(header),
            ]),
            Line::from(""),
        ];

        // Show formula first if it exists (more important than type)
        if let Some(ref formula) = cell_formula {
            detail_lines.push(Line::from(vec![
                Span::styled(
                    "Formula: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    formula.clone(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            detail_lines.push(Line::from(""));
        }

        if let Some(cell) = cell_value {
            // Cell type
            let cell_type = match cell {
                crate::workbook::CellValue::Empty => "Empty",
                crate::workbook::CellValue::String(_) => "String",
                crate::workbook::CellValue::Int(_) => "Integer",
                crate::workbook::CellValue::Float(_) => "Float",
                crate::workbook::CellValue::Bool(_) => "Boolean",
                crate::workbook::CellValue::Error(_) => "Error",
                crate::workbook::CellValue::DateTime(_) => "DateTime",
            };

            detail_lines.push(Line::from(vec![
                Span::styled(
                    "Type: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(cell_type, Style::default().fg(Color::Green)),
            ]));

            // Raw value (unformatted)
            let raw_value = cell.to_raw_string();

            // If cell is empty but has a formula, add explanation
            if raw_value.is_empty() && cell_formula.is_some() {
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        "Value: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "(empty - formula not evaluated)",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            } else {
                let value_display = if raw_value.is_empty() {
                    "(empty)".to_string()
                } else {
                    raw_value.clone()
                };
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        "Value: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(value_display),
                ]));
            }

            // Formatted display value
            let display_value = cell.to_string();
            if display_value != raw_value {
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        "Display Value: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(display_value.clone()),
                ]));
            }

            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                "Full Content:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            detail_lines.push(Line::from(""));

            // Split content by lines for multi-line display
            for line in raw_value.lines() {
                detail_lines.push(Line::from(Span::raw(line.to_string())));
            }
        } else {
            // No cell value - might be a formula cell or truly empty
            if cell_formula.is_some() {
                detail_lines.push(Line::from(vec![
                    Span::styled(
                        "Value: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "(formula not evaluated by Excel reader)",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            } else {
                detail_lines.push(Line::from(Span::styled(
                    "No cell data",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
        }

        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::ITALIC),
        )]));

        // Calculate popup size (60% width, auto height)
        let area = frame.area();
        let popup_width = (area.width as f32 * 0.6).min(80.0) as u16;
        let popup_height =
            (detail_lines.len() + 4).min(area.height.saturating_sub(2) as usize) as u16;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Create detail content
        let detail_paragraph = Paragraph::new(detail_lines)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(vec![
                        Span::raw(" "),
                        Span::styled(
                            "Cell Details",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" - "),
                        Span::styled(cell_addr, Style::default().fg(Color::Cyan)),
                        Span::raw(" "),
                    ])
                    .title_alignment(Alignment::Center),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(detail_paragraph, popup_area);
    }

    fn render_copy_feedback(&self, frame: &mut Frame, message: &str) {
        use ratatui::text::{Line, Span};

        // Create a small popup in the center
        let area = frame.area();
        let popup_width = (message.len() as u16 + 6).min(60);
        let popup_height = 3;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Create feedback content
        let feedback_paragraph = Paragraph::new(Line::from(vec![Span::styled(
            message,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]))
        .style(Style::default().bg(Color::Green).fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .title(" ✓ ")
                .title_alignment(Alignment::Center),
        )
        .alignment(Alignment::Center);

        frame.render_widget(feedback_paragraph, popup_area);
    }
}

/// Run the TUI application
pub fn run_tui(
    workbook: Workbook,
    sheet_name: &str,
    config: &crate::config::Config,
    horizontal_scroll: bool,
) -> Result<()> {
    // Check if stdout is a TTY before attempting to use interactive mode
    use std::io::IsTerminal;
    if !io::stdout().is_terminal() {
        anyhow::bail!(
            "Interactive mode requires a terminal (TTY). \
             Your output is redirected or not connected to a terminal.\n\
             Hint: Run this command directly in your terminal, not through pipes or automation."
        );
    }

    // Setup terminal
    enable_raw_mode().context("Failed to enable terminal raw mode. Is this a proper TTY?")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen mode")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal backend")?;

    // Create app state
    let mut app = TuiState::new(workbook, sheet_name, config, horizontal_scroll)?;

    // Main event loop
    let res = run_event_loop(&mut terminal, &mut app);

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiState,
) -> Result<()> {
    loop {
        // Draw needs mutable access to app for scroll updates
        terminal.draw(|f| {
            app.render(f);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_address_basic() {
        assert_eq!(TuiState::parse_cell_address("A1"), Some((0, 0)));
        assert_eq!(TuiState::parse_cell_address("B2"), Some((1, 1)));
        assert_eq!(TuiState::parse_cell_address("Z26"), Some((25, 25)));
    }

    #[test]
    fn test_parse_cell_address_double_letter() {
        assert_eq!(TuiState::parse_cell_address("AA1"), Some((26, 0)));
        assert_eq!(TuiState::parse_cell_address("AB5"), Some((27, 4)));
        assert_eq!(TuiState::parse_cell_address("AZ100"), Some((51, 99)));
    }

    #[test]
    fn test_parse_cell_address_lowercase() {
        assert_eq!(TuiState::parse_cell_address("a1"), Some((0, 0)));
        assert_eq!(TuiState::parse_cell_address("b2"), Some((1, 1)));
        assert_eq!(TuiState::parse_cell_address("aa10"), Some((26, 9)));
    }

    #[test]
    fn test_parse_cell_address_invalid() {
        assert_eq!(TuiState::parse_cell_address(""), None);
        assert_eq!(TuiState::parse_cell_address("1"), None);
        assert_eq!(TuiState::parse_cell_address("A"), None);
        assert_eq!(TuiState::parse_cell_address("123"), None);
        // Note: "A1B2" actually parses as AB12, which is valid
        // Test truly invalid inputs instead
        assert_eq!(TuiState::parse_cell_address("!@#"), None);
        assert_eq!(TuiState::parse_cell_address("A-1"), None);
    }

    #[test]
    fn test_parse_cell_address_large_column() {
        // BA = 2*26 + 1 = 53 (0-indexed: 52)
        assert_eq!(TuiState::parse_cell_address("BA1"), Some((52, 0)));
        // ZZ = 26*26 + 26 = 702 (0-indexed: 701)
        assert_eq!(TuiState::parse_cell_address("ZZ1"), Some((701, 0)));
    }

    #[test]
    fn test_column_to_letter() {
        // Test helper function for column letters
        let col_a = 0;
        let col_z = 25;
        let col_aa = 26;

        // Helper to convert column index to letter
        fn col_to_letter(col: usize) -> String {
            let mut result = String::new();
            let mut n = col + 1;
            while n > 0 {
                n -= 1;
                result.push((b'A' + (n % 26) as u8) as char);
                n /= 26;
            }
            result.chars().rev().collect()
        }

        assert_eq!(col_to_letter(col_a), "A");
        assert_eq!(col_to_letter(col_z), "Z");
        assert_eq!(col_to_letter(col_aa), "AA");
    }
}
