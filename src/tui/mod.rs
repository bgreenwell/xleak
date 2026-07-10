mod clipboard;
mod event;
mod rendering;
mod state;
mod theme;

pub use event::run_tui;

#[cfg(test)]
mod tests {
    use super::state::TuiState;

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
        assert_eq!(TuiState::parse_cell_address("!@#"), None);
        assert_eq!(TuiState::parse_cell_address("A-1"), None);
    }

    #[test]
    fn test_parse_cell_address_large_column() {
        assert_eq!(TuiState::parse_cell_address("BA1"), Some((52, 0)));
        assert_eq!(TuiState::parse_cell_address("ZZ1"), Some((701, 0)));
    }

    #[test]
    fn test_first_data_sheet_row() {
        // With a header row (default), the header is sheet row 1 and the first
        // data row is sheet row 2.
        assert_eq!(TuiState::first_data_sheet_row_for(false), 2);
        // With --no-header, the first row is data, so it is sheet row 1.
        assert_eq!(TuiState::first_data_sheet_row_for(true), 1);
    }

    #[test]
    fn test_wrapped_line_count() {
        // Fits on one line.
        assert_eq!(TuiState::wrapped_line_count("hello", 80), 1);
        // Empty text is still one line.
        assert_eq!(TuiState::wrapped_line_count("", 80), 1);
        // Wraps onto a second line when too wide.
        assert_eq!(TuiState::wrapped_line_count("aaa bbb", 4), 2);
        // A single word longer than the width is broken across lines.
        assert_eq!(TuiState::wrapped_line_count("aaaaaaaaaa", 5), 2);
        assert_eq!(TuiState::wrapped_line_count("aaaaaaaaaaaa", 5), 3);
        // Explicit newlines are respected.
        assert_eq!(TuiState::wrapped_line_count("a\nb\nc", 80), 3);
        // Zero width is treated as width 1 (no panic).
        assert!(TuiState::wrapped_line_count("ab", 0) >= 1);
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        // Fits without truncation.
        assert_eq!(TuiState::truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(TuiState::truncate_with_ellipsis("hello", 5), "hello");
        // Truncated with ellipsis.
        assert_eq!(TuiState::truncate_with_ellipsis("hello", 4), "hel…");
        assert_eq!(TuiState::truncate_with_ellipsis("hello", 1), "…");
        // Zero width yields empty.
        assert_eq!(TuiState::truncate_with_ellipsis("hello", 0), "");
    }
}
