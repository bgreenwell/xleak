use anyhow::{Context, Result, anyhow};
use calamine::{Data, Range, Reader, Sheets, Table, open_workbook_auto};
use chrono::{Duration, NaiveDate};
use std::path::Path;

/// Backend storage for a workbook: either an Excel file (via calamine) or an
/// in-memory CSV/TSV file parsed into a single sheet.
// The Excel variant (calamine `Sheets`) is intentionally large; it is the common
// path and is allocated once per process, so boxing it adds no real benefit.
#[allow(clippy::large_enum_variant)]
enum Backend {
    Excel(Sheets<std::io::BufReader<std::fs::File>>),
    #[cfg(feature = "csv")]
    Csv(crate::csv::CsvData),
}

pub struct Workbook {
    backend: Backend,
}

impl Workbook {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let sheets = open_workbook_auto(path.as_ref()).context("Failed to open workbook")?;

        Ok(Self {
            backend: Backend::Excel(sheets),
        })
    }

    /// Open a CSV/TSV file as a single-sheet workbook.
    #[cfg(feature = "csv")]
    pub fn open_csv(path: impl AsRef<Path>, delimiter: Option<u8>) -> Result<Self> {
        let data = crate::csv::CsvData::open(path.as_ref(), delimiter)?;
        Ok(Self {
            backend: Backend::Csv(data),
        })
    }

    pub fn sheet_names(&self) -> Vec<String> {
        match &self.backend {
            Backend::Excel(sheets) => sheets.sheet_names(),
            #[cfg(feature = "csv")]
            Backend::Csv(data) => vec![data.sheet_name.clone()],
        }
    }

    /// Loads all rows eagerly into memory
    pub fn load_sheet(&mut self, name: &str, no_header: bool) -> Result<SheetData> {
        match &mut self.backend {
            Backend::Excel(sheets) => {
                let range = sheets
                    .worksheet_range(name)
                    .with_context(|| format!("Sheet '{name}' not found"))?;

                // Try to load formulas, but don't fail if they're not available
                let formula_range = sheets.worksheet_formula(name).ok();

                Ok(SheetData::from_range_with_formulas(
                    range,
                    formula_range,
                    no_header,
                ))
            }
            #[cfg(feature = "csv")]
            Backend::Csv(data) => Ok(data.to_sheet_data(no_header)),
        }
    }

    /// Loads only headers; rows fetched on demand
    pub fn load_sheet_lazy(&mut self, name: &str, no_header: bool) -> Result<LazySheetData> {
        match &mut self.backend {
            Backend::Excel(sheets) => {
                let range = sheets
                    .worksheet_range(name)
                    .with_context(|| format!("Sheet '{name}' not found"))?;

                // Try to load formulas, but don't fail if they're not available
                let formula_range = sheets.worksheet_formula(name).ok();

                Ok(LazySheetData::from_range_with_formulas(
                    range,
                    formula_range,
                    no_header,
                ))
            }
            #[cfg(feature = "csv")]
            Backend::Csv(data) => Ok(data.to_lazy_sheet_data(no_header)),
        }
    }

    // ===== Table API (Xlsx only) =====

    /// Load table metadata from the workbook (Xlsx only)
    pub fn load_tables(&mut self) -> Result<()> {
        match &mut self.backend {
            Backend::Excel(Sheets::Xlsx(xlsx)) => xlsx
                .load_tables()
                .context("Failed to load table metadata")
                .map_err(|e| anyhow!("{e}")),
            _ => Err(anyhow!("Tables are only supported in .xlsx files")),
        }
    }

    /// Get all table names in the workbook (Xlsx only)
    pub fn table_names(&self) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Excel(Sheets::Xlsx(xlsx)) => {
                Ok(xlsx.table_names().iter().map(|s| (*s).clone()).collect())
            }
            _ => Err(anyhow!("Tables are only supported in .xlsx files")),
        }
    }

    /// Get table names in a specific sheet (Xlsx only)
    pub fn table_names_in_sheet(&self, sheet_name: &str) -> Result<Vec<String>> {
        match &self.backend {
            Backend::Excel(Sheets::Xlsx(xlsx)) => Ok(xlsx
                .table_names_in_sheet(sheet_name)
                .iter()
                .map(|s| (*s).clone())
                .collect()),
            _ => Err(anyhow!("Tables are only supported in .xlsx files")),
        }
    }

    /// Get table data by name (Xlsx only)
    pub fn table_by_name(&mut self, table_name: &str) -> Result<TableData> {
        match &mut self.backend {
            Backend::Excel(Sheets::Xlsx(xlsx)) => {
                let table = xlsx
                    .table_by_name(table_name)
                    .map_err(|e| anyhow!("Table '{table_name}' not found: {e}"))?;

                Ok(TableData::from_calamine_table(table))
            }
            _ => Err(anyhow!("Tables are only supported in .xlsx files")),
        }
    }
}

