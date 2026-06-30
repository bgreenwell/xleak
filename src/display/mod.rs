//! Non-interactive output: terminal tables and CSV/JSON/text exports.
//!
//! Split by responsibility:
//! - [`sheet`] — render a [`SheetData`](crate::workbook::SheetData) as a table
//! - [`table`] — render a [`TableData`](crate::workbook::TableData) as a table
//! - [`export`] — CSV/JSON/text exporters for both data types

mod export;
mod sheet;
mod table;

pub(crate) use export::csv_quote;
pub use export::{
    export_csv, export_json, export_table_csv, export_table_json, export_table_text, export_text,
};
pub use sheet::display_table;
pub use table::display_table_data;
