# xleak

@../CLAUDE.md

Excel terminal viewer written in Rust with TUI, search, formulas, and export capabilities.

**Stack:** Rust 2024, calamine, clap, ratatui + crossterm, anyhow, comfy-table, arboard, chrono, csv (optional, `csv` feature)  
**Formats:** `.xlsx`, `.xls`, `.xlsm`, `.xlsb`, `.ods`, `.csv`/`.tsv` (with `csv` feature)  
**Key files:** `main.rs`, `workbook.rs`, `csv.rs`, `tui/` (modular), `display.rs`, `cli.rs`, `config.rs`, `utils.rs` in `src/`

## Commands

```bash
cargo fmt && cargo clippy && cargo build --release
cargo run -- tests/fixtures/test_comprehensive.xlsx -i
cargo run -- tests/fixtures/test_comprehensive.xlsx --sheet Formulas --export csv
cd tests/fixtures && uv run python generate_all_tests.py   # regenerate fixtures
cargo install --path .                                      # install globally
```

## Architecture

- `main.rs` — CLI parsing, orchestration
- `workbook.rs` — Excel/CSV I/O, data extraction; `Workbook` wraps a `Backend` enum (calamine `Sheets` or in-memory CSV)
- `csv.rs` — CSV/TSV parsing into a single-sheet `SheetData`/`LazySheetData` (behind `csv` feature)
- `tui/` — Interactive TUI: `theme.rs` (themes/colors), `state.rs` (app state), `rendering.rs` (draw), `event.rs` (event loop), `clipboard.rs` (OSC 52 + system clipboard)
- `display/` — Non-interactive output: `sheet.rs` (SheetData table), `table.rs` (TableData table), `export.rs` (CSV/JSON/text) via comfy-table
- `cli.rs` — CLI argument definitions (clap)
- `config.rs` — Configuration file loading (TOML)
- `utils.rs` — File type detection (magic bytes)

## Code Style

- Comments: only when "why" is non-obvious; doc comments for public APIs
- `CellValue` enum: exhaustive pattern matching required
- Use `--release` for performance testing; use `-n` to limit rows on large files

## Common Patterns

- **New CLI option:** field on `Cli` in `cli.rs`, then handle it in `main()`
- **New export format:** `export_<format>()` in `display/export.rs`, match arm in `main()`
- **Fix display:** `display_table()` in `display/sheet.rs`, test with DataTypes sheet
- **New cell type:** `CellValue` enum in `workbook.rs`, impl `Display`, update `datatype_to_cellvalue()`

## Development

`devel` is the default and integration branch; `main` tracks releases.
Never commit directly to either — branch off `devel` (`feature/...`,
`fix/...`, `chore/...`), open a PR to `devel`, and merge after CI is green.
Releases merge `devel` into `main` and tag `vX.Y.Z` there (see
`RELEASE_CHECKLIST.md`).

**PR checklist:**
- [ ] Tested with fixtures (multiple formats: .xlsx, .xls, .ods)
- [ ] README.md updated (user-facing) or AGENTS.md (architecture changes)
- [ ] Concise entry added to CHANGELOG.md under `[Unreleased]`

## Release

All distribution channels automated via cargo-dist. See [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md).

- `.github/workflows/release.yml` — GitHub Releases, Homebrew, crates.io
- `.github/workflows/publish-scoop.yml` — Scoop
- `.github/workflows/publish-aur.yml` — AUR
- `.github/workflows/publish-winget.yml` — WinGet

Check `.planning/` (untracked) for planning docs before starting large features.
