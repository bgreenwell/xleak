use crate::utils;
use crate::workbook::{CellValue, LazySheetData, SheetData, Workbook};
use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;

use super::clipboard::{self, CopyOutcome};
use super::theme::Theme;

/// Cached row data for lazy loading
pub(crate) struct RowCache {
    pub start_row: usize,
    pub rows: Vec<Vec<CellValue>>,
    pub formulas: Vec<Vec<Option<String>>>,
}

/// Sheet data source (either eager or lazy)
pub(crate) enum SheetDataSource {
    Eager(SheetData),
    Lazy {
        data: LazySheetData,
        cache: Option<RowCache>,
        cache_size: usize, // Number of rows to cache at once
        no_header: bool,
    },
}

impl SheetDataSource {
    pub fn headers(&self) -> &[String] {
        match self {
            SheetDataSource::Eager(data) => &data.headers,
            SheetDataSource::Lazy { data, .. } => &data.headers,
        }
    }

    pub fn width(&self) -> usize {
        match self {
            SheetDataSource::Eager(data) => data.width,
            SheetDataSource::Lazy { data, .. } => data.width,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            SheetDataSource::Eager(data) => data.height,
            SheetDataSource::Lazy { data, .. } => data.height,
        }
    }

    /// Fetches rows with automatic cache management
    pub fn get_rows(
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
                no_header,
            } => {
                if start >= data.height {
                    return (&[], &[]);
                }
                // Clamp to the rows that actually exist so a request past the
                // sheet end can be satisfied by the cache without reloading.
                let count = count.min(data.height - start);

                // Reload when the request isn't fully covered by the cache
                // (#51: returning only the cached remainder made search and
                // tall viewports silently miss rows).
                let needs_reload = match cache {
                    None => true,
                    Some(c) => start < c.start_row || start + count > c.start_row + c.rows.len(),
                };

                if needs_reload {
                    // Start the chunk a little before the request so backward
                    // scrolling stays within the cache, and load at least
                    // enough to cover the full request.
                    let cache_start = start.saturating_sub(*cache_size / 4);
                    let load = (*cache_size).max(start + count - cache_start);
                    let (rows, formulas) = data.get_rows(cache_start, load, *no_header);
                    *cache = Some(RowCache {
                        start_row: cache_start,
                        rows,
                        formulas,
                    });
                }

                if let Some(c) = cache {
                    let offset = start.saturating_sub(c.start_row);
                    let end = (offset + count).min(c.rows.len());
                    (&c.rows[offset..end], &c.formulas[offset..end])
                } else {
                    (&[], &[])
                }
            }
        }
    }

    pub fn get_cell(&mut self, row: usize, col: usize) -> (Option<CellValue>, Option<String>) {
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
pub(crate) struct ProgressInfo {
    message: String,
    current: usize,
    total: usize,
    started_at: Instant,
}

impl ProgressInfo {
    pub fn new(message: impl Into<String>, total: usize) -> Self {
        Self {
            message: message.into(),
            current: 0,
            total,
            started_at: Instant::now(),
        }
    }

    pub fn update(&mut self, current: usize) {
        self.current = current;
    }

    pub fn percentage(&self) -> usize {
        (self.current * 100).checked_div(self.total).unwrap_or(100)
    }

    pub fn format(&self) -> String {
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
    pub workbook: Workbook,
    pub sheet_names: Vec<String>,
    pub current_sheet_index: usize,
    pub sheet_data: SheetDataSource,
    pub should_quit: bool,
    pub cursor_row: usize,               // Current row (0-indexed in data)
    pub cursor_col: usize,               // Current column (0-indexed)
    pub scroll_offset: usize,            // Vertical scroll offset
    pub horizontal_scroll_offset: usize, // Horizontal scroll offset
    pub horizontal_scroll_enabled: bool, // Whether horizontal scrolling is enabled
    pub column_widths: Vec<usize>,       // Cached column widths for horizontal scroll
    pub show_help: bool,                 // Help overlay visible
    pub help_scroll: usize,              // Scroll offset for help popup
    pub show_cell_detail: bool,          // Cell detail popup visible
    pub cell_detail_scroll: usize,       // Scroll offset for cell detail popup
    // Search state
    pub search_mode: bool,    // Whether we're in search input mode
    pub search_query: String, // Current search query
    pub search_matches: Vec<(usize, usize)>, // List of (row, col) matches
    pub current_match_index: Option<usize>, // Index in search_matches
    // Jump mode state
    pub jump_mode: bool,    // Whether we're in jump input mode
    pub jump_input: String, // Current jump input (row number or cell address)
    // Clipboard state
    pub copy_feedback: Option<(String, Instant)>, // Message and timestamp for copy feedback
    // Progress state
    pub progress: Option<ProgressInfo>, // Current operation progress
    // Theme state
    pub current_theme: Theme, // Current color theme
    // Config state
    pub config: crate::config::Config, // User configuration
    // No-header mode
    pub no_header: bool,
    // Hide the column-letter row (A, B, C, ...)
    pub no_column_id: bool,
    // Hide the row-number column
    pub no_row_id: bool,
    // Remembered cursor/scroll position per sheet index, so switching back to a
    // previously-viewed sheet restores where the user left off.
    // Value: (cursor_row, cursor_col, scroll_offset, horizontal_scroll_offset).
    sheet_positions: HashMap<usize, (usize, usize, usize, usize)>,
}

impl TuiState {
    pub const LAZY_LOADING_THRESHOLD: usize = 1000;
    pub const ROW_CACHE_SIZE: usize = 200;

    pub fn new(
        mut workbook: Workbook,
        initial_sheet_name: &str,
        config: &crate::config::Config,
        horizontal_scroll: bool,
        no_header: bool,
        no_column_id: bool,
        no_row_id: bool,
    ) -> Result<Self> {
        let sheet_names = workbook.sheet_names();
        let current_sheet_index = sheet_names
            .iter()
            .position(|name| name == initial_sheet_name)
            .unwrap_or(0);

        // Load sheet lazily first to check size
        let lazy_data = workbook.load_sheet_lazy(&sheet_names[current_sheet_index], no_header)?;
        let sheet_height = lazy_data.height;

        // Choose loading strategy based on size
        let sheet_data = if sheet_height > Self::LAZY_LOADING_THRESHOLD {
            SheetDataSource::Lazy {
                data: lazy_data,
                cache: None,
                cache_size: Self::ROW_CACHE_SIZE,
                no_header,
            }
        } else {
            // Convert to eager loading for small files
            SheetDataSource::Eager(lazy_data.to_sheet_data(no_header))
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
            help_scroll: 0,
            show_cell_detail: false,
            cell_detail_scroll: 0,
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
            no_header,
            no_column_id,
            no_row_id,
            sheet_positions: HashMap::new(),
        };

        // Calculate column widths if horizontal scrolling is enabled
        if horizontal_scroll {
            state.column_widths = state.calculate_column_widths();
        }

        Ok(state)
    }

    /// Parse theme name from config string
    pub fn parse_theme_name(name: &str) -> Theme {
        match name.to_lowercase().as_str() {
            "dracula" => Theme::Dracula,
            "solarized dark" | "solarizeddark" => Theme::SolarizedDark,
            "solarized light" | "solarizedlight" => Theme::SolarizedLight,
            "github dark" | "githubdark" => Theme::GitHubDark,
            "nord" => Theme::Nord,
            _ => Theme::Default,
        }
    }

    pub fn current_sheet_name(&self) -> &str {
        &self.sheet_names[self.current_sheet_index]
    }

    pub fn switch_to_next_sheet(&mut self) -> Result<()> {
        if self.sheet_names.len() <= 1 {
            return Ok(());
        }

        self.save_current_position();
        self.current_sheet_index = (self.current_sheet_index + 1) % self.sheet_names.len();
        self.load_current_sheet()?;
        self.restore_position();
        self.clear_search();
        Ok(())
    }

    pub fn switch_to_prev_sheet(&mut self) -> Result<()> {
        if self.sheet_names.len() <= 1 {
            return Ok(());
        }

        self.save_current_position();
        self.current_sheet_index = if self.current_sheet_index == 0 {
            self.sheet_names.len() - 1
        } else {
            self.current_sheet_index - 1
        };
        self.load_current_sheet()?;
        self.restore_position();
        self.clear_search();
        Ok(())
    }

    /// Remember the current cursor and scroll position for the active sheet.
    fn save_current_position(&mut self) {
        self.sheet_positions.insert(
            self.current_sheet_index,
            (
                self.cursor_row,
                self.cursor_col,
                self.scroll_offset,
                self.horizontal_scroll_offset,
            ),
        );
    }

    /// Restore the remembered position for the active sheet, clamping to the new
    /// sheet's bounds. Falls back to the top-left when no position is stored.
    fn restore_position(&mut self) {
        if let Some(&(row, col, scroll, hscroll)) =
            self.sheet_positions.get(&self.current_sheet_index)
        {
            let max_row = self.sheet_data.height().saturating_sub(1);
            let max_col = self.sheet_data.width().saturating_sub(1);
            self.cursor_row = row.min(max_row);
            self.cursor_col = col.min(max_col);
            // Keep the stored scroll offsets (clamped); the next render's
            // update_scroll/update_horizontal_scroll will fine-tune visibility.
            self.scroll_offset = scroll.min(max_row);
            self.horizontal_scroll_offset = hscroll.min(max_col);
        } else {
            self.reset_cursor();
        }
    }

    pub fn load_current_sheet(&mut self) -> Result<()> {
        let sheet_name = self.sheet_names[self.current_sheet_index].clone();

        let lazy_data = self.workbook.load_sheet_lazy(&sheet_name, self.no_header)?;
        let sheet_height = lazy_data.height;

        self.sheet_data = if sheet_height > Self::LAZY_LOADING_THRESHOLD {
            SheetDataSource::Lazy {
                data: lazy_data,
                cache: None,
                cache_size: Self::ROW_CACHE_SIZE,
                no_header: self.no_header,
            }
        } else {
            SheetDataSource::Eager(lazy_data.to_sheet_data(self.no_header))
        };

        if self.horizontal_scroll_enabled {
            self.column_widths = self.calculate_column_widths();
        }

        Ok(())
    }

    pub fn reset_cursor(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.horizontal_scroll_offset = 0;
    }

    /// Perform case-insensitive search across all cells
    pub fn perform_search(&mut self) {
        self.search_matches.clear();
        self.current_match_index = None;

        if self.search_query.is_empty() {
            self.progress = None;
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        let total_height = self.sheet_data.height();

        if total_height > 1000 {
            self.progress = Some(ProgressInfo::new("Searching", total_height));
        }

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

            if let Some(ref mut progress) = self.progress {
                progress.update(chunk_start + chunk_size);
            }
        }

        self.progress = None;

        if !self.search_matches.is_empty() {
            self.current_match_index = Some(0);
            self.jump_to_current_match();
        }
    }

    pub fn jump_to_next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match_index = Some(match self.current_match_index {
            Some(idx) => (idx + 1) % self.search_matches.len(),
            None => 0,
        });

        self.jump_to_current_match();
    }

    pub fn jump_to_prev_match(&mut self) {
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

    pub fn jump_to_current_match(&mut self) {
        if let Some(idx) = self.current_match_index
            && let Some(&(row, col)) = self.search_matches.get(idx)
        {
            self.cursor_row = row;
            self.cursor_col = col;
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match_index = None;
    }

    pub fn enter_jump_mode(&mut self) {
        self.jump_mode = true;
        self.jump_input.clear();
    }

    /// Parse jump input and navigate to that location
    pub fn perform_jump(&mut self) {
        if self.jump_input.is_empty() {
            self.jump_mode = false;
            return;
        }

        let input = self.jump_input.trim();

        // Row numbers and cell addresses are interpreted as real sheet rows, so
        // they match the row numbers shown in the gutter. Sheet row 1 is the
        // header (when present); jumping there lands on the first data row.
        let first_row = self.first_data_sheet_row();
        let last_row = self.data_row_to_sheet_row(self.sheet_data.height().saturating_sub(1));

        if let Ok(row_num) = input.parse::<usize>() {
            if self.sheet_data.height() > 0 && row_num >= first_row && row_num <= last_row {
                self.cursor_row = self.sheet_row_to_data_row(row_num);
                self.copy_feedback = Some((format!("Jumped to row {}", row_num), Instant::now()));
            } else {
                self.copy_feedback = Some((
                    format!("Invalid row: {} (max: {})", row_num, last_row),
                    Instant::now(),
                ));
            }
        } else if let Some((col, row)) = Self::parse_cell_address(input) {
            // `row` is a 0-based real sheet row; map it to a data-row index.
            let sheet_row = row + 1;
            if self.sheet_data.height() > 0
                && sheet_row >= first_row
                && sheet_row <= last_row
                && col < self.sheet_data.width()
            {
                self.cursor_row = self.sheet_row_to_data_row(sheet_row);
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
        } else if let Some((row, col)) = input.split_once(',') {
            if let (Ok(row_num), Ok(col_num)) =
                (row.trim().parse::<usize>(), col.trim().parse::<usize>())
            {
                if self.sheet_data.height() > 0
                    && row_num >= first_row
                    && row_num <= last_row
                    && col_num > 0
                    && col_num <= self.sheet_data.width()
                {
                    self.cursor_row = self.sheet_row_to_data_row(row_num);
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
    pub fn parse_cell_address(addr: &str) -> Option<(usize, usize)> {
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
        Some((col - 1, row - 1))
    }

    pub fn copy_current_cell(&mut self) {
        let (cell, _formula) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);
        let cell_value = cell.map(|v| v.to_raw_string()).unwrap_or_default();

        match clipboard::copy(&cell_value) {
            CopyOutcome::Ok => {
                let cell_addr = self.current_cell_address();
                self.copy_feedback = Some((format!("Copied cell {}", cell_addr), Instant::now()));
            }
            CopyOutcome::Failed(e) => {
                self.copy_feedback = Some((format!("Copy failed: {}", e), Instant::now()));
            }
        }
    }

    pub fn copy_current_row(&mut self) {
        let (rows, _formulas) = self.sheet_data.get_rows(self.cursor_row, 1);
        let row_values = rows
            .first()
            .map(|row| {
                row.iter()
                    .map(|cell| crate::display::csv_quote(cell.to_raw_string(), "\t"))
                    .collect::<Vec<_>>()
                    .join("\t")
            })
            .unwrap_or_default();

        match clipboard::copy(&row_values) {
            CopyOutcome::Ok => {
                self.copy_feedback = Some((
                    format!(
                        "Copied row {} ({} cells)",
                        self.data_row_to_sheet_row(self.cursor_row),
                        self.sheet_data.width()
                    ),
                    Instant::now(),
                ));
            }
            CopyOutcome::Failed(e) => {
                self.copy_feedback = Some((format!("Copy failed: {}", e), Instant::now()));
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row < self.sheet_data.height().saturating_sub(1) {
            self.cursor_row += 1;
        }
    }

    pub fn update_scroll(&mut self, viewport_height: usize) {
        if self.cursor_row >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_row.saturating_sub(viewport_height - 1);
        }
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        }
    }

    pub fn calculate_column_widths(&mut self) -> Vec<usize> {
        let num_cols = self.sheet_data.width();
        let mut widths = vec![0; num_cols];

        let headers = self.sheet_data.headers();
        for (i, header) in headers.iter().enumerate() {
            widths[i] = header.len();
        }

        let sample_size = 100.min(self.sheet_data.height());
        let (sample_rows, _) = self.sheet_data.get_rows(0, sample_size);

        for row in sample_rows.iter() {
            for (col_idx, cell) in row.iter().enumerate() {
                let len = cell.to_string().len();
                widths[col_idx] = widths[col_idx].max(len);
            }
        }

        widths.iter().map(|&w| w.clamp(3, 30)).collect()
    }

    pub fn update_horizontal_scroll(&mut self, viewport_width: usize) {
        if !self.horizontal_scroll_enabled {
            return;
        }

        let mut total_width = 0;
        let mut visible_end = self.horizontal_scroll_offset;

        for i in self.horizontal_scroll_offset..self.column_widths.len() {
            total_width += self.column_widths[i] + 1;
            visible_end = i + 1;
            if total_width > viewport_width {
                break;
            }
        }

        if self.cursor_col >= visible_end {
            self.horizontal_scroll_offset += 1;
        }

        if self.cursor_col < self.horizontal_scroll_offset {
            self.horizontal_scroll_offset = self.cursor_col;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            if self.horizontal_scroll_enabled && self.cursor_col < self.horizontal_scroll_offset {
                self.horizontal_scroll_offset = self.cursor_col;
            }
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_col < self.sheet_data.width().saturating_sub(1) {
            self.cursor_col += 1;
        }
    }

    pub fn move_to_start_of_row(&mut self) {
        self.cursor_col = 0;
        if self.horizontal_scroll_enabled {
            self.horizontal_scroll_offset = 0;
        }
    }

    pub fn move_to_end_of_row(&mut self) {
        self.cursor_col = self.sheet_data.width().saturating_sub(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.cursor_row = self.cursor_row.saturating_sub(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.cursor_row =
            (self.cursor_row + page_size).min(self.sheet_data.height().saturating_sub(1));
    }

    pub fn move_to_top(&mut self) {
        self.cursor_row = 0;
    }

    pub fn move_to_bottom(&mut self) {
        self.cursor_row = self.sheet_data.height().saturating_sub(1);
    }

    /// The real sheet row number (1-based) of the first data row.
    ///
    /// The header row is always sheet row 1. When it is treated as a title
    /// (default), the first data row is sheet row 2; with `--no-header` the
    /// header row is itself data, so the first data row is sheet row 1.
    pub fn first_data_sheet_row(&self) -> usize {
        Self::first_data_sheet_row_for(self.no_header)
    }

    /// Pure form of [`first_data_sheet_row`](Self::first_data_sheet_row).
    pub(crate) fn first_data_sheet_row_for(no_header: bool) -> usize {
        if no_header { 1 } else { 2 }
    }

    /// Convert a 0-based data-row index into its real 1-based sheet row number.
    pub fn data_row_to_sheet_row(&self, data_row: usize) -> usize {
        data_row + self.first_data_sheet_row()
    }

    /// Convert a real 1-based sheet row number into a 0-based data-row index,
    /// clamping the header row (and anything above the first data row) to the
    /// first selectable data row.
    pub fn sheet_row_to_data_row(&self, sheet_row: usize) -> usize {
        sheet_row.saturating_sub(self.first_data_sheet_row())
    }

    pub fn current_cell_address(&self) -> String {
        format!(
            "{}{}",
            utils::column_index_to_letters(self.cursor_col),
            self.data_row_to_sheet_row(self.cursor_row)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Data, Range};

    /// Build a lazy source over `height` data rows (plus a header row), with a
    /// deliberately small cache so requests can exceed it.
    fn lazy_source(height: usize, cache_size: usize) -> SheetDataSource {
        let mut range: Range<Data> = Range::new((0, 0), (height as u32, 0));
        range.set_value((0, 0), Data::String("header".into()));
        for r in 1..=height {
            range.set_value((r as u32, 0), Data::Int(r as i64));
        }
        SheetDataSource::Lazy {
            data: LazySheetData::from_range_with_formulas(range, None, false),
            cache: None,
            cache_size,
            no_header: false,
        }
    }

    #[test]
    fn test_lazy_get_rows_returns_full_request_beyond_cache_size() {
        // Regression for #51: a request larger than the cache came back
        // truncated to the cache size, so search silently skipped rows.
        let mut source = lazy_source(100, 10);
        let (rows, formulas) = source.get_rows(0, 50);
        assert_eq!(rows.len(), 50);
        assert_eq!(formulas.len(), 50);
        assert_eq!(rows[49][0].to_raw_string(), "50");
    }

    #[test]
    fn test_lazy_get_rows_reloads_when_request_overruns_cache_tail() {
        // A request starting inside the cache but extending past its end must
        // reload rather than return the truncated remainder.
        let mut source = lazy_source(100, 20);
        source.get_rows(0, 20); // warm the cache with rows 0..20
        let (rows, _) = source.get_rows(15, 20);
        assert_eq!(rows.len(), 20);
        assert_eq!(rows[0][0].to_raw_string(), "16");
        assert_eq!(rows[19][0].to_raw_string(), "35");
    }

    #[test]
    fn test_lazy_get_rows_clamps_at_sheet_end_without_thrashing() {
        let mut source = lazy_source(30, 10);
        let (rows, _) = source.get_rows(25, 50);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[4][0].to_raw_string(), "30");
        // Entirely past the end: empty, no panic.
        let (rows, _) = source.get_rows(30, 10);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_search_style_chunked_scan_visits_every_row() {
        // Mimics perform_search: fixed 500-row chunks over a lazy source whose
        // cache is smaller than the chunk. Every row must be visited once.
        let mut source = lazy_source(1200, 200);
        let mut seen = Vec::new();
        const CHUNK: usize = 500;
        let total = source.height();
        for chunk_start in (0..total).step_by(CHUNK) {
            let chunk_size = CHUNK.min(total - chunk_start);
            let (rows, _) = source.get_rows(chunk_start, chunk_size);
            assert_eq!(rows.len(), chunk_size, "chunk at {chunk_start} truncated");
            for (i, row) in rows.iter().enumerate() {
                seen.push((chunk_start + i, row[0].to_raw_string()));
            }
        }
        assert_eq!(seen.len(), 1200);
        // Row i holds the value i+1; spot-check the previously skipped region.
        assert_eq!(seen[250], (250, "251".to_string()));
        assert_eq!(seen[1199], (1199, "1200".to_string()));
    }
}
