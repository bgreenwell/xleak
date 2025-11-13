# Merged Cells Support in xleak

## Problem Statement

Merged cells are a very common Excel feature where multiple cells are combined into a single cell, typically used for:
- Headers spanning multiple columns (e.g., "Q1 Sales" across Jan/Feb/Mar)
- Section labels spanning rows
- Visual grouping and formatting

**Current State:** xleak doesn't handle merged cells. When viewing files with merged cells:
- Only the top-left cell shows the value
- Other cells in the merged range appear empty
- No visual indication that cells are merged
- Users lose context about table structure

## Technical Context

### Calamine Support (v0.26+)

The underlying `calamine` crate **does support merged cells**:

```rust
// Available API
workbook.load_merged_regions()?;
let regions = workbook.merged_regions_by_sheet("Sheet1");
// Returns: Vec<(sheet_name, sheet_path, Dimensions)>
// where Dimensions = (start_row, start_col, end_row, end_col)
```

### TUI Rendering Challenge

**ratatui's `Table` widget does not support cell spanning.** Terminal rendering is inherently grid-based - each character position is independent. Unlike HTML tables with `colspan`/`rowspan`, we must work within these constraints.

## Solution Options

### Option 1: Content Repetition (EASIEST - 30-45 min)

**Approach:** Duplicate the merged cell value across all cells in the merged range.

**Visual Example:**
```
┌──────────────┬──────────────┬──────────────┐
│ Q1 Sales     │ Q1 Sales     │ Q1 Sales     │  ← Same value repeated
├──────────────┼──────────────┼──────────────┤
│ Product      │ Units        │ Revenue      │
```

**Pros:**
- Very easy to implement
- No changes to rendering logic
- Content is visible and understandable
- Works immediately

**Cons:**
- Not visually accurate to Excel
- Redundant text might confuse users
- Doesn't look like a "merged" cell

**Implementation:**
1. Load merged regions when loading sheet
2. For each merged range, copy top-left value to all cells in range
3. Mark cells as "part of merge" in metadata
4. Show merge info in cell detail popup (Enter key)

---

### Option 2: Visual Border Indicators (EASY - 1-2 hours)

**Approach:** Keep content in top-left cell, but modify borders to show merging.

#### Variant 2a: Dimmed Internal Borders
```
┌──────────────┬──────────────┬──────────────┐
│ Q1 Sales     ·              ·              │  ← Dim borders inside merge
├──────────────┼──────────────┼──────────────┤
│ Product      │ Units        │ Revenue      │
```

Use `Color::DarkGray` for borders between merged cells.

#### Variant 2b: Continuation Arrows
```
┌──────────────┬──────────────┬──────────────┐
│ Q1 Sales  →  │       →      │              │  ← Arrows indicate span
├──────────────┼──────────────┼──────────────┤
│ Product      │ Units        │ Revenue      │
```

Add "→" or "─" in empty cells of merged range.

#### Variant 2c: Different Border Styles
```
┌──────────────╥──────────────╥──────────────┐
║ Q1 Sales     ║              ║              ║  ← Double borders for merged
╞══════════════╬══════════════╬══════════════╡
│ Product      │ Units        │ Revenue      │
```

Use Unicode box drawing characters:
- `═` (double horizontal)
- `║` (double vertical)
- `╬` `╞` `╡` (double intersections)

**Pros:**
- Visual distinction without custom rendering
- Relatively easy to implement
- Clear indication of merged regions
- Maintains grid structure

**Cons:**
- Still requires custom border rendering logic
- May look cluttered with many merged cells
- ratatui's Table doesn't expose border customization per-cell

**Implementation Complexity:**
- Need to track merged regions per cell
- Modify Table rendering or use custom Cell styling
- May require switching to manual grid rendering

---

### Option 3: Background Color Highlighting (EASY - 1 hour)

**Approach:** Use subtle background color to show merged cells belong together.

```
┌──────────────┬──────────────┬──────────────┐
│░Q1 Sales░░░░░│░░░░░░░░░░░░░│░░░░░░░░░░░░░│  ← Shared background
├──────────────┼──────────────┼──────────────┤
│ Product      │ Units        │ Revenue      │
```

**Implementation:**
```rust
// In cell rendering
if cell_is_part_of_merge(row, col) {
    style = style.bg(Color::Rgb(40, 40, 50)); // Subtle highlight
}
```

**Pros:**
- Very easy to implement with current architecture
- Works with existing Table widget
- Visually clear grouping
- Can combine with content repetition

