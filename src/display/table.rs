//! Render a [`TableData`] (named Excel table) as a formatted terminal table.

use crate::workbook::{CellValue, TableData};
use anyhow::Result;
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Row, Table, Width,
};

pub fn display_table_data(
    table: &TableData,
    max_rows: usize,
    max_width: usize,
    use_color: bool,
) -> Result<()> {
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

    let max_width_u16 = max_width as u16;
    let mut table_obj = Table::new();
    table_obj.set_content_arrangement(ContentArrangement::Dynamic);
    table_obj.set_width(
        (table.headers.len() as u16)
            .saturating_mul(max_width_u16 + 3)
            .max(max_width_u16),
    );

    let mut header_row = Row::new();
    for h in &table.headers {
        let mut cell = Cell::new(h).add_attribute(Attribute::Bold);
        if use_color {
            cell = cell.fg(Color::Green);
        }
        header_row.add_cell(cell);
    }
    table_obj.set_header(header_row);
    table_obj.set_constraints(
        (0..table.headers.len())
            .map(|_| ColumnConstraint::UpperBoundary(Width::Fixed(max_width_u16))),
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
                CellValue::Int(_) | CellValue::Float(_) => {
                    Cell::new(cell.to_string()).set_alignment(CellAlignment::Right)
                }
                CellValue::Bool(_) => {
                    Cell::new(cell.to_string()).set_alignment(CellAlignment::Center)
                }
                CellValue::Error(_) => {
                    let mut c = Cell::new(cell.to_string()).set_alignment(CellAlignment::Center);
                    if use_color {
                        c = c.fg(Color::Red);
                    }
                    c
                }
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
