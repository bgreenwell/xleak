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

/// Join fields into one CSV line, quoting each as needed.
fn csv_line<I: IntoIterator<Item = String>>(fields: I, delimiter: &str) -> String {
    fields
        .into_iter()
        .map(|f| csv_quote(f, delimiter))
        .collect::<Vec<_>>()
        .join(delimiter)
}

/// Serialize a string as a JSON string literal with full escaping.
fn json_string(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

/// Serialize a cell as a JSON value (numbers/bools bare, empty as null, rest quoted).
fn json_cell(cell: &CellValue) -> String {
    match cell {
        CellValue::String(s) => json_string(s),
        CellValue::Int(i) => i.to_string(),
        // NaN and infinities have no JSON representation; export as null.
        CellValue::Float(f) => {
            serde_json::Number::from_f64(*f).map_or_else(|| "null".to_string(), |n| n.to_string())
        }
        CellValue::Bool(b) => b.to_string(),
        CellValue::Empty => "null".to_string(),
        _ => json_string(&cell.to_string()),
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
        println!("    {}{comma}", json_string(item));
    }
}

// ===== SheetData =====

pub fn export_csv(data: &SheetData, delimiter: &str) -> Result<()> {
    println!("{}", csv_line(data.headers.iter().cloned(), delimiter));
    for row in &data.rows {
        println!(
            "{}",
            csv_line(row.iter().map(|cell| cell.to_raw_string()), delimiter)
        );
    }
    Ok(())
}

pub fn export_json(data: &SheetData, sheet_name: &str) -> Result<()> {
    println!("{{");
    println!("  \"sheet\": {},", json_string(sheet_name));
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
    println!("  \"table\": {},", json_string(&table.name));
    println!("  \"sheet\": {},", json_string(&table.sheet_name));
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
    println!("{}", csv_line(table.headers.iter().cloned(), delimiter));
    for row in &table.rows {
        println!(
            "{}",
            csv_line(row.iter().map(|cell| cell.to_raw_string()), delimiter)
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_line_quotes_headers_and_fields() {
        // #52: headers with delimiters/quotes/newlines must be quoted like data.
        let line = csv_line(
            vec![
                "Name, Title".to_string(),
                "Say \"hi\"".to_string(),
                "plain".to_string(),
            ],
            ",",
        );
        assert_eq!(line, "\"Name, Title\",\"Say \"\"hi\"\"\",plain");
    }

    #[test]
    fn test_csv_line_respects_custom_delimiter() {
        let line = csv_line(vec!["a;b".to_string(), "c,d".to_string()], ";");
        assert_eq!(line, "\"a;b\";c,d");
    }

    #[test]
    fn test_json_string_escapes_specials() {
        // #53: backslashes, quotes, newlines, and control chars must be escaped.
        assert_eq!(json_string("say \"hi\""), r#""say \"hi\"""#);
        assert_eq!(json_string("back\\slash"), r#""back\\slash""#);
        assert_eq!(json_string("line1\nline2"), r#""line1\nline2""#);
        assert_eq!(json_string("tab\there"), r#""tab\there""#);
    }

    #[test]
    fn test_json_cell_values() {
        assert_eq!(json_cell(&CellValue::Int(42)), "42");
        assert_eq!(json_cell(&CellValue::Bool(true)), "true");
        assert_eq!(json_cell(&CellValue::Empty), "null");
        assert_eq!(
            json_cell(&CellValue::String("a\\b\n\"c\"".to_string())),
            r#""a\\b\n\"c\"""#
        );
    }

    #[test]
    fn test_json_cell_nonfinite_floats_are_null() {
        assert_eq!(json_cell(&CellValue::Float(f64::NAN)), "null");
        assert_eq!(json_cell(&CellValue::Float(f64::INFINITY)), "null");
        assert_eq!(json_cell(&CellValue::Float(1.5)), "1.5");
    }
}
