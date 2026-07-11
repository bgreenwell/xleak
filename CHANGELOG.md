# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- OSC 52 clipboard support: `c`/`C` now copy via OSC 52 (works over SSH) in addition to the system clipboard
- CSV/TSV support: read and interactively view `.csv`/`.tsv` files as a single sheet (behind the default-on `csv` feature)
- `--csv-delimiter` option to override the inferred CSV/TSV field delimiter
- `--no-header` flag to treat first row as data (generates A, B, C... column headers)
- `--no-column-id` flag to hide the column-letter row (A, B, C...) in interactive mode
- `--no-row-id` flag to hide the row-number column in interactive mode
- Per-sheet cursor position memory: switching back to a sheet restores the previous cell and scroll position
- `--color`/`--no-color` flags to force or disable colored output
- `--delimiter` option for custom CSV/text export separators (e.g., `-d ";"`)
- File type detection with helpful error messages for unknown formats
- OLE2 (.xls) magic byte detection in file type checker
- Integration tests covering CLI, exports, tables, CSV/TSV, edge cases, and error handling

### Changed
- Interactive mode shows a sticky column-letter row and row-number column
- Sheet name (top-left) and `rows×cols` dimensions (top-right) are shown on the table's top border, color-differentiated from the headers
- Info bar left side shows the cell address (plus the live search query while searching); theme and key hints are right-aligned
- Status bar shows the current cell value, wrapping onto extra lines (up to 3/4 of the terminal height) when too long
- Upgraded dependencies: `calamine` 0.34→0.35, `ratatui` 0.29→0.30, `crossterm` 0.28→0.29, `toml` 0.8→1.1

