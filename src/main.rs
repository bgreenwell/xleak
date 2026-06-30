mod cli;
mod config;
#[cfg(feature = "csv")]
mod csv;
mod display;
mod tui;
mod utils;
mod workbook;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::io::IsTerminal;
use utils::detect_file_type;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = config::Config::load(cli.config.clone())?;

    // Determine color output: --color forces color, --no-color disables it,
    // otherwise auto-detect based on whether stdout is a terminal
    let use_color = if cli.no_color {
        false
    } else if cli.color {
        true
    } else {
        std::io::stdout().is_terminal()
    };

    // Validate file exists
    if !cli.file.exists() {
        anyhow::bail!("File not found: {}", cli.file.display());
    }

    // Detect file type and provide helpful error messages for unsupported formats
    let file_type = detect_file_type(&cli.file)?;
    match file_type {
        utils::FileType::Csv => {
            #[cfg(not(feature = "csv"))]
            anyhow::bail!(
                "This file appears to be a CSV/TSV file, but xleak was built \
                 without CSV support.\n\
                 \n\
                 Rebuild with the `csv` feature enabled: cargo install xleak --features csv"
            );
            #[cfg(feature = "csv")]
            {
                // CSV is supported — handled below when opening the workbook.
            }
        }
        utils::FileType::Unknown => {
            anyhow::bail!(
                "Unrecognized file format: '{}'\n\
                 \n\
                 Supported formats: .xlsx, .xls, .xlsm, .xlsb, .ods, .csv, .tsv\n\
                 \n\
                 If this is a valid Excel file, please report this as a bug.",
                cli.file.display()
            );
        }
        utils::FileType::Xlsx | utils::FileType::Xls => {
            // Supported formats, proceed
        }
    }

    // Load the workbook (Excel via calamine, or CSV/TSV into a single sheet)
    let mut wb = match file_type {
        #[cfg(feature = "csv")]
        utils::FileType::Csv => {
            let delimiter = parse_delimiter_byte(cli.csv_delimiter.as_deref())?;
            workbook::Workbook::open_csv(&cli.file, delimiter).context("Failed to open CSV file")?
        }
        _ => workbook::Workbook::open(&cli.file).context("Failed to open Excel file")?,
    };

    // Handle table operations (xlsx only)
    if cli.list_tables {
        wb.load_tables()?;
        let table_names = wb.table_names()?;

        if table_names.is_empty() {
            println!("No tables found in workbook");
        } else {
            println!("Sheet\tTable");
            println!("-----\t-----");
            for table_name in &table_names {
                // Get which sheet this table is in
                let sheet_names = wb.sheet_names();
                for sheet in &sheet_names {
                    let tables_in_sheet = wb.table_names_in_sheet(sheet)?;
                    if tables_in_sheet.contains(table_name) {
                        println!("{sheet}\t{table_name}");
                        break;
                    }
                }
            }
        }
        return Ok(());
    }

    if let Some(ref table_name) = cli.table {
        wb.load_tables()?;
        let table_data = wb.table_by_name(table_name)?;

        // Handle export formats (non-interactive)
        if let Some(format) = cli.export.as_deref() {
            let delimiter = cli.delimiter.as_deref().unwrap_or(match format {
                "csv" => ",",
                "text" => "\t",
                _ => ",",
            });
            match format {
                "json" => display::export_table_json(&table_data)?,
                "csv" => display::export_table_csv(&table_data, delimiter)?,
                "text" => display::export_table_text(&table_data, delimiter)?,
                _ => anyhow::bail!("Unknown export format: {format}. Use: csv, json, or text"),
            }
            return Ok(());
        }

        if cli.interactive {
            anyhow::bail!(
                "Interactive mode (-i) is not supported with --table.\n\
                 \n\
                 Options:\n\
                 • View table in terminal: xleak file.xlsx --table \"{table_name}\"\n\
                 • View full sheet in TUI: xleak file.xlsx --sheet \"{}\" -i",
                table_data.sheet_name
            );
        }

        // Default: display table in terminal
        display::display_table_data(&table_data, cli.max_rows, cli.max_width, use_color)?;
        return Ok(());
    }

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
        tui::run_tui(
            wb,
            &sheet_name,
            &config,
            cli.horizontal_scroll,
            cli.no_header,
            cli.no_column_id,
            cli.no_row_id,
        )?;
    } else {
        // Load the sheet data for non-interactive modes
        let data = wb
            .load_sheet(&sheet_name, cli.no_header)
            .with_context(|| format!("Failed to load sheet '{sheet_name}'"))?;
        match cli.export.as_deref() {
            Some("csv") => {
                let delimiter = cli.delimiter.as_deref().unwrap_or(",");
                display::export_csv(&data, delimiter)?;
            }
            Some("json") => {
                display::export_json(&data, &sheet_name)?;
            }
            Some("text") => {
                let delimiter = cli.delimiter.as_deref().unwrap_or("\t");
                display::export_text(&data, delimiter)?;
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
                    use_color,
                )?;
            }
        }
    }

    Ok(())
}

/// Parse a user-supplied CSV delimiter string into a single byte.
/// Accepts a single character, or the escape sequence `\t` for a tab.
#[cfg(feature = "csv")]
fn parse_delimiter_byte(delimiter: Option<&str>) -> Result<Option<u8>> {
    let Some(d) = delimiter else {
        return Ok(None);
    };

    let resolved = match d {
        "\\t" | "\t" => b'\t',
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii() => c as u8,
                _ => anyhow::bail!(
                    "Invalid --csv-delimiter '{d}': expected a single ASCII character or '\\t'"
                ),
            }
        }
    };

    Ok(Some(resolved))
}
