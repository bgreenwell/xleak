use std::cmp::Ordering;

use crate::workbook::CellValue;

use super::state::{RowCache, SheetDataSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Next state when the sort key is pressed on `col`:
/// file order -> ascending -> descending -> file order.
pub fn next_sort(
    current: Option<(usize, SortDirection)>,
    col: usize,
) -> Option<(usize, SortDirection)> {
    match current {
        Some((c, SortDirection::Ascending)) if c == col => Some((col, SortDirection::Descending)),
        Some((c, SortDirection::Descending)) if c == col => None,
        _ => Some((col, SortDirection::Ascending)),
    }
}

/// Sort key following Excel's order: numbers (and dates) < text < FALSE < TRUE
/// < errors, with blanks always last regardless of direction.
enum SortKey {
    Number(f64),
    Text(String),
    Bool(bool),
    Error,
    Blank,
}

impl SortKey {
    fn from_cell(cell: &CellValue) -> Self {
        match cell {
            CellValue::Empty => SortKey::Blank,
            CellValue::String(s) if s.is_empty() => SortKey::Blank,
            CellValue::String(s) => SortKey::Text(s.to_lowercase()),
            CellValue::Int(i) => SortKey::Number(*i as f64),
            CellValue::Float(f) | CellValue::DateTime(f) => SortKey::Number(*f),
            CellValue::Bool(b) => SortKey::Bool(*b),
            CellValue::Error(_) => SortKey::Error,
        }
    }

    fn rank(&self) -> u8 {
        match self {
            SortKey::Number(_) => 0,
            SortKey::Text(_) => 1,
            SortKey::Bool(_) => 2,
            SortKey::Error => 3,
            SortKey::Blank => 4,
        }
    }

    /// Ascending comparison of two non-blank keys.
    fn cmp_values(&self, other: &Self) -> Ordering {
        match (self, other) {
            (SortKey::Number(a), SortKey::Number(b)) => a.total_cmp(b),
            (SortKey::Text(a), SortKey::Text(b)) => a.cmp(b),
            (SortKey::Bool(a), SortKey::Bool(b)) => a.cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

/// Stable sort of data-row indices by their cell values in one column.
pub fn sort_order(cells: &[CellValue], direction: SortDirection) -> Vec<usize> {
    let keys: Vec<SortKey> = cells.iter().map(SortKey::from_cell).collect();
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by(|&a, &b| {
        let (ka, kb) = (&keys[a], &keys[b]);
        match (matches!(ka, SortKey::Blank), matches!(kb, SortKey::Blank)) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => match direction {
                SortDirection::Ascending => ka.cmp_values(kb),
                SortDirection::Descending => kb.cmp_values(ka),
            },
        }
    });
    order
}

struct RowSort {
    col: usize,
    direction: SortDirection,
    /// Display row -> data row.
    order: Vec<usize>,
    /// Data row -> display row.
    position: Vec<usize>,
    /// Rows in display order, starting at display row `start_row`.
    cache: Option<RowCache>,
}

/// Sheet data as displayed: the underlying source, optionally reordered by a
/// column sort. Row indices passed in and out are display rows.
pub(crate) struct SheetView {
    source: SheetDataSource,
    sort: Option<RowSort>,
}

impl SheetView {
    pub const SORTED_CACHE_SIZE: usize = 200;

    pub fn new(source: SheetDataSource) -> Self {
        Self { source, sort: None }
    }

    pub fn headers(&self) -> &[String] {
        self.source.headers()
    }

    pub fn width(&self) -> usize {
        self.source.width()
    }

    pub fn height(&self) -> usize {
        self.source.height()
    }

    pub fn sort(&self) -> Option<(usize, SortDirection)> {
        self.sort.as_ref().map(|s| (s.col, s.direction))
    }

    /// Apply a sort, or restore file order with `None`.
    pub fn set_sort(&mut self, sort: Option<(usize, SortDirection)>) {
        self.sort = sort
            .filter(|&(col, _)| col < self.width())
            .map(|(col, direction)| {
                let cells: Vec<CellValue> = (0..self.height())
                    .map(|row| self.source.cell_value(row, col))
                    .collect();
                let order = sort_order(&cells, direction);
                let mut position = vec![0; order.len()];
                for (display, &data) in order.iter().enumerate() {
                    position[data] = display;
                }
                RowSort {
                    col,
                    direction,
                    order,
                    position,
                    cache: None,
                }
            });
    }

    /// Display row -> data row (file order).
    pub fn data_row(&self, display_row: usize) -> usize {
        match &self.sort {
            Some(s) => s.order.get(display_row).copied().unwrap_or(display_row),
            None => display_row,
        }
    }

    /// Data row (file order) -> display row.
    pub fn display_row(&self, data_row: usize) -> usize {
        match &self.sort {
            Some(s) => s.position.get(data_row).copied().unwrap_or(data_row),
            None => data_row,
        }
    }

    pub fn get_rows(
        &mut self,
        start: usize,
        count: usize,
    ) -> (&[Vec<CellValue>], &[Vec<Option<String>>]) {
        let Some(sort) = &mut self.sort else {
            return self.source.get_rows(start, count);
        };

        let height = sort.order.len();
        if start >= height {
            return (&[], &[]);
        }
        let count = count.min(height - start);

        let covered = sort
            .cache
            .as_ref()
            .is_some_and(|c| start >= c.start_row && start + count <= c.start_row + c.rows.len());
        if !covered {
            let cache_start = start.saturating_sub(Self::SORTED_CACHE_SIZE / 4);
            let cache_end = (cache_start + Self::SORTED_CACHE_SIZE)
                .max(start + count)
                .min(height);
            let (rows, formulas) = sort.order[cache_start..cache_end]
                .iter()
                .map(|&data_row| self.source.get_row(data_row))
                .unzip();
            sort.cache = Some(RowCache {
                start_row: cache_start,
                rows,
                formulas,
            });
        }

        match &sort.cache {
            Some(c) => {
                let offset = start - c.start_row;
                (
                    &c.rows[offset..offset + count],
                    &c.formulas[offset..offset + count],
                )
            }
            None => (&[], &[]),
        }
    }

    pub fn get_cell(&mut self, row: usize, col: usize) -> (Option<CellValue>, Option<String>) {
        if self.sort.is_none() {
            return self.source.get_cell(row, col);
        }
        let (rows, formulas) = self.get_rows(row, 1);
        let cell = rows.first().and_then(|r| r.get(col)).cloned();
        let formula = formulas
            .first()
            .and_then(|r| r.get(col))
            .and_then(|f| f.clone());
        (cell, formula)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> CellValue {
        CellValue::String(v.to_string())
    }

    fn mixed() -> Vec<CellValue> {
        vec![
            s("banana"),                     // 0
            CellValue::Empty,                // 1
            CellValue::Int(10),              // 2
            CellValue::Bool(true),           // 3
            s("Apple"),                      // 4
            CellValue::Error("#N/A".into()), // 5
            CellValue::Float(2.5),           // 6
            CellValue::Bool(false),          // 7
            CellValue::DateTime(45000.0),    // 8
        ]
    }

    #[test]
    fn ascending_follows_excel_type_order() {
        assert_eq!(
            sort_order(&mixed(), SortDirection::Ascending),
            vec![6, 2, 8, 4, 0, 7, 3, 5, 1]
        );
    }

    #[test]
    fn descending_reverses_values_but_keeps_blanks_last() {
        assert_eq!(
            sort_order(&mixed(), SortDirection::Descending),
            vec![5, 3, 7, 0, 4, 8, 2, 6, 1]
        );
    }

    #[test]
    fn ties_keep_file_order_in_both_directions() {
        let cells = vec![s("b"), s("a"), s("B"), s("A"), CellValue::Empty, s("")];
        assert_eq!(
            sort_order(&cells, SortDirection::Ascending),
            vec![1, 3, 0, 2, 4, 5]
        );
        assert_eq!(
            sort_order(&cells, SortDirection::Descending),
            vec![0, 2, 1, 3, 4, 5]
        );
    }

    /// Lazy source with a header row, column A = row number (1..=height),
    /// column B = (height - row) so B sorts in reverse file order, and a
    /// formula on every row of column B.
    fn lazy_view(height: usize) -> SheetView {
        use crate::workbook::LazySheetData;
        use calamine::{Data, Range};

        let mut range: Range<Data> = Range::new((0, 0), (height as u32, 1));
        let mut formulas: Range<String> = Range::new((1, 1), (height as u32, 1));
        range.set_value((0, 0), Data::String("a".into()));
        range.set_value((0, 1), Data::String("b".into()));
        for r in 1..=height {
            range.set_value((r as u32, 0), Data::Int(r as i64));
            range.set_value((r as u32, 1), Data::Int((height - r) as i64));
            formulas.set_value((r as u32, 1), format!("={height}-A{}", r + 1));
        }
        SheetView::new(SheetDataSource::Lazy {
            data: LazySheetData::from_range_with_formulas(range, Some(formulas), false),
            cache: None,
            cache_size: 10,
            no_header: false,
        })
    }

    #[test]
    fn sorted_view_reorders_rows_and_formulas() {
        let mut view = lazy_view(500);
        view.set_sort(Some((1, SortDirection::Ascending)));

        let (rows, formulas) = view.get_rows(0, 3);
        let col_a: Vec<String> = rows.iter().map(|r| r[0].to_raw_string()).collect();
        assert_eq!(col_a, ["500", "499", "498"]);
        assert_eq!(formulas[0][1].as_deref(), Some("=500-A501"));

        // Request spanning beyond the sorted cache must come back whole.
        let (rows, _) = view.get_rows(150, 300);
        assert_eq!(rows.len(), 300);
        assert_eq!(rows[299][0].to_raw_string(), "51");

        let (cell, formula) = view.get_cell(499, 1);
        assert_eq!(cell.unwrap().to_raw_string(), "499");
        assert_eq!(formula.as_deref(), Some("=500-A2"));
    }

    #[test]
    fn sorted_view_maps_rows_both_ways() {
        let mut view = lazy_view(20);
        view.set_sort(Some((1, SortDirection::Ascending)));
        for display in 0..20 {
            assert_eq!(view.data_row(display), 19 - display);
            assert_eq!(view.display_row(view.data_row(display)), display);
        }

        view.set_sort(None);
        assert_eq!(view.data_row(3), 3);
        let (rows, _) = view.get_rows(0, 1);
        assert_eq!(rows[0][0].to_raw_string(), "1");
    }

    #[test]
    fn next_sort_cycles_and_resets_on_new_column() {
        let asc = Some((2, SortDirection::Ascending));
        let desc = Some((2, SortDirection::Descending));
        assert_eq!(next_sort(None, 2), asc);
        assert_eq!(next_sort(asc, 2), desc);
        assert_eq!(next_sort(desc, 2), None);
        assert_eq!(next_sort(desc, 3), Some((3, SortDirection::Ascending)));
    }
}
