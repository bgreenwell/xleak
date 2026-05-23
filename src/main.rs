use anyhow::{Context, Result};
use clap::Parser;
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Row, Table, Width,
};
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

    /// Enable horizontal scrolling in TUI mode (auto-size columns)
    #[arg(short = 'H', long)]
    horizontal_scroll: bool,

    /// Path to custom config file (default: $XDG_CONFIG_HOME/xleak/config.toml)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// List all Excel tables in the workbook (.xlsx only)
    #[arg(long)]
    list_tables: bool,

    /// Extract a specific Excel table by name (.xlsx only)
    #[arg(short = 't', long, value_name = "TABLE")]
    table: Option<String>,
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
            match format {
                "json" => export_table_json(&table_data)?,
                "csv" => export_table_csv(&table_data)?,
                "text" => export_table_text(&table_data)?,
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
        display_table_data(&table_data, cli.max_rows)?;
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
        tui::run_tui(wb, &sheet_name, &config, cli.horizontal_scroll)?;
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

/// Display table data in terminal (default behavior)
fn display_table_data(table: &workbook::TableData, max_rows: usize) -> Result<()> {
    println!("\n╔═════════════════════════════════════════════════╗");
    println!("║  xleak - Excel Table Viewer                     ║");
    println!("╚═════════════════════════════════════════════════╝");
    println!();
    println!("Table: {} (from sheet: {})", table.name, table.sheet_name);
    println!(
        "{} rows × {} columns",
        table.rows.len(),
        table.headers.len()
    );
    println!();

    let max_width = 30u16;
    let mut table_obj = Table::new();
    table_obj.set_content_arrangement(ContentArrangement::Dynamic);
    table_obj.set_width(
        (table.headers.len() as u16)
            .saturating_mul(max_width + 3)
            .max(max_width),
    );

    let mut header_row = Row::new();
    for h in &table.headers {
        header_row.add_cell(
            Cell::new(h)
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
        );
    }
    table_obj.set_header(header_row);
    table_obj.set_constraints(
        (0..table.headers.len()).map(|_| ColumnConstraint::UpperBoundary(Width::Fixed(max_width))),
    );

    let rows_to_show = if max_rows == 0 {
        table.rows.len()
    } else {
        std::cmp::min(max_rows, table.rows.len())
    };

    for row in table.rows.iter().take(rows_to_show) {
        let mut table_row = Row::new();
        for cell in row {
            let cell_obj = match cell {
                workbook::CellValue::Int(_) | workbook::CellValue::Float(_) => {
                    Cell::new(cell.to_string()).set_alignment(CellAlignment::Right)
                }
                workbook::CellValue::Bool(_) => {
                    Cell::new(cell.to_string()).set_alignment(CellAlignment::Center)
                }
                workbook::CellValue::Error(_) => Cell::new(cell.to_string())
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Red),
                _ => Cell::new(cell.to_string()).set_alignment(CellAlignment::Left),
            };
            table_row.add_cell(cell_obj);
        }
        table_obj.add_row(table_row);
    }

    println!("{}", table_obj);

    println!();
    if rows_to_show < table.rows.len() {
        println!(
            "⚠️  Showing {} of {} rows (use -n 0 to show all)",
            rows_to_show,
            table.rows.len()
        );
    } else {
        println!(
            "Total: {} rows × {} columns",
            table.rows.len(),
            table.headers.len()
        );
    }

    println!();
    Ok(())
}

/// Export table data as JSON
fn export_table_json(table: &workbook::TableData) -> Result<()> {
    println!("{{");
    println!("  \"table\": \"{}\",", table.name);
    println!("  \"sheet\": \"{}\",", table.sheet_name);
    println!("  \"columns\": {},", table.headers.len());
    println!("  \"rows\": {},", table.rows.len());
    println!("  \"headers\": [");
    for (i, header) in table.headers.iter().enumerate() {
        let comma = if i < table.headers.len() - 1 { "," } else { "" };
        println!("    \"{header}\"{comma}");
    }
    println!("  ],");
    println!("  \"data\": [");

    for (i, row) in table.rows.iter().enumerate() {
        print!("    [");
        for (j, cell) in row.iter().enumerate() {
            let value = match cell {
                workbook::CellValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
                workbook::CellValue::Int(i) => i.to_string(),
                workbook::CellValue::Float(f) => f.to_string(),
                workbook::CellValue::Bool(b) => b.to_string(),
                workbook::CellValue::Empty => "null".to_string(),
                _ => format!("\"{cell}\""),
            };
            print!("{value}");
            if j < row.len() - 1 {
                print!(", ");
            }
        }
        let comma = if i < table.rows.len() - 1 { "," } else { "" };
        println!("]{comma}");
    }

    println!("  ]");
    println!("}}");

    Ok(())
}

/// Export table data as CSV
fn export_table_csv(table: &workbook::TableData) -> Result<()> {
    // Print headers
    println!("{}", table.headers.join(","));

    // Print rows
    for row in &table.rows {
        let row_str: Vec<String> = row
            .iter()
            .map(|cell| {
                let val = cell.to_raw_string();
                // Quote if contains comma or quotes
                if val.contains(',') || val.contains('"') {
                    format!("\"{}\"", val.replace('"', "\"\""))
                } else {
                    val
                }
            })
            .collect();
        println!("{}", row_str.join(","));
    }

    Ok(())
}

/// Export table data as plain text (tab-separated)
fn export_table_text(table: &workbook::TableData) -> Result<()> {
    // Print headers
    println!("{}", table.headers.join("\t"));

    // Print rows
    for row in &table.rows {
        let row_str: Vec<String> = row.iter().map(|cell| cell.to_raw_string()).collect();
        println!("{}", row_str.join("\t"));
    }

    Ok(())
}
