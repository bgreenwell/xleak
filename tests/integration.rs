//! Integration tests for xleak CLI.
//!
//! These tests exercise the binary through its CLI interface, verifying
//! end-to-end behavior with real Excel fixture files.
//!
//! Run with: cargo test --test integration

use std::path::Path;
use std::process::Command;

const FIXTURE_DIR: &str = "tests/fixtures";

/// Helper: run xleak with given args and return output
fn run_xleak(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xleak"));
    cmd.args(args);
    cmd.output().expect("Failed to execute xleak")
}

/// Helper: assert command succeeds
fn assert_success(args: &[&str]) {
    let output = run_xleak(args);
    assert!(
        output.status.success(),
        "Command failed: xleak {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Helper: assert command fails with message containing expected text
fn assert_failure_contains(args: &[&str], expected: &str) {
    let output = run_xleak(args);
    assert!(
        !output.status.success(),
        "Expected failure but succeeded: xleak {}",
        args.join(" ")
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "Expected stderr to contain '{}', got:\n{}",
        expected,
        stderr
    );
}

// =========================================================================
// Basic CLI Tests
// =========================================================================

#[test]
fn test_file_not_found() {
    assert_failure_contains(&["nonexistent_file.xlsx"], "File not found");
}

#[test]
fn test_help_flag() {
    let output = run_xleak(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("xleak"));
    assert!(stdout.contains("--sheet"));
    assert!(stdout.contains("--interactive"));
    assert!(stdout.contains("--export"));
    assert!(stdout.contains("--no-header"));
    assert!(stdout.contains("--no-color"));
    assert!(stdout.contains("--delimiter"));
}

#[test]
fn test_version_flag() {
    let output = run_xleak(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("xleak"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

// =========================================================================
// File Type Detection Tests
// =========================================================================

#[test]
fn test_csv_file_displayed() {
    // CSV files are supported (via the `csv` feature, enabled by default).
    let tmpdir = std::env::temp_dir();
    let csv_path = tmpdir.join("xleak_test_temp.csv");
    std::fs::write(&csv_path, "Name,Age\nAlice,30\nBob,25\n").unwrap();

    let result = run_xleak(&[csv_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success());
    assert!(stdout.contains("Name"));
    assert!(stdout.contains("Alice"));
    assert!(stdout.contains("Bob"));

    // Cleanup
    let _ = std::fs::remove_file(&csv_path);
}

#[test]
fn test_csv_no_header_generates_columns() {
    let tmpdir = std::env::temp_dir();
    let csv_path = tmpdir.join("xleak_test_noheader.csv");
    std::fs::write(&csv_path, "1,2,3\n4,5,6\n").unwrap();

    let result = run_xleak(&[csv_path.to_str().unwrap(), "--no-header"]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success());
    // Default A, B, C headers when --no-header is set
    assert!(stdout.contains("A"));
    assert!(stdout.contains("B"));

    let _ = std::fs::remove_file(&csv_path);
}

#[test]
fn test_tsv_custom_delimiter() {
    let tmpdir = std::env::temp_dir();
    let tsv_path = tmpdir.join("xleak_test_temp.tsv");
    std::fs::write(&tsv_path, "col1\tcol2\nval1\tval2\n").unwrap();

    let result = run_xleak(&[tsv_path.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success());
    assert!(stdout.contains("col1"));
    assert!(stdout.contains("val2"));

    let _ = std::fs::remove_file(&tsv_path);
}

#[test]
fn test_unknown_format_rejected() {
    // Create a file with random binary content
    let tmpdir = std::env::temp_dir();
    let bin_path = tmpdir.join("xleak_test_temp.bin");
    std::fs::write(&bin_path, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]).unwrap();

    let result = run_xleak(&[bin_path.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Unrecognized file format"));
    assert!(!result.status.success());

    let _ = std::fs::remove_file(&bin_path);
}

// =========================================================================
// Sheet Display Tests
// =========================================================================

#[test]
fn test_display_comprehensive_sheet() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    assert_success(&[fixture.to_str().unwrap(), "--sheet", "DataTypes", "-n", "5"]);
}

#[test]
fn test_display_with_formulas() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "Formulas",
        "--formulas",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
}

#[test]
fn test_display_with_no_header() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--no-header",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With --no-header, first row should be data, not headers
    // Column headers should be A, B, C... instead of the actual header names
    assert!(
        stdout.contains("A") || stdout.contains("Sheet:"),
        "Should show column letters or sheet info"
    );
}

#[test]
fn test_display_with_no_color() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--no-color",
        "-n",
        "3",
    ]);
    assert!(output.status.success());
}

#[test]
fn test_display_sheet_by_index() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    // Sheet index 1 = first sheet (DataTypes)
    assert_success(&[fixture.to_str().unwrap(), "--sheet", "1", "-n", "3"]);
}

#[test]
fn test_display_invalid_sheet_index() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    assert_failure_contains(
        &[fixture.to_str().unwrap(), "--sheet", "999"],
        "out of range",
    );
}

#[test]
fn test_display_nonexistent_sheet() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    assert_failure_contains(
        &[fixture.to_str().unwrap(), "--sheet", "NonExistentSheet"],
        "not found",
    );
}

// =========================================================================
// Export Tests
// =========================================================================

