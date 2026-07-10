//! CSV/JSON/text exporters for sheet and table data.

use crate::workbook::{CellValue, SheetData, TableData};
use anyhow::Result;

/// Quote a CSV field when it contains the delimiter, a quote, or a newline,
/// escaping embedded quotes by doubling them.
pub(crate) fn csv_quote(value: String, delimiter: &str) -> String {
    if value.contains(delimiter) || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

/// Serialize a cell as a JSON value (numbers/bools bare, empty as null, rest quoted).
fn json_cell(cell: &CellValue) -> String {
    match cell {
        CellValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        CellValue::Int(i) => i.to_string(),
        CellValue::Float(f) => f.to_string(),
        CellValue::Bool(b) => b.to_string(),
        CellValue::Empty => "null".to_string(),
        _ => format!("\"{cell}\""),
    }
}

/// Print rows of JSON arrays (the `"data"` body), one array per row.
fn print_json_rows(rows: &[Vec<CellValue>]) {
    for (i, row) in rows.iter().enumerate() {
        print!("    [");
        for (j, cell) in row.iter().enumerate() {
            print!("{}", json_cell(cell));
            if j < row.len() - 1 {
                print!(", ");
            }
        }
        let comma = if i < rows.len() - 1 { "," } else { "" };
        println!("]{comma}");
    }
}

/// Print a JSON string array (e.g. headers), one element per line.
fn print_json_string_array(items: &[String]) {
    for (i, item) in items.iter().enumerate() {
        let comma = if i < items.len() - 1 { "," } else { "" };
        println!("    \"{item}\"{comma}");
    }
}

// ===== SheetData =====

pub fn export_csv(data: &SheetData, delimiter: &str) -> Result<()> {
    println!("{}", data.headers.join(delimiter));
    for row in &data.rows {
        let row_str: Vec<String> = row
            .iter()
            .map(|cell| csv_quote(cell.to_raw_string(), delimiter))
            .collect();
        println!("{}", row_str.join(delimiter));
    }
    Ok(())
}

pub fn export_json(data: &SheetData, sheet_name: &str) -> Result<()> {
    println!("{{");
    println!("  \"sheet\": \"{sheet_name}\",");
    println!("  \"rows\": {},", data.height);
    println!("  \"columns\": {},", data.width);
    println!("  \"headers\": [");
    print_json_string_array(&data.headers);
    println!("  ],");
    println!("  \"data\": [");
    print_json_rows(&data.rows);
    println!("  ]");
    println!("}}");
    Ok(())
}

pub fn export_text(data: &SheetData, delimiter: &str) -> Result<()> {
    println!("{}", data.headers.join(delimiter));
    for row in &data.rows {
        let row_str: Vec<String> = row.iter().map(|cell| cell.to_raw_string()).collect();
        println!("{}", row_str.join(delimiter));
    }
    Ok(())
}

// ===== TableData =====

pub fn export_table_json(table: &TableData) -> Result<()> {
    println!("{{");
    println!("  \"table\": \"{}\",", table.name);
    println!("  \"sheet\": \"{}\",", table.sheet_name);
    println!("  \"columns\": {},", table.headers.len());
    println!("  \"rows\": {},", table.rows.len());
    println!("  \"headers\": [");
    print_json_string_array(&table.headers);
    println!("  ],");
    println!("  \"data\": [");
    print_json_rows(&table.rows);
    println!("  ]");
    println!("}}");
    Ok(())
}

pub fn export_table_csv(table: &TableData, delimiter: &str) -> Result<()> {
    println!("{}", table.headers.join(delimiter));
    for row in &table.rows {
        let row_str: Vec<String> = row
            .iter()
            .map(|cell| csv_quote(cell.to_raw_string(), delimiter))
            .collect();
        println!("{}", row_str.join(delimiter));
    }
    Ok(())
}

pub fn export_table_text(table: &TableData, delimiter: &str) -> Result<()> {
    println!("{}", table.headers.join(delimiter));
    for row in &table.rows {
        let row_str: Vec<String> = row.iter().map(|cell| cell.to_raw_string()).collect();
        println!("{}", row_str.join(delimiter));
    }
    Ok(())
}
