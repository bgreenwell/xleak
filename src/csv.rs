//! CSV/TSV file support (enabled via the `csv` feature).
//!
//! CSV and Excel files share the same row/column structure, so a delimited
//! text file is exposed to the rest of xleak as a single-sheet [`Workbook`].

use anyhow::{Context, Result};
use std::path::Path;

use crate::workbook::{LazySheetData, SheetData};

/// A CSV/TSV file parsed fully into memory as one sheet.
pub struct CsvData {
    /// Display name for the single sheet (derived from the file name).
    pub sheet_name: String,
    /// All records, including the header row (if any), as raw strings.
    rows: Vec<Vec<String>>,
}

impl CsvData {
    /// Open and parse a CSV/TSV file.
    ///
    /// When `delimiter` is `None`, the delimiter is inferred: `.tsv` files use a
    /// tab, everything else uses a comma.
    pub fn open(path: &Path, delimiter: Option<u8>) -> Result<Self> {
        let delimiter = delimiter.unwrap_or_else(|| default_delimiter(path));

        let sheet_name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "CSV".to_string());

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false) // We manage the header row ourselves.
            .flexible(true) // Allow rows with differing field counts.
            .delimiter(delimiter)
            .from_path(path)
            .with_context(|| format!("Failed to open CSV file: {}", path.display()))?;

        let mut rows: Vec<Vec<String>> = Vec::new();
        for (i, record) in reader.records().enumerate() {
            let record = record.with_context(|| format!("Failed to parse CSV record {}", i + 1))?;
            rows.push(record.iter().map(|f| f.to_string()).collect());
        }

        Ok(Self { sheet_name, rows })
    }

    /// Materialize all rows into an eager [`SheetData`].
    pub fn to_sheet_data(&self, no_header: bool) -> SheetData {
        SheetData::from_string_rows(self.rows.clone(), no_header)
    }

    /// Build a [`LazySheetData`] (CSV is already fully in memory).
    pub fn to_lazy_sheet_data(&self, no_header: bool) -> LazySheetData {
        LazySheetData::from_string_rows(self.rows.clone(), no_header)
    }
}

/// Infer the field delimiter from the file extension.
fn default_delimiter(path: &Path) -> u8 {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("tsv") | Some("tab") => b'\t',
        _ => b',',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_delimiter_tsv() {
        assert_eq!(default_delimiter(Path::new("data.tsv")), b'\t');
        assert_eq!(default_delimiter(Path::new("data.csv")), b',');
        assert_eq!(default_delimiter(Path::new("data.txt")), b',');
    }
}
