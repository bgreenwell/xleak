# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Fix clipboard copy on pure-Wayland Linux setups (e.g., niri, sway) by enabling arboard's `wayland-data-control` feature ([#45](https://github.com/bgreenwell/xleak/issues/45))

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