/// Eagerly-loaded sheet data (loads all rows immediately)
#[derive(Debug, Clone)]
pub struct SheetData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<CellValue>>,
    pub formulas: Vec<Vec<Option<String>>>, // Parallel structure to rows with formulas
    pub width: usize,
    pub height: usize,
}

/// Backing source for lazily-served rows.
enum LazySource {
    /// Excel range backed by calamine (rows materialized on demand).
    Excel {
        range: Range<Data>,
        formula_range: Option<Range<String>>,
    },
    /// CSV/TSV data already fully parsed in memory.
    #[cfg(feature = "csv")]
    Csv { rows: Vec<Vec<CellValue>> },
}

/// Lazy-loaded sheet data (loads rows on demand)
pub struct LazySheetData {
    source: LazySource,
    pub headers: Vec<String>,
    pub width: usize,
    pub height: usize,
}

impl LazySheetData {
    /// Build a lazy sheet from already-parsed CSV string rows.
    #[cfg(feature = "csv")]
    pub fn from_string_rows(mut rows: Vec<Vec<String>>, no_header: bool) -> Self {
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);

        let headers = if !no_header && !rows.is_empty() {
            let mut header_row = rows.remove(0);
            header_row.resize(width, String::new());
            header_row
        } else {
            SheetData::default_headers(width)
        };

        let data_rows: Vec<Vec<CellValue>> = rows
            .into_iter()
            .map(|mut row| {
                row.resize(width, String::new());
                row.into_iter().map(CellValue::from_csv_field).collect()
            })
            .collect();

        let height = data_rows.len();

