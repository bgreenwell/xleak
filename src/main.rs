use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod config;
mod display;
mod tui;
mod workbook;

#[derive(Parser)]
#[command(name = "xleak")]
#[command(author, version, about = "Expose Excel files in your terminal - no Microsoft Excel required", long_about = None)]
struct Cli {
    /// Path to the Excel file (.xlsx, .xls, .xlsm, .ods)
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Sheet name or index to display (default: first sheet)
    #[arg(short, long, value_name = "SHEET")]
    sheet: Option<String>,

    /// Export format: csv, json, text
    #[arg(short, long, value_name = "FORMAT")]
    export: Option<String>,

    /// Maximum number of rows to display (0 = all)
    #[arg(short = 'n', long, default_value = "50")]
    max_rows: usize,

    /// Show formulas instead of values
    #[arg(short, long)]
    formulas: bool,

    /// Maximum column width in characters (default: 30)
    #[arg(short = 'w', long, default_value = "30")]
    max_width: usize,

    /// Wrap long text instead of truncating
    #[arg(long)]
    wrap: bool,

    /// Interactive TUI mode
    #[arg(short, long)]
    interactive: bool,

    /// Path to custom config file (default: $XDG_CONFIG_HOME/xleak/config.toml)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = config::Config::load(cli.config.clone())?;

    // Validate file exists
    if !cli.file.exists() {
        anyhow::bail!("File not found: {}", cli.file.display());
    }

    // Load the workbook
    let mut wb = workbook::Workbook::open(&cli.file).context("Failed to open Excel file")?;

    // Get sheet names (clone to avoid borrow issues)
    let sheet_names = wb.sheet_names();
    if sheet_names.is_empty() {
        anyhow::bail!("No sheets found in workbook");
    }

    // Determine which sheet to display
    let sheet_name = if let Some(ref name) = cli.sheet {
        // Try as name first
        if sheet_names.iter().any(|s| s == name) {
            name.clone()
        } else {
            // Try as index
            if let Ok(idx) = name.parse::<usize>() {
                if idx > 0 && idx <= sheet_names.len() {
                    sheet_names[idx - 1].clone()
                } else {
                    anyhow::bail!("Sheet index {} out of range (1-{})", idx, sheet_names.len());
                }
            } else {
                anyhow::bail!(
                    "Sheet '{}' not found. Available sheets: {}",
                    name,
                    sheet_names.join(", ")
                );
            }
        }
    } else {
        sheet_names[0].clone()
    };

    // Display, export, or run TUI
    if cli.interactive {
        // Interactive TUI mode - pass the workbook so it can switch sheets
        tui::run_tui(wb, &sheet_name, &config)?;
    } else {
        // Load the sheet data for non-interactive modes
        let data = wb
            .load_sheet(&sheet_name)
            .with_context(|| format!("Failed to load sheet '{sheet_name}'"))?;
        match cli.export.as_deref() {
            Some("csv") => {
                display::export_csv(&data)?;
            }
            Some("json") => {
                display::export_json(&data, &sheet_name)?;
            }
            Some("text") => {
                display::export_text(&data)?;
            }
            Some(format) => {
                anyhow::bail!("Unknown export format: {format}. Use: csv, json, or text");
            }
            None => {
                // Non-interactive display
                let sheet_names_refs: Vec<&str> = sheet_names.iter().map(|s| s.as_str()).collect();
                display::display_table(
                    &data,
                    &sheet_name,
                    cli.max_rows,
                    &sheet_names_refs,
                    cli.max_width,
                    cli.wrap,
                    cli.formulas,
                )?;
            }
        }
    }

    Ok(())
}
