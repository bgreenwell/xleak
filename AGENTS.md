# xleak

Excel terminal viewer written in Rust with TUI, search, formulas, and export capabilities.

**Stack:** Rust 2024, calamine, clap, ratatui + crossterm, anyhow, comfy-table, arboard, chrono  
**Formats:** `.xlsx`, `.xls`, `.xlsm`, `.xlsb`, `.ods`  
**Key files:** `main.rs`, `workbook.rs`, `tui.rs`, `display.rs` in `src/`

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
- `workbook.rs` — Excel I/O, data extraction (calamine)
- `tui.rs` — Interactive TUI state and rendering (ratatui)
- `display.rs` — Non-interactive output (terminal, CSV, JSON, text) via comfy-table

## Code Style

- Fix all `cargo clippy` warnings; run `cargo fmt` before committing
- Error handling: `anyhow::Result<T>` with `.context()` for messages
- Comments: only when "why" is non-obvious; doc comments for public APIs
- `CellValue` enum: exhaustive pattern matching required
- Use `--release` for performance testing; use `-n` to limit rows on large files

## Common Patterns

- **New CLI option:** field on `Cli` in `main.rs`, clap macros, handle in `main()`
- **New export format:** `export_<format>()` in `display.rs`, match arm in `main()`
- **Fix display:** `display_table()` in `display.rs`, test with DataTypes sheet
- **New cell type:** `CellValue` enum in `workbook.rs`, impl `Display`, update `datatype_to_cellvalue()`
- **New built-in theme:** add factory method on `ColorScheme` in `tui.rs`, add entry to `builtin_themes()`
- **Custom themes:** `[[theme.custom]]` in config.toml, parsed via `CustomTheme` in `config.rs`, resolved by `resolve_themes()` in `tui.rs`. Supports `inherits`, `foreground`/`background` aliases, and all 19 `ColorScheme` fields

## Development

Conventional commits: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.  
Feature branches → PR to `main`. Direct commits: releases, hotfixes, minor docs only.

**PR checklist:**
- [ ] Compiles, no clippy warnings, `cargo fmt` clean
- [ ] Tested with fixtures (multiple formats: .xlsx, .xls, .ods)
- [ ] README.md updated (user-facing) or AGENTS.md (architecture changes)
- [ ] Concise entry added to CHANGELOG.md under `[Unreleased]`

**Changelog style:** One line per item, no filler words. Bad: `"Formula cells are now detected and a warning is shown to inform users that..."`. Good: `"Warn when formula cells are blank due to uncached xlsx values"`.

## Release

All distribution channels automated via cargo-dist. See [RELEASE_CHECKLIST.md](./RELEASE_CHECKLIST.md).

- `.github/workflows/release.yml` — GitHub Releases, Homebrew, crates.io
- `.github/workflows/publish-scoop.yml` — Scoop
- `.github/workflows/publish-aur.yml` — AUR
- `.github/workflows/publish-winget.yml` — WinGet

Check `.planning/` (untracked) for planning docs before starting large features.