        Self {
            source: LazySource::Csv { rows: data_rows },
            headers,
            width,
            height,
        }
    }

    /// Extracts headers only; defers row loading
    pub fn from_range_with_formulas(
        range: Range<Data>,
        formula_range: Option<Range<String>>,
        no_header: bool,
    ) -> Self {
        let (height, width) = range.get_size();

        let headers = if !no_header && height > 0 {
            range
                .rows()
                .next()
                .map(|row| row.iter().map(SheetData::cell_to_string).collect())
                .unwrap_or_default()
        } else {
            SheetData::default_headers(width)
        };

        Self {
            source: LazySource::Excel {
                range,
                formula_range,
            },
            headers,
            width,
            // Exclude the header row from the row count unless no_header.
            height: if no_header {
                height
            } else {
                height.saturating_sub(1)
            },
        }
    }

    /// Zero-indexed row range; header excluded (unless no_header)
    pub fn get_rows(
        &self,
        start: usize,
        count: usize,
        no_header: bool,
    ) -> (Vec<Vec<CellValue>>, Vec<Vec<Option<String>>>) {
        let end = (start + count).min(self.height);
        if end <= start {
            return (Vec::new(), Vec::new());
        }

        match &self.source {
            LazySource::Excel { range, .. } => {
                let skip = if no_header { start } else { 1 + start };
                let rows: Vec<Vec<CellValue>> = range
                    .rows()
                    .skip(skip)
                    .take(end - start)
                    .map(|row| row.iter().map(SheetData::datatype_to_cellvalue).collect())
                    .collect();

                let formulas = self.get_formulas_for_range(start, end, no_header);
                (rows, formulas)
            }
            #[cfg(feature = "csv")]
            LazySource::Csv { rows } => {
                let slice: Vec<Vec<CellValue>> = rows[start..end].to_vec();
                let formulas = vec![vec![None; self.width]; end - start];
                (slice, formulas)
            }
        }
    }

    fn get_formulas_for_range(
        &self,
        start: usize,
        end: usize,
        no_header: bool,
    ) -> Vec<Vec<Option<String>>> {
        match &self.source {
            LazySource::Excel { formula_range, .. } => {
                if let Some(formula_range) = formula_range {
                    SheetData::build_formula_grid(
                        formula_range,
                        self.width,
                        start,
                        end - start,
                        no_header,
                    )
                } else {
                    vec![vec![None; self.width]; end - start]
                }
            }
            #[cfg(feature = "csv")]
            LazySource::Csv { .. } => vec![vec![None; self.width]; end - start],
        }
    }

    /// Consumes lazy data and loads all rows into memory
    #[allow(clippy::wrong_self_convention)]
    pub fn to_sheet_data(self, no_header: bool) -> SheetData {
        match self.source {
            LazySource::Excel {
                range,
                formula_range,
            } => SheetData::from_range_with_formulas(range, formula_range, no_header),
            #[cfg(feature = "csv")]
            LazySource::Csv { rows } => {
                let width = self.width;
                let height = rows.len();
                let formulas = vec![vec![None; width]; height];
                SheetData {
                    headers: self.headers,
                    rows,
                    formulas,
                    width,
                    height,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CellValue {
    Empty,
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Error(String),
    DateTime(f64), // Excel datetime as float
}

impl CellValue {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    #[allow(dead_code)]
    pub fn is_numeric(&self) -> bool {
        matches!(self, CellValue::Int(_) | CellValue::Float(_))
    }

    /// Infer a `CellValue` from a raw CSV field, detecting empty, booleans, and
    /// numbers. Everything else is kept as a string (preserving the original text).
    #[cfg(feature = "csv")]
    pub fn from_csv_field(field: String) -> Self {
        let trimmed = field.trim();
        if trimmed.is_empty() {
            return CellValue::Empty;
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "true" => return CellValue::Bool(true),
            "false" => return CellValue::Bool(false),
            _ => {}
        }

        // Integers (avoid treating values with leading zeros like "007" as numbers
        // to preserve identifiers such as zip codes).
        let has_leading_zero =
            trimmed.len() > 1 && trimmed.starts_with('0') && !trimmed.starts_with("0.");
        if !has_leading_zero {
            if let Ok(i) = trimmed.parse::<i64>() {
                return CellValue::Int(i);
            }
            if let Ok(fl) = trimmed.parse::<f64>()
                && fl.is_finite()
            {
                return CellValue::Float(fl);
            }
        }

        CellValue::String(field)
    }

    /// Format an Excel datetime serial number into a human-readable date/time string.
    /// Returns (date_string, time_string) where time_string is None if there's no time component.
    fn format_datetime(dt: f64) -> (String, Option<String>) {
        let days = dt.floor() as i64;
        let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).unwrap();
        // Adjust for Excel's 1900 leap year bug (day 60 = Feb 29, 1900 which didn't exist)
        let adjusted_days = if days > 60 { days - 1 } else { days };

        let date = if let Some(d) = epoch.checked_add_signed(Duration::days(adjusted_days)) {
            d
        } else {
            return (format!("Date[{}]", days), None);
        };

        // Relative to the floored day, so it's always in [0, 1) even for
        // negative serials (f64::fract is negative there).
        let frac = dt - days as f64;
        if frac < 0.000001 {
            return (date.format("%Y-%m-%d").to_string(), None);
        }

        let total_seconds = (frac * 86400.0).round() as u32;
        // Rounding a time just before midnight can hit exactly 86400 seconds;
        // that's midnight of the next day, not 24:00:00 (#54).
        let (date, total_seconds) = if total_seconds >= 86400 {
            match date.checked_add_signed(Duration::days(1)) {
                Some(d) => (d, 0),
                None => return (format!("Date[{}]", days), None),
            }
        } else {
            (date, total_seconds)
        };
        if total_seconds == 0 {
            return (date.format("%Y-%m-%d").to_string(), None);
        }

        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        (
            date.format("%Y-%m-%d").to_string(),
            Some(format!("{:02}:{:02}:{:02}", hours, minutes, seconds)),
        )
    }

    /// Returns unformatted value (for export/clipboard)
    pub fn to_raw_string(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::String(s) => s.clone(),
            CellValue::Int(i) => i.to_string(),
            CellValue::Float(val) => {
                if val.fract() == 0.0 {
                    format!("{val:.0}")
                } else {
                    val.to_string()
                }
            }
            CellValue::Bool(b) => b.to_string(),
            CellValue::Error(e) => format!("#{e}"),
            CellValue::DateTime(dt) => {
                let (date_str, time_str) = Self::format_datetime(*dt);
                if let Some(time) = time_str {
                    format!("{} {}", date_str, time)
                } else {
                    date_str
                }
            }
        }
    }
}

/// Excel Table data
#[derive(Debug, Clone)]
pub struct TableData {
    pub name: String,
    pub sheet_name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<CellValue>>,
}

impl TableData {
    pub fn from_calamine_table(table: Table<Data>) -> Self {
        let name = table.name().to_string();
        let sheet_name = table.sheet_name().to_string();
        let headers = table.columns().to_vec();

        let rows: Vec<Vec<CellValue>> = table
            .data()
            .rows()
            .map(|row| row.iter().map(SheetData::datatype_to_cellvalue).collect())
            .collect();

        Self {
            name,
            sheet_name,
            headers,
            rows,
        }
    }
}

impl std::fmt::Display for CellValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellValue::Empty => write!(f, ""),
            CellValue::String(s) => write!(f, "{s}"),
            CellValue::Int(i) => {
                // Format integers with thousand separators
                let s = i.to_string();
                let negative = s.starts_with('-');
                let digits: String = s.trim_start_matches('-').chars().collect();
                let mut result = String::new();
                for (idx, ch) in digits.chars().rev().enumerate() {
                    if idx > 0 && idx % 3 == 0 {
                        result.push(',');
                    }
                    result.push(ch);
                }
                if negative {
                    result.push('-');
                }
                write!(f, "{}", result.chars().rev().collect::<String>())
            }
            CellValue::Float(val) => {
                // Format floats with thousand separators
                let formatted = if val.fract() == 0.0 {
                    format!("{val:.0}")
                } else {
                    format!("{val:.2}")
                };
                let parts: Vec<&str> = formatted.split('.').collect();
                let int_part = parts[0];
                let negative = int_part.starts_with('-');
                let digits: String = int_part.trim_start_matches('-').chars().collect();
                let mut result = String::new();
                for (idx, ch) in digits.chars().rev().enumerate() {
                    if idx > 0 && idx % 3 == 0 {
                        result.push(',');
                    }
                    result.push(ch);
                }
                if negative {
                    result.push('-');
                }
                let int_formatted: String = result.chars().rev().collect();
                if parts.len() > 1 {
                    write!(f, "{}.{}", int_formatted, parts[1])
                } else {
                    write!(f, "{}", int_formatted)
                }
            }
            CellValue::Bool(b) => {
                // Use lowercase for booleans
                write!(f, "{}", if *b { "true" } else { "false" })
            }
            CellValue::Error(e) => write!(f, "ERROR: {e}"),
            CellValue::DateTime(d) => {
                let (date_str, time_str) = Self::format_datetime(*d);
                match time_str {
                    Some(time) => write!(f, "{} {}", date_str, time),
                    None => write!(f, "{}", date_str),
                }
            }
        }
    }
}

