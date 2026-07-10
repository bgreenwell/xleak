use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "xleak")]
#[command(author, version, about = "Expose Excel files in your terminal - no Microsoft Excel required", long_about = None)]
pub struct Cli {
    /// Path to the Excel file (.xlsx, .xls, .xlsm, .ods)
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Sheet name or index to display (default: first sheet)
    #[arg(short, long, value_name = "SHEET")]
    pub sheet: Option<String>,

    /// Export format: csv, json, text
    #[arg(short, long, value_name = "FORMAT")]
    pub export: Option<String>,

    /// Maximum number of rows to display (0 = all)
    #[arg(short = 'n', long, default_value = "50")]
    pub max_rows: usize,

    /// Show formulas instead of values
    #[arg(short, long)]
    pub formulas: bool,

    /// Maximum column width in characters (default: 30)
    #[arg(short = 'w', long, default_value = "30")]
    pub max_width: usize,

    /// Wrap long text instead of truncating
    #[arg(long)]
    pub wrap: bool,

    /// Interactive TUI mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Enable horizontal scrolling in TUI mode (auto-size columns)
    #[arg(short = 'H', long)]
    pub horizontal_scroll: bool,

    /// Path to custom config file (default: $XDG_CONFIG_HOME/xleak/config.toml)
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// List all Excel tables in the workbook (.xlsx only)
    #[arg(long)]
    pub list_tables: bool,

    /// Extract a specific Excel table by name (.xlsx only)
    #[arg(short = 't', long, value_name = "TABLE")]
    pub table: Option<String>,

    /// Treat first row as data, not headers
    #[arg(long)]
    pub no_header: bool,

    /// Hide the column-letter row (A, B, C, ...) in interactive mode
    #[arg(long)]
    pub no_column_id: bool,

    /// Hide the row-number column in interactive mode
    #[arg(long)]
    pub no_row_id: bool,

    /// Disable colored output (useful for piping)
    #[arg(long)]
    pub no_color: bool,

    /// Force colored output even when piped
    #[arg(long, conflicts_with = "no_color")]
    pub color: bool,

    /// Delimiter for CSV/text export (default: ',' for CSV, '\t' for text)
    #[arg(short = 'd', long, value_name = "CHAR")]
    pub delimiter: Option<String>,

    /// Field delimiter when reading CSV/TSV input (default: ',' for .csv, '\t' for .tsv)
    #[arg(long, value_name = "CHAR")]
    pub csv_delimiter: Option<String>,
}
