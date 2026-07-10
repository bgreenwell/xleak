//! Render a [`SheetData`] as a formatted terminal table.

use crate::workbook::{CellValue, SheetData};
use anyhow::Result;
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Row, Table, Width,
};
use crossterm::style::Stylize;
use std::io::IsTerminal;

/// Format a cell value with width limiting. With `wrap`, the full text is
/// returned and comfy-table wraps it; otherwise long text is truncated with "...".
pub(crate) fn format_cell_value(value: &str, max_width: usize, wrap: bool) -> String {
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_string();
    }

    if wrap {
        value.to_string()
    } else if max_width > 3 {
        let truncated: String = value.chars().take(max_width - 3).collect();
        format!("{}...", truncated)
    } else {
        value.chars().take(max_width).collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn display_table(
    data: &SheetData,
    sheet_name: &str,
    max_rows: usize,
    all_sheets: &[&str],
    max_width: usize,
    wrap: bool,
    show_formulas: bool,
    use_color: bool,
) -> Result<()> {
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

    if !show_formulas {
        warn_if_blank_formulas(data);
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
        let mut cell = Cell::new(formatted).add_attribute(Attribute::Bold);
        if use_color {
            cell = cell.fg(Color::Green);
        }
        header_row.add_cell(cell);
    }
    table.set_header(header_row);
    table.set_constraints(
        (0..data.width).map(|_| ColumnConstraint::UpperBoundary(Width::Fixed(max_width as u16))),
    );

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

            cell_obj = if show_formulas {
                if use_color {
                    cell_obj.set_alignment(CellAlignment::Left).fg(Color::Green)
                } else {
                    cell_obj.set_alignment(CellAlignment::Left)
                }
            } else {
                match cell {
                    CellValue::Int(_) | CellValue::Float(_) => {
                        cell_obj.set_alignment(CellAlignment::Right)
                    }
                    CellValue::Bool(_) => cell_obj.set_alignment(CellAlignment::Center),
                    CellValue::Error(_) => {
                        if use_color {
                            cell_obj.set_alignment(CellAlignment::Center).fg(Color::Red)
                        } else {
                            cell_obj.set_alignment(CellAlignment::Center)
                        }
                    }
                    _ => cell_obj.set_alignment(CellAlignment::Left),
                }
            };
            table_row.add_cell(cell_obj);
        }
        table.add_row(table_row);
    }

    println!("{}", table);

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

/// Print a NOTE when several formula cells have no cached value, so users know
/// to pass `--formulas` or re-save the file in Excel/LibreOffice.
fn warn_if_blank_formulas(data: &SheetData) {
    let has_formulas = data
        .formulas
        .iter()
        .any(|row| row.iter().any(|f| f.is_some()));
    if !has_formulas {
        return;
    }

    let is_blank = |cell: &CellValue| match cell {
        CellValue::Empty => true,
        CellValue::String(s) => s.is_empty(),
        _ => false,
    };
    let blank_formula_count: usize = data
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .filter(|(c, cell)| {
                    is_blank(cell)
                        && data
                            .formulas
                            .get(r)
                            .and_then(|fr| fr.get(*c))
                            .and_then(|f| f.as_ref())
                            .is_some()
                })
                .count()
        })
        .sum();

    if blank_formula_count >= 2 {
        let prefix = if std::io::stdout().is_terminal() {
            format!("{}", "NOTE:".bold().yellow())
        } else {
            "NOTE:".to_string()
        };
        println!(
            "{prefix} Formula cells empty (not cached). Try --formulas or opening/saving in Excel/LibreOffice to cache the results."
        );
    }
}