impl SheetData {
    /// Build a formula grid for data rows `start_row..start_row + num_rows`,
    /// mapping formulas from their absolute sheet positions.
    fn build_formula_grid(
        formula_range: &Range<String>,
        width: usize,
        start_row: usize,
        num_rows: usize,
        no_header: bool,
    ) -> Vec<Vec<Option<String>>> {
        let formula_start = formula_range.start().unwrap_or((0, 0));

        let mut formula_grid: Vec<Vec<Option<String>>> = vec![vec![None; width]; num_rows];

        for (row_offset, formula_row) in formula_range.rows().enumerate() {
            let absolute_row = formula_start.0 as usize + row_offset;

            // Sheet row -> data row: with a header, data row i is sheet row
            // i + 1 (the header row has no data row); without one they
            // coincide (#50: the -1 shift used to apply unconditionally).
            let data_row_idx = if no_header {
                absolute_row
            } else {
                match absolute_row.checked_sub(1) {
                    Some(idx) => idx,
                    None => continue, // formula on the header row
                }
            };

            // Only process if this row is in our requested range
            if data_row_idx >= start_row && data_row_idx < start_row + num_rows {
                let result_idx = data_row_idx - start_row;

                for (col_offset, formula_str) in formula_row.iter().enumerate() {
                    let absolute_col = formula_start.1 as usize + col_offset;
                    if absolute_col < width && !formula_str.is_empty() {
                        formula_grid[result_idx][absolute_col] = Some(formula_str.clone());
                    }
                }
            }
        }

        formula_grid
    }