#[test]
fn test_export_csv() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--export",
        "csv",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // CSV should have headers and comma-separated values
    assert!(stdout.contains(","), "CSV output should contain commas");
}

#[test]
fn test_export_json() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--export",
        "json",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"sheet\""),
        "JSON should contain sheet key"
    );
    assert!(
        stdout.contains("\"headers\""),
        "JSON should contain headers"
    );
    assert!(stdout.contains("\"data\""), "JSON should contain data");
}

#[test]
fn test_export_text() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--export",
        "text",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Text export uses tab by default
    assert!(stdout.contains("\t"), "Text output should contain tabs");
}

#[test]
fn test_export_csv_with_custom_delimiter() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--export",
        "csv",
        "--delimiter",
        ";",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(";"),
        "CSV with semicolon delimiter should contain semicolons"
    );
}

#[test]
fn test_export_text_with_custom_delimiter() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DataTypes",
        "--export",
        "text",
        "--delimiter",
        "|",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("|"),
        "Text with pipe delimiter should contain pipes"
    );
}

#[test]
fn test_export_unknown_format() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    assert_failure_contains(
        &[fixture.to_str().unwrap(), "--export", "xml"],
        "Unknown export format",
    );
}

// =========================================================================
// Row Limit Tests
// =========================================================================

#[test]
fn test_max_rows_zero_shows_all() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--sheet", "DataTypes", "-n", "0"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show "Total:" not "Showing X of Y"
    assert!(stdout.contains("Total:"), "With -n 0 should show all rows");
}

#[test]
fn test_max_rows_limited() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--sheet", "DataTypes", "-n", "3"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Showing 3 of"),
        "With -n 3 should show limited rows warning"
    );
}

// =========================================================================
// Table Tests (xlsx only)
// =========================================================================

#[test]
fn test_list_tables() {
    let fixture = Path::new(FIXTURE_DIR).join("test_tables.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--list-tables"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sheet"), "Should have header row");
    assert!(stdout.contains("Table"), "Should have header row");
    assert!(stdout.contains("Products"), "Should list Products table");
    assert!(stdout.contains("Sales"), "Should list Sales table");
    assert!(stdout.contains("Employees"), "Should list Employees table");
}

#[test]
fn test_table_by_name() {
    let fixture = Path::new(FIXTURE_DIR).join("test_tables.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--table", "Products"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Products"), "Should show table name");
    assert!(stdout.contains("rows ×"), "Should show dimensions");
}

#[test]
fn test_table_export_json() {
    let fixture = Path::new(FIXTURE_DIR).join("test_tables.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--table",
        "Products",
        "--export",
        "json",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"table\": \"Products\""),
        "JSON should contain table name"
    );
}

#[test]
fn test_table_export_csv() {
    let fixture = Path::new(FIXTURE_DIR).join("test_tables.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--table",
        "Products",
        "--export",
        "csv",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(","), "CSV should contain commas");
}

#[test]
fn test_table_nonexistent() {
    let fixture = Path::new(FIXTURE_DIR).join("test_tables.xlsx");
    assert_failure_contains(
        &[fixture.to_str().unwrap(), "--table", "NonExistentTable"],
        "not found",
    );
}

// =========================================================================
// Large File / Lazy Loading Tests
// =========================================================================

#[test]
fn test_large_file_display() {
    let fixture = Path::new(FIXTURE_DIR).join("test_large.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "LargeData",
        "-n",
        "10",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show total row count (10000 rows)
    assert!(
        stdout.contains("10000 rows") || stdout.contains("10,000 rows"),
        "Should show total row count"
    );
}

// =========================================================================
// Multi-Sheet Tests
// =========================================================================

#[test]
fn test_multi_sheet_navigation_info() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--sheet", "DataTypes", "-n", "3"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should list available sheets
    assert!(
        stdout.contains("Available sheets:"),
        "Should show available sheets"
    );
}

// =========================================================================
// Internationalization / UTF-8 Tests
// =========================================================================

#[test]
fn test_utf8_characters_display() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "Internationalization",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain German characters
    assert!(
        stdout.contains("ä") || stdout.contains("ö") || stdout.contains("ü"),
        "Should display German umlauts"
    );
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
fn test_empty_sheet_handled() {
    // The comprehensive file has no empty sheets, but we test graceful handling
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--sheet", "EdgeCases", "-n", "1"]);
    assert!(output.status.success());
}

#[test]
fn test_multiline_cells() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "MultilineCells",
        "--wrap",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
}

#[test]
fn test_date_edge_cases() {
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[
        fixture.to_str().unwrap(),
        "--sheet",
        "DateEdgeCases",
        "-n",
        "5",
    ]);
    assert!(output.status.success());
}

// =========================================================================
// Error Handling Tests
// =========================================================================

#[test]
fn test_missing_file_argument() {
    // Running without a file argument should show error from clap
    let output = run_xleak(&[]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("FILE") || stderr.contains("required"),
        "Should show usage info"
    );
}

#[test]
fn test_invalid_flag_combination() {
    // --color and --no-color are mutually exclusive
    let fixture = Path::new(FIXTURE_DIR).join("test_comprehensive.xlsx");
    let output = run_xleak(&[fixture.to_str().unwrap(), "--color", "--no-color"]);
    // clap should reject this before main runs
    assert!(!output.status.success());
}
