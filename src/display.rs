use crate::workbook::{CellValue, SheetData};
use anyhow::Result;
use comfy_table::{
    Attribute, Cell, CellAlignment, ColumnConstraint, ContentArrangement, Row, Table, Width,
};

/// Format a cell value with width limiting
fn format_cell_value(value: &str, max_width: usize, wrap: bool) -> String {
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_string();
    }

    if wrap {
        // Return full text; comfy-table handles the multi-line wrap based on column width
        value.to_string()
    } else {
        // Truncate with "..."
        if max_width > 3 {
            let truncated: String = value.chars().take(max_width - 3).collect();
            format!("{}...", truncated)
        } else {
            value.chars().take(max_width).collect()
        }
    }
}

/// Display sheet data as a formatted table in the terminal
pub fn display_table(
    data: &SheetData,
    sheet_name: &str,
    max_rows: usize,
    all_sheets: &[&str],
    max_width: usize,
    wrap: bool,
    show_formulas: bool,
) -> Result<()> {
    // Print header info
    println!("\n╔═════════════════════════════════════════════════╗");
    println!("║  xleak - Excel File Viewer                      ║");
    println!("╚═════════════════════════════════════════════════╝");
    println!();
    println!(
        "Sheet: {} ({} rows × {} columns)",
        sheet_name, data.height, data.width
    );

    if all_sheets.len() > 1 {
        println!("Available sheets: {}", all_sheets.join(", "));
    }
    println!();

    if data.rows.is_empty() {
        println!("⚠️  Sheet is empty");
        return Ok(());
    }

    let mut table = Table::new();
    if wrap {
        let width = (data.width as u16)
            .saturating_mul(max_width as u16 + 3)
            .max(max_width as u16);
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(width);
    }

    let mut header_row = Row::new();
    for h in &data.headers {
        let formatted = format_cell_value(h, max_width, wrap);
        header_row.add_cell(Cell::new(formatted).add_attribute(Attribute::Bold));
    }
    table.set_header(header_row);
    table.set_constraints(
        (0..data.width).map(|_| ColumnConstraint::UpperBoundary(Width::Fixed(max_width as u16))),
    );

    // Add data rows (limit if needed)
    let rows_to_show = if max_rows == 0 {
        data.rows.len()
    } else {
        std::cmp::min(max_rows, data.rows.len())
    };

    for (row_idx, row) in data.rows.iter().enumerate().take(rows_to_show) {
        let mut table_row = Row::new();
        for (col_idx, cell) in row.iter().enumerate() {
            let value = if show_formulas {
                data.formulas
                    .get(row_idx)
                    .and_then(|formula_row| formula_row.get(col_idx))
                    .and_then(|f| f.as_ref())
                    .cloned()
                    .unwrap_or_else(|| cell.to_string())
            } else {
                cell.to_string()
            };

            let formatted = format_cell_value(&value, max_width, wrap);
            let mut cell_obj = Cell::new(formatted);

            // Alignment mapping
            if !show_formulas {
                cell_obj = match cell {
                    CellValue::Int(_) | CellValue::Float(_) => {
                        cell_obj.set_alignment(CellAlignment::Right)
                    }
                    CellValue::Bool(_) => cell_obj.set_alignment(CellAlignment::Center),
                    CellValue::Error(_) => cell_obj.set_alignment(CellAlignment::Center),
                    _ => cell_obj.set_alignment(CellAlignment::Left),
                };
            } else {
                cell_obj = cell_obj.set_alignment(CellAlignment::Left);
            }
            table_row.add_cell(cell_obj);
        }
        table.add_row(table_row);
    }

    println!("{}", table);

    // Show row count summary
    println!();
    if rows_to_show < data.rows.len() {
        println!(
            "⚠️  Showing {} of {} rows (use -n 0 to show all)",
            rows_to_show,
            data.rows.len()
        );
    } else {
        println!("Total: {} rows × {} columns", data.height, data.width);
    }

    println!();
    Ok(())
}

/// Export data as CSV to stdout
pub fn export_csv(data: &SheetData) -> Result<()> {
    // Print headers
    println!("{}", data.headers.join(","));

    // Print rows
    for row in &data.rows {
        let row_str: Vec<String> = row
            .iter()
            .map(|cell| {
                let val = cell.to_string();
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

/// Export data as JSON to stdout
pub fn export_json(data: &SheetData, sheet_name: &str) -> Result<()> {
    println!("{{");
    println!("  \"sheet\": \"{sheet_name}\",");
    println!("  \"rows\": {},", data.height);
    println!("  \"columns\": {},", data.width);
    println!("  \"headers\": [");
    for (i, header) in data.headers.iter().enumerate() {
        let comma = if i < data.headers.len() - 1 { "," } else { "" };
        println!("    \"{header}\"{comma}");
    }
    println!("  ],");
    println!("  \"data\": [");

    for (i, row) in data.rows.iter().enumerate() {
        print!("    [");
        for (j, cell) in row.iter().enumerate() {
            let value = match cell {
                CellValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
                CellValue::Int(i) => i.to_string(),
                CellValue::Float(f) => f.to_string(),
                CellValue::Bool(b) => b.to_string(),
                CellValue::Empty => "null".to_string(),
                _ => format!("\"{cell}\""),
            };
            print!("{value}");
            if j < row.len() - 1 {
                print!(", ");
            }
        }
        let comma = if i < data.rows.len() - 1 { "," } else { "" };
        println!("]{comma}");
    }

    println!("  ]");
    println!("}}");

    Ok(())
}

/// Export data as plain text to stdout
pub fn export_text(data: &SheetData) -> Result<()> {
    // Headers
    println!("{}", data.headers.join("\t"));

    // Data rows
    for row in &data.rows {
        let row_str: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
        println!("{}", row_str.join("\t"));
    }

    Ok(())
}