    pub fn from_range_with_formulas(
        range: Range<Data>,
        formula_range: Option<Range<String>>,
        no_header: bool,
    ) -> Self {
        let (height, width) = range.get_size();

        let headers = if !no_header && height > 0 {
            range
                .rows()
                .next()
                .map(|row| row.iter().map(Self::cell_to_string).collect())
                .unwrap_or_default()
        } else {
            Self::default_headers(width)
        };

        let rows: Vec<Vec<CellValue>> = if no_header {
            range
                .rows()
                .map(|row| row.iter().map(Self::datatype_to_cellvalue).collect())
                .collect()
        } else {
            range
                .rows()
                .skip(1)
                .map(|row| row.iter().map(Self::datatype_to_cellvalue).collect())
                .collect()
        };

        let row_count = if no_header {
            height
        } else {
            height.saturating_sub(1)
        };

        let formulas: Vec<Vec<Option<String>>> = if let Some(ref formula_range) = formula_range {
            Self::build_formula_grid(formula_range, width, 0, row_count, no_header)
        } else {
            vec![vec![None; width]; row_count]
        };

        Self {
            headers,
            rows,
            formulas,
            width,
            height: row_count,
        }
    }

    /// Generate default spreadsheet-style column headers (A, B, C, ... Z, AA, ...).
    pub(crate) fn default_headers(width: usize) -> Vec<String> {
        (0..width)
            .map(crate::utils::column_index_to_letters)
            .collect()
    }

    /// Build a `SheetData` from already-parsed string rows (e.g. from a CSV file).
    /// When `no_header` is false, the first row is used as headers.
    #[cfg(feature = "csv")]
    pub fn from_string_rows(mut rows: Vec<Vec<String>>, no_header: bool) -> Self {
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);

        let headers = if !no_header && !rows.is_empty() {
            let mut header_row = rows.remove(0);
            header_row.resize(width, String::new());
            header_row
        } else {
            Self::default_headers(width)
        };

        let data_rows: Vec<Vec<CellValue>> = rows
            .into_iter()
            .map(|mut row| {
                row.resize(width, String::new());
                row.into_iter().map(CellValue::from_csv_field).collect()
            })
            .collect();

        let height = data_rows.len();
        let formulas = vec![vec![None; width]; height];

