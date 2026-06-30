use crate::{utils, workbook::CellValue};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use std::time::Duration;

use super::state::TuiState;

impl TuiState {
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // The status bar shows the current cell value, wrapping onto extra lines
        // (up to 3/4 of the terminal height) when it is too long for one line.
        let (cell, _) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);
        let current_cell_value = cell.map(|v| v.to_string()).unwrap_or_default();
        let status_text = if current_cell_value.is_empty() {
            "(empty)".to_string()
        } else {
            format!("{current_cell_value} ")
        };

        let mut status_style = Style::default().fg(self.current_theme.colors().status_bar_fg);
        if let Some(bg) = self.current_theme.colors().status_bar_bg {
            status_style = status_style.bg(bg);
        }

        // Sticky header rows: optional column-letter row and optional title row.
        let show_column_id = !self.no_column_id;
        let show_title_row = !self.no_header;
        let show_row_id = !self.no_row_id;
        let header_row_count: u16 = show_column_id as u16 + show_title_row as u16;

        // The data table needs at least: top/bottom borders (2) + the sticky
        // header rows + one visible data row. Reserve that (plus the 1-line info
        // bar) so a tall cell value can never squeeze the data area to nothing.
        let min_table_height = 2 + header_row_count + 1;
        let reserved = min_table_height + 1; // + info bar
        let height_cap = area.height.saturating_sub(reserved).max(1);

        let max_status_height = ((area.height as usize * 3 / 4).max(1) as u16).min(height_cap);
        let wrapped_lines = Self::wrapped_line_count(&status_text, area.width) as u16;
        let status_height = wrapped_lines.max(1).min(max_status_height);

        // When the value is taller than the space we can give the status bar, the
        // last line becomes a hint pointing to the full-value popup.
        let status_truncated = wrapped_lines > status_height;

        let status_paragraph = Paragraph::new(status_text)
            .style(status_style)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::NONE));

        // Layout: data table | info bar (1 line) | status bar (dynamic height)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(min_table_height),
                Constraint::Length(1),
                Constraint::Length(status_height),
            ])
            .split(area);

        // Viewport excludes borders (2) and the sticky header rows.
        let table_height = chunks[0]
            .height
            .saturating_sub(2)
            .saturating_sub(header_row_count) as usize;
        let viewport_width = chunks[0].width.saturating_sub(2) as usize;

        self.update_scroll(table_height);
        self.update_horizontal_scroll(viewport_width);

        let visible_start = self.scroll_offset;

        let (visible_col_start, visible_col_end) = if self.horizontal_scroll_enabled {
            let mut total_width = 0;
            let mut end = self.horizontal_scroll_offset;

            for i in self.horizontal_scroll_offset..self.column_widths.len() {
                total_width += self.column_widths[i] + 1;
                end = i + 1;
                if total_width > viewport_width {
                    break;
                }
            }
            (self.horizontal_scroll_offset, end)
        } else {
            (0, self.sheet_data.width())
        };

        // Cloned to avoid borrowing self while building rows.
        let headers = self.sheet_data.headers().to_vec();
        let colors = self.current_theme.colors();

        let mut header_cells: Vec<Cell> = Vec::new();

        // Row-number gutter for the sticky header. The column-letter line (if any)
        // has no sheet row, so it stays blank; the title line is the real sheet
        // row 1, so it shows "1" — matching the gutter used for data rows.
        if show_row_id {
            let mut gutter_lines: Vec<Line> = Vec::with_capacity(header_row_count.max(1) as usize);
            if show_column_id {
                gutter_lines.push(Line::from(""));
            }
            if show_title_row {
                gutter_lines.push(Line::from(format!("{:>4} ", 1)));
            }
            if gutter_lines.is_empty() {
                gutter_lines.push(Line::from(""));
            }
            header_cells.push(
                Cell::from(gutter_lines).style(
                    Style::default()
                        .fg(colors.header_fg)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }

        header_cells.extend(
            headers
                .iter()
                .enumerate()
                .skip(visible_col_start)
                .take(visible_col_end - visible_col_start)
                .map(|(col_idx, h)| {
                    let mut base_style = Style::default()
                        .fg(colors.header_fg)
                        .add_modifier(Modifier::BOLD);

                    if let Some(bg) = colors.header_bg {
                        base_style = base_style.bg(bg);
                    }

                    let highlight_style = if col_idx == self.cursor_col {
                        base_style.fg(colors.current_col_fg)
                    } else {
                        base_style
                    };

                    let mut lines = Vec::with_capacity(header_row_count as usize);

                    // Column-letter line.
                    if show_column_id {
                        lines.push(Line::from(Span::styled(
                            utils::column_index_to_letters(col_idx),
                            highlight_style,
                        )));
                    }

                    // Title line.
                    if show_title_row {
                        lines.push(Line::from(Span::styled(h.as_str(), highlight_style)));
                    }

                    Cell::from(lines).style(base_style)
                }),
        );

        let header = Row::new(header_cells).height(header_row_count.max(1));

        let row_number_offset = self.first_data_sheet_row();

        let (visible_rows, _visible_formulas) =
            self.sheet_data.get_rows(visible_start, table_height);

        let data_rows: Vec<Row> = visible_rows
            .iter()
            .enumerate()
            .map(|(visible_idx, row)| {
                let row_idx = visible_start + visible_idx;
                let display_row_num = row_idx + row_number_offset;

                let mut cells: Vec<Cell> = Vec::new();

                if show_row_id {
                    let row_num_style = if row_idx == self.cursor_row {
                        Style::default()
                            .fg(colors.current_cell_fg)
                            .bg(colors.current_cell_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.empty_fg)
                    };
                    cells.push(Cell::from(format!("{:>4} ", display_row_num)).style(row_num_style));
                }

                cells.extend(
                    row.iter()
                        .enumerate()
                        .skip(visible_col_start)
                        .take(visible_col_end - visible_col_start)
                        .map(|(col_idx, cell)| {
                            let mut style = Style::default().fg(colors.cell_color(cell));

                            let is_alternating_row = row_idx % 2 == 1;
                            if is_alternating_row && let Some(alt_bg) = colors.alternating_row_bg {
                                style = style.bg(alt_bg);
                            }

                            let is_search_match = self.search_matches.contains(&(row_idx, col_idx));
                            let is_current_match = self
                                .current_match_index
                                .and_then(|idx| self.search_matches.get(idx))
                                .map(|&pos| pos == (row_idx, col_idx))
                                .unwrap_or(false);

                            if is_current_match {
                                style = style
                                    .bg(colors.current_search_bg)
                                    .fg(colors.current_search_fg)
                                    .add_modifier(Modifier::BOLD);
                            } else if row_idx == self.cursor_row && col_idx == self.cursor_col {
                                style = style
                                    .bg(colors.current_cell_bg)
                                    .fg(colors.current_cell_fg)
                                    .add_modifier(Modifier::BOLD);
                            } else if is_search_match {
                                style = style.bg(colors.search_match_bg).fg(colors.search_match_fg);
                            } else if row_idx == self.cursor_row {
                                style = style.bg(colors.current_row_bg);
                            } else if col_idx == self.cursor_col {
                                style = style.fg(colors.current_col_fg);
                            }
                            Cell::from(cell.to_string()).style(style)
                        })
                        .collect::<Vec<_>>(),
                );

                Row::new(cells).height(1)
            })
            .collect();

        let mut col_widths: Vec<Constraint> = Vec::new();
        if show_row_id {
            col_widths.push(Constraint::Length(5));
        }
        if self.horizontal_scroll_enabled {
            col_widths.extend(
                self.column_widths[visible_col_start..visible_col_end]
                    .iter()
                    .map(|&w| Constraint::Length(w as u16)),
            );
        } else {
            let sheet_width = self.sheet_data.width();
            col_widths.extend(
                headers
                    .iter()
                    .map(|_| Constraint::Percentage((100 / sheet_width.max(1)) as u16)),
            );
        }

        // Top-border titles: sheet name (left) and "rows×cols" dimensions (right).
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
                "{}×{} ({}-{})",
                self.sheet_data.height(),
                self.sheet_data.width(),
                first_col,
                last_col
            )
        } else {
            format!("{}×{}", self.sheet_data.height(), self.sheet_data.width())
        };

        // Sheet name (with index when there are multiple sheets) goes top-left.
        let sheet_name_full = if self.sheet_names.len() > 1 {
            format!(
                "{} ({}/{})",
                self.current_sheet_name(),
                self.current_sheet_index + 1,
                self.sheet_names.len()
            )
        } else {
            self.current_sheet_name().to_string()
        };

        let border_width = chunks[0].width.saturating_sub(2) as usize;
        let dims_display_len = sheet_dims.chars().count() + 2;

        // The sheet name may use at most 3/4 of the terminal width; dimensions take
        // priority, so the name is shortened (or dropped) when space is tight.
        let name_cap = (area.width as usize * 3 / 4).min(border_width);
        let name_budget = border_width
            .saturating_sub(dims_display_len + 1)
            .min(name_cap)
            .saturating_sub(2);
        let sheet_name_title = Self::truncate_with_ellipsis(&sheet_name_full, name_budget);

        let mut table_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.border_fg));

        if !sheet_name_title.is_empty() {
            table_block = table_block.title_top(
                Line::from(Span::styled(
                    format!(" {sheet_name_title} "),
                    // Distinct from the header/column-letter color for contrast.
                    Style::default()
                        .fg(colors.datetime_fg)
                        .add_modifier(Modifier::BOLD),
                ))
                .left_aligned(),
            );
        }
        // Only show dimensions if they fit on the border.
        if dims_display_len <= border_width {
            table_block = table_block.title_top(
                Line::from(Span::styled(
                    format!(" {sheet_dims} "),
                    // Use the numeric color since dimensions are row/column counts.
                    Style::default()
                        .fg(colors.number_fg)
                        .add_modifier(Modifier::BOLD),
                ))
                .right_aligned(),
            );
        }

        // Data table — attach the sticky header only when at least one header row
        // (column id or title) is visible.
        let mut table = Table::new(data_rows, col_widths).block(table_block);
        if header_row_count > 0 {
            table = table.header(header);
        }

        frame.render_widget(table, chunks[0]);

        // Info bar: left segment shows the cell address (and live search query while
        // searching); right segment shows the theme and key hints.
        let cell_addr = self.current_cell_address();

        let (info_left, info_right) = if let Some(ref progress) = self.progress {
            (format!(" ⏳ {} ", progress.format()), String::new())
        } else if self.jump_mode {
            (format!(" Jump: {} ", self.jump_input), String::new())
        } else if self.search_mode {
            (
                format!(" {} | {} ", cell_addr, self.search_query),
                format!(
                    "{} | t:theme /:search ?:help q:quit ",
                    self.current_theme.name()
                ),
            )
        } else if let Some(idx) = self.current_match_index {
            let match_info = format!("Match {}/{}", idx + 1, self.search_matches.len());
            (
                format!(" {} | {} ", match_info, cell_addr),
                format!(
                    "{} | n:next N:prev Esc:clear ?:help q:quit ",
                    self.current_theme.name()
                ),
            )
        } else {
            (
                format!(" {} ", cell_addr),
                format!(
                    "{} | t:theme /:search ?:help q:quit ",
                    self.current_theme.name()
                ),
            )
        };

        let info_style = Style::default()
            .fg(colors.status_bar_fg)
            .add_modifier(Modifier::BOLD);
        let info_left_widget = Paragraph::new(info_left)
            .style(info_style)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(info_left_widget, chunks[1]);

        if !info_right.is_empty() {
            let info_right_widget = Paragraph::new(info_right)
                .style(info_style)
                .alignment(Alignment::Right)
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(info_right_widget, chunks[1]);
        }

        // Status bar: the prepared (possibly multi-line) cell-value paragraph.
        // When the value is taller than the available space, reserve the bottom
        // line for a hint that points to the full-value popup.
        if status_truncated && chunks[2].height >= 2 {
            let status_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(chunks[2]);

            frame.render_widget(status_paragraph, status_chunks[0]);

            let hint = Paragraph::new(Line::from(Span::styled(
                "⏷ value truncated — press Enter to view the full cell",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            )))
            .style(status_style)
            .alignment(Alignment::Center);
            frame.render_widget(hint, status_chunks[1]);
        } else {
            frame.render_widget(status_paragraph, chunks[2]);
        }

        // Render overlays
        if self.show_cell_detail {
            self.render_cell_detail(frame);
        }

        if self.show_help {
            self.render_help(frame);
        }

        if let Some((ref message, timestamp)) = self.copy_feedback {
            if timestamp.elapsed() < Duration::from_secs(2) {
                self.render_copy_feedback(frame, message);
            } else {
                self.copy_feedback = None;
            }
        }
    }

    /// Estimate how many lines `text` occupies when wrapped to `width` columns,
    /// approximating ratatui's `Wrap { trim: false }` behavior (word wrapping with
    /// long words broken across lines, and explicit newlines respected).
    ///
    /// Width is measured in `char`s, which matches typical ASCII content; wide
    /// (CJK) characters may be slightly under-counted.
    pub(crate) fn wrapped_line_count(text: &str, width: u16) -> usize {
        let width = width.max(1) as usize;
        let mut lines = 0usize;

        for segment in text.split('\n') {
            if segment.is_empty() {
                lines += 1;
                continue;
            }

            let mut current = 0usize; // columns used on the current line
            let mut segment_lines = 1usize;

            for word in segment.split_inclusive(' ') {
                let word_len = word.chars().count();

                if word_len > width {
                    // A word longer than the line width is broken across lines.
                    if current > 0 {
                        segment_lines += 1;
                    }
                    let full = word_len / width;
                    let rem = word_len % width;
                    segment_lines += full.saturating_sub(if rem == 0 { 1 } else { 0 });
                    current = if rem == 0 { width } else { rem };
                } else if current + word_len > width {
                    segment_lines += 1;
                    current = word_len;
                } else {
                    current += word_len;
                }
            }

            lines += segment_lines;
        }

        lines.max(1)
    }

    /// Truncate `text` to at most `max` display columns, appending an ellipsis
    /// (`…`) when characters are dropped. Width is measured in `char`s.
    pub(crate) fn truncate_with_ellipsis(text: &str, max: usize) -> String {
        if max == 0 {
            return String::new();
        }
        let len = text.chars().count();
        if len <= max {
            return text.to_string();
        }
        if max == 1 {
            return "…".to_string();
        }
        // Keep (max - 1) chars and append the ellipsis.
        let kept: String = text.chars().take(max - 1).collect();
        format!("{kept}…")
    }

    fn render_help(&mut self, frame: &mut Frame) {
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
            Line::from(vec![
                Span::styled("                   ", Style::default().fg(Color::Green)),
                Span::styled(
                    "(uses OSC 52 + system clipboard; works over SSH)",
                    Style::default().fg(Color::DarkGray),
                ),
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

        let area = frame.area();
        let popup_width = (area.width as f32 * 0.7).min(80.0) as u16;

        let content_width = popup_width.saturating_sub(2);
        let total_visual_lines: usize = help_lines
            .iter()
            .map(|line| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                Self::wrapped_line_count(&text, content_width)
            })
            .sum();

        let popup_height =
            (total_visual_lines + 2).min(area.height.saturating_sub(2) as usize) as u16;
        let content_height = popup_height.saturating_sub(2) as usize;

        let max_scroll = total_visual_lines.saturating_sub(content_height);
        let scroll_offset = self.help_scroll.min(max_scroll);
        // Persist the clamp so key handlers don't accumulate past the bottom.
        self.help_scroll = scroll_offset;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let mut title_spans = vec![
            Span::raw(" "),
            Span::styled(
                "Help",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - Keyboard Shortcuts"),
        ];

        if total_visual_lines > content_height {
            let scroll_info = format!(" [{}/{}]", scroll_offset + 1, max_scroll + 1);
            title_spans.push(Span::styled(
                scroll_info,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        title_spans.push(Span::raw(" "));

        let help_paragraph = Paragraph::new(help_lines)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .scroll((scroll_offset as u16, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(title_spans)
                    .title_alignment(Alignment::Center),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(help_paragraph, popup_area);
    }

    fn render_cell_detail(&mut self, frame: &mut Frame) {
        let (cell_value, cell_formula) = self.sheet_data.get_cell(self.cursor_row, self.cursor_col);

        let cell_addr = self.current_cell_address();
        let header = self
            .sheet_data
            .headers()
            .get(self.cursor_col)
            .map(|s| s.as_str())
            .unwrap_or("");

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
            let cell_type = match cell {
                CellValue::Empty => "Empty",
                CellValue::String(_) => "String",
                CellValue::Int(_) => "Integer",
                CellValue::Float(_) => "Float",
                CellValue::Bool(_) => "Boolean",
                CellValue::Error(_) => "Error",
                CellValue::DateTime(_) => "DateTime",
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

            let raw_value = cell.to_raw_string();

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
                    let mut lines = raw_value.lines();
                    let first = lines.next().unwrap_or("");
                    let remaining = raw_value.lines().count().saturating_sub(1);
                    if remaining > 0 {
                        format!("{first} … (+{remaining} more lines)")
                    } else {
                        first.to_string()
                    }
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

            let display_value = cell.to_string();
            if display_value != raw_value && !display_value.contains('\n') {
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

            for line in raw_value.lines() {
                detail_lines.push(Line::from(Span::raw(line.to_string())));
            }
        } else {
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
            "↑↓ to scroll | Any other key to close",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::ITALIC),
        )]));

        let area = frame.area();
        let popup_width = (area.width as f32 * 0.6).min(80.0) as u16;

        // The paragraph wraps with `Wrap { trim: false }`, so a single logical
        // line can occupy several visual rows. The popup is sized — and scrolling
        // is measured — in visual rows, so both the height and `max_scroll` are
        // derived from the wrapped count rather than the logical line count.
        let content_width = popup_width.saturating_sub(2);
        let total_visual_lines: usize = detail_lines
            .iter()
            .map(|line| {
                let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                Self::wrapped_line_count(&text, content_width)
            })
            .sum();

        // Popup height fits the visual content (plus borders), capped to the
        // terminal. `content_height` is then the exact number of visible rows.
        let popup_height =
            (total_visual_lines + 2).min(area.height.saturating_sub(2) as usize) as u16;
        let content_height = popup_height.saturating_sub(2) as usize;

        let max_scroll = total_visual_lines.saturating_sub(content_height);
        let scroll_offset = self.cell_detail_scroll.min(max_scroll);
        // Persist the clamp so key handlers don't accumulate past the bottom.
        self.cell_detail_scroll = scroll_offset;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let mut title_spans = vec![
            Span::raw(" "),
            Span::styled(
                "Cell Details",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(cell_addr.clone(), Style::default().fg(Color::Cyan)),
        ];

        if total_visual_lines > content_height {
            let scroll_info = format!(" [{}/{}]", scroll_offset + 1, max_scroll + 1);
            title_spans.push(Span::styled(
                scroll_info,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        title_spans.push(Span::raw(" "));

        let detail_paragraph = Paragraph::new(detail_lines)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .scroll((scroll_offset as u16, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .title(title_spans)
                    .title_alignment(Alignment::Center),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(detail_paragraph, popup_area);
    }

    fn render_copy_feedback(&self, frame: &mut Frame, message: &str) {
        let area = frame.area();
        let popup_width = (message.len() as u16 + 6).min(60);
        let popup_height = 3;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

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