**Cons:**
- Conflicts with theme color schemes
- May interfere with alternating row colors
- Doesn't clearly show which cell has the "real" value

**Best Used With:** Content repetition (Option 1)

---

### Option 4: Status Bar + Cell Detail (EASIEST - 30 min)

**Approach:** Don't change grid display, just inform users about merges.

**Status Bar:**
```
A1 | "Q1 Sales" | Merged: A1:C1 | 50 rows × 10 columns
```

**Cell Detail Popup (Enter key):**
```
┌─────────────────────────────┐
│ Cell: A1                    │
│ Value: "Q1 Sales"           │
│ Type: String                │
│ Merged Region: A1:C1        │  ← NEW
│ Formula: None               │
└─────────────────────────────┘
```

**Pros:**
- Trivial to implement
- No rendering changes needed
- Clear information for users who care
- No visual clutter

**Cons:**
- Not visible at a glance
- Users must navigate to merged cells to see info
- Doesn't solve the "empty cell" problem

**Best Used With:** Any other option as supplementary info

---

### Option 5: Hybrid Approach (RECOMMENDED - 2-3 hours)

**Combine multiple easy solutions:**

1. **Repeat content** across merged cells (Option 1)
2. **Dim the repeated text** to show it's not the "primary" cell
3. **Shared background color** for theme-aware subtle grouping (Option 3)
4. **Show merge info** in status bar and cell detail (Option 4)
5. **Cursor jumps** to top-left when navigating into merged region

**Visual Example:**
```
┌──────────────┬──────────────┬──────────────┐
│ Q1 Sales     │░Q1 Sales░░░░░│░Q1 Sales░░░░░│  ← Bold vs dim, shared bg
├──────────────┼──────────────┼──────────────┤
│ Product      │ Units        │ Revenue      │
```

**Implementation:**
```rust
struct CellMetadata {
    is_merged: bool,
    merge_region: Option<(usize, usize, usize, usize)>, // (start_row, start_col, end_row, end_col)
    is_merge_primary: bool, // True for top-left cell
}

// In rendering
if cell.is_merged && !cell.is_merge_primary {
    style = style
        .fg(colors.empty_fg)  // Dimmed color
        .bg(colors.merge_bg); // Subtle background
}
```

**Pros:**
- Best balance of clarity and simplicity
- Multiple visual cues reinforce understanding
- Works within existing architecture
- Gradual enhancement - can add features incrementally

**Cons:**
- Slightly more complex than single-solution approaches
- Need to track metadata for each cell

---

### Option 6: Custom Table Widget (COMPLEX - 8-12 hours)

**Approach:** Build a custom widget that actually spans cells visually.

**Visual Example:**
```
┌─────────────────────────────────────────────┐
│              Q1 Sales                       │  ← Actually spans
├──────────────┬──────────────┬──────────────┤
│ Product      │ Units        │ Revenue      │
```

**Major Implementation Challenges:**

1. **Layout Calculation**
   - Calculate merged cell width: sum of column widths + borders
   - Handle partial visibility at viewport edges
   - Recalculate on every scroll

2. **Custom Rendering**
   ```rust
   impl Widget for MergedTable {
       fn render(self, area: Rect, buf: &mut Buffer) {
           // Track which positions are "occupied" by merged cells
           let mut occupied: HashSet<(u16, u16)> = HashSet::new();

           // Render merged cells first
           for merge in merged_regions {
               let width = calculate_span_width(merge);
               render_merged_cell(buf, merge, width);
               mark_occupied(&mut occupied, merge);
           }

           // Render normal cells, skipping occupied positions
           for cell in cells {
               if !occupied.contains(&cell.pos) {
                   render_normal_cell(buf, cell);
               }
           }
       }
   }
   ```

3. **Border Management**
   - Suppress internal borders within merged cells
   - Handle border intersections
   - Maintain external borders

4. **Viewport/Scrolling**
   - Clip merged cells at viewport boundaries
   - Handle merged cells larger than viewport
   - Maintain state across scroll events

5. **Navigation**
   - Cursor behavior in merged regions
   - Search highlighting across spans
   - Copy/paste semantics

6. **Edge Cases**
   - Overlapping merged regions (invalid but might exist)
   - Merged cells at sheet boundaries
   - Vertical merges (row spanning)
   - Both horizontal and vertical merges

**Pros:**
- Visually accurate to Excel
- Professional appearance
- Handles complex merge patterns
- Full control over rendering

