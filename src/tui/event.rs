use crate::workbook::Workbook;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;

use super::state::TuiState;

impl TuiState {
    /// Check if a key press matches a configured action
    pub fn key_matches(
        &self,
        code: KeyCode,
        modifiers: crossterm::event::KeyModifiers,
        action: &str,
    ) -> bool {
        if let Some((expected_code, expected_mods)) = self.config.get_keybinding(action) {
            if code != expected_code {
                return false;
            }
            // Some terminals omit SHIFT for shifted symbol characters (e.g. '?', '$')
            // because the shift is already encoded in the character itself. Strip SHIFT
            // from both sides for non-alphabetic chars so bindings work consistently.
            if let KeyCode::Char(c) = code
                && !c.is_alphabetic()
            {
                let strip = crossterm::event::KeyModifiers::SHIFT;
                return (modifiers - strip) == (expected_mods - strip);
            }
            modifiers == expected_mods
        } else {
            false
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            // If help is showing, handle scrolling or close
            if self.show_help {
                match code {
                    KeyCode::Up => {
                        self.help_scroll = self.help_scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        self.help_scroll = self.help_scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        self.help_scroll = self.help_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        self.help_scroll = self.help_scroll.saturating_add(10);
                    }
                    KeyCode::Home => {
                        self.help_scroll = 0;
                    }
                    _ => {
                        self.show_help = false;
                        self.help_scroll = 0;
                    }
                }
                return;
            }

            // If cell detail is showing, handle scrolling or close
            if self.show_cell_detail {
                match code {
                    KeyCode::Up => {
                        self.cell_detail_scroll = self.cell_detail_scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        self.cell_detail_scroll = self.cell_detail_scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        self.cell_detail_scroll = self.cell_detail_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        self.cell_detail_scroll = self.cell_detail_scroll.saturating_add(10);
                    }
                    KeyCode::Home => {
                        self.cell_detail_scroll = 0;
                    }
                    _ => {
                        self.show_cell_detail = false;
                        self.cell_detail_scroll = 0;
                    }
                }
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
                        self.search_mode = false;
                    }
                    KeyCode::Esc => {
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
                        self.perform_jump();
                    }
                    KeyCode::Esc => {
                        self.jump_mode = false;
                        self.jump_input.clear();
                    }
                    _ => {}
                }
                return;
            }

            // Normal navigation and commands
            if self.key_matches(code, modifiers, "quit") {
                self.should_quit = true;
            } else if self.key_matches(code, modifiers, "help") {
                self.show_help = true;
                self.help_scroll = 0;
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
                self.cell_detail_scroll = 0;
            } else if self.key_matches(code, modifiers, "next_sheet") {
                let _ = self.switch_to_next_sheet();
            } else if self.key_matches(code, modifiers, "prev_sheet") || code == KeyCode::BackTab {
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
                if !self.search_matches.is_empty() {
                    self.clear_search();
                } else {
                    self.should_quit = true;
                }
            }
        }
    }
}

/// Undo raw mode and the alternate screen. Safe to call more than once.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
}

/// RAII guard that restores the terminal when dropped, so early `?` returns
/// and panic unwinds can't leave the user's shell in raw mode (#58).
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Run the TUI application
pub fn run_tui(
    workbook: Workbook,
    sheet_name: &str,
    config: &crate::config::Config,
    horizontal_scroll: bool,
    no_header: bool,
    no_column_id: bool,
    no_row_id: bool,
) -> Result<()> {
    use std::io::IsTerminal;
    if !io::stdout().is_terminal() {
        anyhow::bail!(
            "Interactive mode requires a terminal (TTY). \
             Your output is redirected or not connected to a terminal.\n\
             Hint: Run this command directly in your terminal, not through pipes or automation."
        );
    }

    // Restore the terminal before the panic message prints, so it lands on a
    // readable screen instead of vanishing into the alternate buffer.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    // Setup terminal. The guard covers every exit from here on: normal
    // return, `?` on a failed sheet load, or a panic unwind.
    enable_raw_mode().context("Failed to enable terminal raw mode. Is this a proper TTY?")?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen mode")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal backend")?;

    // Create app state
    let mut app = TuiState::new(
        workbook,
        sheet_name,
        config,
        horizontal_scroll,
        no_header,
        no_column_id,
        no_row_id,
    )?;

    // Main event loop
    run_event_loop(&mut terminal, &mut app)
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiState,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            app.render(f);
        })?;

        if event::poll(Duration::from_millis(100))? {
            let event = event::read()?;
            app.handle_event(event);
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