        Self {
            headers,
            rows: data_rows,
            formulas,
            width,
            height,
        }
    }

    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{f:.0}")
                } else {
                    f.to_string()
                }
            }
            Data::Bool(b) => b.to_string(),
            Data::Error(e) => format!("ERROR: {e:?}"),
            Data::DateTime(d) => format!("Date({})", d.as_f64()),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        }
    }

    fn datatype_to_cellvalue(cell: &Data) -> CellValue {
        match cell {
            Data::Empty => CellValue::Empty,
            Data::String(s) => CellValue::String(s.clone()),
            Data::Int(i) => CellValue::Int(*i),
            Data::Float(f) => CellValue::Float(*f),
            Data::Bool(b) => CellValue::Bool(*b),
            Data::Error(e) => CellValue::Error(format!("{e:?}")),
            Data::DateTime(d) => CellValue::DateTime(d.as_f64()),
            Data::DateTimeIso(s) => CellValue::String(s.clone()),
            Data::DurationIso(s) => CellValue::String(s.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cellvalue_display_integer() {
        let val = CellValue::Int(1234567);
        assert_eq!(val.to_string(), "1,234,567");
    }

    #[test]
    fn test_cellvalue_display_negative_integer() {
        let val = CellValue::Int(-1234567);
        assert_eq!(val.to_string(), "-1,234,567");
    }

    #[test]
    fn test_cellvalue_display_float() {
        let val = CellValue::Float(1234567.89);
        assert_eq!(val.to_string(), "1,234,567.89");
    }

    #[test]
    fn test_cellvalue_display_float_whole_number() {
        let val = CellValue::Float(1000.0);
        assert_eq!(val.to_string(), "1,000");
    }

    #[test]
    fn test_cellvalue_display_boolean() {
        assert_eq!(CellValue::Bool(true).to_string(), "true");
        assert_eq!(CellValue::Bool(false).to_string(), "false");
    }

    #[test]
    fn test_cellvalue_display_string() {
        let val = CellValue::String("Hello, World!".to_string());
        assert_eq!(val.to_string(), "Hello, World!");
    }

    #[test]
    fn test_cellvalue_display_empty() {
        let val = CellValue::Empty;
        assert_eq!(val.to_string(), "");
    }

    #[test]
    fn test_cellvalue_display_error() {
        let val = CellValue::Error("DIV/0!".to_string());
        assert_eq!(val.to_string(), "ERROR: DIV/0!");
    }

    #[test]
    fn test_cellvalue_to_raw_string_integer() {
        let val = CellValue::Int(1234567);
        assert_eq!(val.to_raw_string(), "1234567");
    }

    #[test]
    fn test_cellvalue_to_raw_string_float() {
        let val = CellValue::Float(123.45);
        assert_eq!(val.to_raw_string(), "123.45");
    }

    #[test]
    fn test_cellvalue_to_raw_string_large_float() {
        // Regression: large whole-number floats were exported with thousands separators
        // (e.g. "18,441,600,422") making CSV/text output unparseable as numbers (#34)
        let val = CellValue::Float(18_441_600_422.0);
        assert_eq!(val.to_raw_string(), "18441600422");
        // Display (TUI) formatting should still use separators
        assert_eq!(val.to_string(), "18,441,600,422");
    }

    #[test]
    fn test_cellvalue_to_raw_string_large_int() {
        let val = CellValue::Int(18_441_600_422);
        assert_eq!(val.to_raw_string(), "18441600422");
        assert_eq!(val.to_string(), "18,441,600,422");
    }

    #[test]
    fn test_cellvalue_is_empty() {
        assert!(CellValue::Empty.is_empty());
        assert!(!CellValue::Int(0).is_empty());
        assert!(!CellValue::String("".to_string()).is_empty());
    }

    #[test]
    fn test_cellvalue_is_numeric() {
        assert!(CellValue::Int(123).is_numeric());
        assert!(CellValue::Float(123.45).is_numeric());
        assert!(!CellValue::String("123".to_string()).is_numeric());
        assert!(!CellValue::Empty.is_numeric());
    }

    #[test]
    fn test_datetime_display() {
        // Excel date: January 1, 1900 is day 1
        let val = CellValue::DateTime(1.0);
        let display = val.to_string();
        // Should contain a date in YYYY-MM-DD format
        assert!(display.contains("1900") || display.contains("1899"));
    }

    #[test]
    fn test_datetime_with_time() {
        // Excel datetime with time component
        // Day 1 + 0.5 = 12:00:00 on Jan 1, 1900
        let val = CellValue::DateTime(1.5);
        let display = val.to_string();
        // Should contain both date and time
        assert!(display.contains(":"));
        assert!(display.len() > 10); // Date + time is longer than just date
    }

    #[test]
    fn test_datetime_just_before_midnight_rolls_to_next_day() {
        // Regression for #54: a fraction that rounds to 86400 seconds must
        // render as midnight of the next day, never as 24:00:00.
        // Day 1 = 1900-01-01; 0.9999999 * 86400 rounds to 86400.
        let val = CellValue::DateTime(1.999_999_9);
        assert_eq!(val.to_string(), "1900-01-02");
        assert_eq!(val.to_raw_string(), "1900-01-02");

        // A representable time just before midnight still renders normally.
        let val = CellValue::DateTime(1.0 + 86399.0 / 86400.0);
        assert_eq!(val.to_string(), "1900-01-01 23:59:59");
    }

    #[test]
    fn test_workbook_open_real_file() {
        // Test with actual test file if it exists
        if let Ok(wb) = Workbook::open("tests/fixtures/test_data.xlsx") {
            let sheet_names = wb.sheet_names();
            assert!(!sheet_names.is_empty(), "Should have at least one sheet");
        }
        // If file doesn't exist, test passes (integration test needs real file)
    }

    #[test]
    fn test_sheet_data_structure() {
        // Test SheetData structure can be created
        let sheet = SheetData {
            headers: vec!["Name".to_string(), "Age".to_string()],
            rows: vec![
                vec![CellValue::String("Alice".to_string()), CellValue::Int(30)],
                vec![CellValue::String("Bob".to_string()), CellValue::Int(25)],
            ],
            formulas: vec![vec![None, None], vec![None, None]],
            width: 2,
            height: 2,
        };

        assert_eq!(sheet.width, 2);
        assert_eq!(sheet.height, 2);
        assert_eq!(sheet.headers.len(), 2);
        assert_eq!(sheet.rows.len(), 2);
    }

    #[test]
    fn test_no_header_row_and_formula_lengths_match() {
        // Regression: with `no_header`, every source row is kept as data, so
        // `rows`, `formulas`, and `height` must all agree (previously `formulas`
        // and `height` were sized to H-1, causing an out-of-bounds slice).
        let mut range: Range<Data> = Range::new((0, 0), (2, 1));
        range.set_value((0, 0), Data::String("a".into()));
        range.set_value((0, 1), Data::String("b".into()));
        range.set_value((1, 0), Data::Int(1));
        range.set_value((1, 1), Data::Int(2));
        range.set_value((2, 0), Data::Int(3));
        range.set_value((2, 1), Data::Int(4));

        let sheet = SheetData::from_range_with_formulas(range.clone(), None, true);
        assert_eq!(sheet.height, 3);
        assert_eq!(sheet.rows.len(), 3);
        assert_eq!(sheet.formulas.len(), 3);

        // With a header, the first row is consumed and counts drop by one.
        let sheet = SheetData::from_range_with_formulas(range, None, false);
        assert_eq!(sheet.height, 2);
        assert_eq!(sheet.rows.len(), 2);
        assert_eq!(sheet.formulas.len(), 2);
    }

    /// 3-row sheet with a formula on sheet rows 0 and 2 (0-indexed).
    fn range_with_formulas() -> (Range<Data>, Range<String>) {
        let mut range: Range<Data> = Range::new((0, 0), (2, 0));
        range.set_value((0, 0), Data::String("head".into()));
        range.set_value((1, 0), Data::Int(10));
        range.set_value((2, 0), Data::Int(20));

        let mut formulas: Range<String> = Range::new((0, 0), (2, 0));
        formulas.set_value((0, 0), "=ROW0".to_string());
        formulas.set_value((2, 0), "=ROW2".to_string());
        (range, formulas)
    }

    #[test]
    fn test_formula_alignment_with_no_header() {
        // Regression for #50: without a header, data row i IS sheet row i, but
        // the grid builder applied the header -1 shift unconditionally, showing
        // every formula one row off and dropping the one on sheet row 0.
        let (range, formulas) = range_with_formulas();
        let sheet = SheetData::from_range_with_formulas(range, Some(formulas), true);
        assert_eq!(sheet.formulas[0][0].as_deref(), Some("=ROW0"));
        assert_eq!(sheet.formulas[1][0], None);
        assert_eq!(sheet.formulas[2][0].as_deref(), Some("=ROW2"));
    }

    #[test]
    fn test_formula_alignment_with_header() {
        // With a header, data row i is sheet row i + 1; the formula on the
        // header row itself is not part of the data grid.
        let (range, formulas) = range_with_formulas();
        let sheet = SheetData::from_range_with_formulas(range, Some(formulas), false);
        assert_eq!(sheet.formulas[0][0], None); // sheet row 1 has no formula
        assert_eq!(sheet.formulas[1][0].as_deref(), Some("=ROW2"));
    }

    #[test]
    fn test_formula_alignment_lazy_no_header() {
        // Same regression via the lazy path used by the TUI.
        let (range, formulas) = range_with_formulas();
        let lazy = LazySheetData::from_range_with_formulas(range, Some(formulas), true);
        let (rows, grid) = lazy.get_rows(0, 3, true);
        assert_eq!(rows.len(), 3);
        assert_eq!(grid[0][0].as_deref(), Some("=ROW0"));
        assert_eq!(grid[1][0], None);
        assert_eq!(grid[2][0].as_deref(), Some("=ROW2"));
    }
}
