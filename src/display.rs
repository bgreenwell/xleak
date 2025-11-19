use crate::workbook::{CellValue, SheetData};
use anyhow::Result;
use prettytable::{format, Cell, Row, Table};

/// Format a cell value with width limiting
fn format_cell_value(value: &str, max_width: usize, wrap: bool) -> String {
    if value.len() <= max_width {
        return value.to_string();
    }

    let num_chars_to_take = 20;

    if wrap {
        // For now, wrapping is not fully implemented with prettytable
        // We'll truncate with a note. Full wrapping would require custom rendering.
        // Future: implement multi-line cell support
        if max_width > num_chars_to_take {
            format!("{}...", &value[..max_width - num_chars_to_take])
        } else {
            value[..max_width].to_string()
        }
    } else {
        // Tìm chỉ mục byte sau ký tự thứ 10
        let end_byte_index = value
            .char_indices()
            .nth(num_chars_to_take)
            .map(|(i, _)| i)
            // Nếu chuỗi ngắn hơn 10 ký tự, lấy toàn bộ chiều dài byte
            .unwrap_or(value.len());
        // Truncate with "..."
        if max_width > num_chars_to_take {
            format!("{}...", &value[0..end_byte_index])

            //value.to_string()
        } else {
            value[..max_width].to_string()
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

    // Create table
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);

    // Add headers (with width limiting)
    let header_cells: Vec<Cell> = data
        .headers
        .iter()
        .map(|h| {
            let formatted = format_cell_value(h, max_width, wrap);
            Cell::new(&formatted).style_spec("Fgbc")
        })
        .collect();
    table.set_titles(Row::new(header_cells));

    // Add data rows (limit if needed)
    let rows_to_show = if max_rows == 0 {
        data.rows.len()
    } else {
        std::cmp::min(max_rows, data.rows.len())
    };

    for (row_idx, row) in data.rows.iter().enumerate().take(rows_to_show) {
        let cells: Vec<Cell> = row
            .iter()
            .enumerate()
            .map(|(col_idx, cell)| {
                // Get formula if it exists and show_formulas is true
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
                let cell_obj = Cell::new(&formatted);

                // Style based on type (only when not showing formulas)
                if show_formulas {
                    cell_obj.style_spec("Fg") // Green for formulas
                } else {
                    match cell {
                        CellValue::Int(_) | CellValue::Float(_) => {
                            cell_obj.style_spec("Fr") // Right-aligned numbers
                        }
                        CellValue::Bool(_) => {
                            cell_obj.style_spec("Fc") // Centered booleans
                        }
                        CellValue::Error(_) => {
                            cell_obj.style_spec("Frc") // Red errors, centered
                        }
                        _ => cell_obj,
                    }
                }
            })
            .collect();
        table.add_row(Row::new(cells));
    }

    table.printstd();

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