### Fixed
- Formula display misaligned by one row with `--no-header` ([#50](https://github.com/bgreenwell/xleak/issues/50))
- Datetimes just before midnight rendered as `24:00:00` instead of rolling to the next day ([#54](https://github.com/bgreenwell/xleak/issues/54))
- Sheets wider than 100 columns rendered every column at zero width in interactive mode without `-H`
- Search-match highlighting lagged on large sheets with many matches
- Jump-mode cell addresses with 14+ letters overflowed instead of being rejected
- Auto-sized column widths (`-H`) over-counted multibyte UTF-8 content
- Terminal left in raw mode when TUI setup failed or a panic unwound ([#58](https://github.com/bgreenwell/xleak/issues/58))
- Control characters in cells could inject terminal escape sequences via the non-interactive table view ([#59](https://github.com/bgreenwell/xleak/issues/59))
- CSV export did not quote the header row ([#52](https://github.com/bgreenwell/xleak/issues/52))
- JSON export produced invalid JSON for quotes, backslashes, newlines, and non-finite floats ([#53](https://github.com/bgreenwell/xleak/issues/53))
- TUI search skipped most rows on lazy-loaded sheets (>1000 rows) ([#51](https://github.com/bgreenwell/xleak/issues/51))
- Help popup (`?`) is now scrollable (↑↓/PageUp/PageDown/Home) so it stays usable on short terminals; the title shows a `[x/y]` scroll indicator
- `display_table_data` now respects `--max-width` CLI flag instead of hardcoded 30
- Display corruption when switching sheets in interactive mode (removed `eprintln!` writes during raw mode)
- macOS: silence NSPasteboard stderr diagnostics during clipboard writes that corrupted the TUI

## [0.2.6] - 2026-05-24

### Added
- NetBSD installation via `pkgin install xleak` (thanks [@0323pin](https://github.com/0323pin)! [#40](https://github.com/bgreenwell/xleak/pull/40))
- Automated AUR, WinGet, and Scoop publishing via GitHub Actions
- Warn when formula cells are blank due to uncached xlsx values (`NOTE:` before table, suggests `--formulas` or re-saving in Excel/LibreOffice)

### Changed
- Upgrade `calamine` 0.26 → 0.34 and `dirs` 5.0 → 6.0 for Debian packaging compatibility (thanks [@nadzyah](https://github.com/nadzyah)! [#43](https://github.com/bgreenwell/xleak/pull/43))
- Nix flake version now read dynamically from Cargo.toml; homepage URL fixed
- Replaced `prettytable-rs` with `comfy-table` for non-interactive output, enabling correct multiline cell wrapping with `--wrap` ([#44](https://github.com/bgreenwell/xleak/pull/44))
- Non-interactive table output: green bold headers, red errors, green formula-mode cells

### Fixed
- CSV and text export wrote large numbers with thousands separators (e.g. `"18,441,600,422"` instead of `18441600422`), making them unparseable as numbers ([#34](https://github.com/bgreenwell/xleak/issues/34))
- AUR `xleak-bin` PKGBUILD: `package()` missing `cd "$srcdir/..."` caused install failures
- `?` help keybinding not firing on macOS — macOS terminals omit SHIFT for symbol chars
- Formulas fixture: `Formula` column now shows expression text; `Result` holds the live formula

## [0.2.5] - 2025-12-04

### Fixed
- Help popup not appearing on Windows - `?` key now correctly expects SHIFT modifier (thanks [@aarif](https://github.com/aarif)! [#27](https://github.com/bgreenwell/xleak/issues/27))
- VIM mode `$` keybinding now correctly expects SHIFT modifier on Windows

### Added
- Automated crates.io publishing via custom GitHub Action for all future releases

## [0.2.4] - 2025-12-04

### Fixed
- Time precision issue causing seconds to be off by 1 due to floating point truncation (thanks [@Xuquansheng](https://github.com/Xuquansheng)! [#25](https://github.com/bgreenwell/xleak/issues/25))

### Changed
- Enhanced installation documentation with Scoop (Windows), AUR (Arch Linux), shell/PowerShell installers, and MSI details
- Condensed AGENTS.md from 460 to 117 lines for better maintainability

## [0.2.3] - 2025-12-03

### Fixed
- Date display off by one day - corrected Excel epoch from December 30 to December 31, 1899 (thanks [@Xuquansheng](https://github.com/Xuquansheng)! [#25](https://github.com/bgreenwell/xleak/issues/25))

### Changed
- Consolidated test fixtures from 6 files to 3 standardized files (test_comprehensive.xlsx, test_large.xlsx, test_tables.xlsx)

### Added
- Release checklist and GitHub issue templates (Bug Report, Feature Request, Release)

## [0.2.0] - 2025-12-03

### Changed
- Migrated to cargo-dist for automated multi-platform releases
- Release process now supports shell/PowerShell installers and Homebrew tap updates

## [0.1.1] - 2025-12-03

### Added
- Configuration file support via TOML at `~/.config/xleak/config.toml` (thanks [@izelnakri](https://github.com/izelnakri) for the suggestion! [#1](https://github.com/bgreenwell/xleak/issues/1))
- Six built-in color themes: Default, Dracula, Solarized Dark/Light, GitHub Dark, Nord
- VIM keybinding profile with hjkl navigation, gg/G jumps, and yank operations
- Custom keybinding overrides for 23 different actions
- `--config` flag to specify custom configuration file location
- Excel Table support (.xlsx only) with `--list-tables` and `--table` flags (thanks [@jgranduel](https://github.com/jgranduel)! [#18](https://github.com/bgreenwell/xleak/issues/18), [#21](https://github.com/bgreenwell/xleak/pull/21))
- Horizontal scrolling mode with auto-sized columns via `-H` flag (thanks [@YannickHerrero](https://github.com/YannickHerrero)! [#13](https://github.com/bgreenwell/xleak/pull/13))
- Scrollable cell detail popup for viewing multi-line cells (thanks [@ket000](https://github.com/ket000)! [#16](https://github.com/bgreenwell/xleak/issues/16))
- MIT License (thanks [@hardBSDk](https://github.com/hardBSDk) and [@hwpplayer1](https://github.com/hwpplayer1)! [#6](https://github.com/bgreenwell/xleak/issues/6))

### Changed
- Help screen now includes configuration information

### Fixed
- UTF-8 character boundary panic with multi-byte characters like German umlauts (thanks [@steffenbusch](https://github.com/steffenbusch)! [#11](https://github.com/bgreenwell/xleak/issues/11), [#15](https://github.com/bgreenwell/xleak/pull/15))
- VIM key bindings for `Shift+G` and `$` not working properly (thanks [@hungltth](https://github.com/hungltth)! [#20](https://github.com/bgreenwell/xleak/pull/20))
- Nix installation from GitHub by adding missing `flake.lock` (thanks [@senorsmile](https://github.com/senorsmile)! [#17](https://github.com/bgreenwell/xleak/issues/17))
- Double keypress issue on Windows by filtering key release events (thanks [@clindholm](https://github.com/clindholm)! [#2](https://github.com/bgreenwell/xleak/issues/2), [#4](https://github.com/bgreenwell/xleak/pull/4))
- Needless borrow in table lookup (clippy warning)

## [0.1.0] - 2025-01-08

### Added
- Initial release of xleak
- Interactive TUI mode with ratatui
- Support for multiple Excel formats (.xlsx, .xls, .xlsm, .xlsb, .ods)
- Search functionality across sheets
- Formula display mode
- Export to CSV, JSON, and text formats
- Lazy loading for large files
- Sheet selection
- Row limit option
- Cross-platform support (Linux, macOS, Windows)

[Unreleased]: https://github.com/greenwbm/xleak/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/greenwbm/xleak/releases/tag/v0.1.0