**Cons:**
- Significant development time (8-12 hours)
- Ongoing maintenance burden
- Complex debugging
- May need to maintain custom fork of ratatui Table
- Performance considerations for large sheets

**Estimated Effort:**
- Phase 1: Basic horizontal merges (4-6 hours)
- Phase 2: Viewport integration (2-3 hours)
- Phase 3: Navigation & interaction (2-3 hours)
- Phase 4: Vertical merges & edge cases (2-3 hours)
- **Total: 10-15 hours**

---

## Recommendations

### Immediate Term (v0.1.1)
**Implement Option 5 (Hybrid Approach)** - 2-3 hours

This gives users:
- ✅ All content is visible (no empty cells)
- ✅ Clear visual indication of merging
- ✅ Detailed information when needed
- ✅ Works with all themes
- ✅ No major architectural changes

### Medium Term (v0.2.0)
**Add Option 2c (Border Styles)** - 2 hours additional

If users want better visual distinction:
- Use Unicode box drawing for merged regions
- Custom border rendering per-cell
- May require replacing ratatui's Table with manual grid

### Long Term (v1.0+)
**Consider Option 6 (Custom Widget)** - if demanded

Only pursue if:
- Users specifically request true cell spanning
- Complex merged layouts are common in target use cases
- Team has bandwidth for ongoing maintenance

## Implementation Priority

1. **Option 4** (Status bar info) - 30 min - Do FIRST for quick win
2. **Option 1** (Content repetition) - 30 min - Add immediately after
3. **Option 3** (Background color) - 1 hour - Polish the visual
4. **Option 2b** (Arrows) - 30 min - Optional additional clarity

**Total: 2.5-3 hours for complete basic support**

## Technical Notes

### Data Structure

```rust
// In workbook.rs
pub struct SheetData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<CellValue>>,
    pub formulas: Vec<Vec<Option<String>>>,
    pub merged_regions: Vec<(usize, usize, usize, usize)>, // NEW: (start_row, start_col, end_row, end_col)
    pub width: usize,
    pub height: usize,
}

impl SheetData {
    fn is_cell_merged(&self, row: usize, col: usize) -> Option<&(usize, usize, usize, usize)> {
        self.merged_regions.iter().find(|(sr, sc, er, ec)| {
            row >= *sr && row <= *er && col >= *sc && col <= *ec
        })
    }

    fn is_merge_primary(&self, row: usize, col: usize) -> bool {
        self.merged_regions.iter().any(|(sr, sc, _, _)| {
            row == *sr && col == *sc
        })
    }
}
```

### Calamine Integration

```rust
// In workbook.rs - modify load_sheet()
pub fn load_sheet(&mut self, name: &str) -> Result<SheetData> {
    let range = self.sheets.worksheet_range(name)?;
    let formula_range = self.sheets.worksheet_formula(name).ok();

    // NEW: Load merged regions
    self.sheets.load_merged_regions()?;
    let merged = self.sheets.merged_regions_by_sheet(name);

    let mut sheet_data = SheetData::from_range_with_formulas(range, formula_range);

    // Convert calamine Dimensions to our format
    sheet_data.merged_regions = merged
        .iter()
        .map(|(_, _, dim)| {
            (dim.start.0, dim.start.1, dim.end.0, dim.end.1)
        })
        .collect();

    // Apply content repetition
    sheet_data.expand_merged_cells();

    Ok(sheet_data)
}
```

### Theme Integration

```rust
// In tui.rs - add to ColorScheme
pub struct ColorScheme {
    // ... existing fields
    pub merge_secondary_fg: Color,  // Color for repeated content
    pub merge_bg: Option<Color>,    // Shared background for merged cells
}
```

## Testing Strategy

Create test files with:
1. Simple horizontal merge (A1:C1)
2. Simple vertical merge (A1:A3)
3. Both horizontal and vertical (A1:C3)
4. Multiple non-overlapping merges
5. Merged cells at edges/corners
6. Single-cell "merges" (A1:A1)
7. Large merged regions (A1:Z100)

Test scenarios:
- Navigation through merged regions
- Search within merged cells
- Copy/paste behavior
- Theme switching with merged cells
- Scrolling with merged cells at viewport edge

## Future Enhancements

- **Merge detection warnings**: Alert if file has complex merges that may not display perfectly
- **Export preservation**: When exporting to CSV, add metadata about merges
- **Formula evaluation**: Handle formulas that reference merged cells
- **Merge creation**: (Future) Allow creating merged cells in edit mode
