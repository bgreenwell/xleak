use crate::config::{CustomTheme, ThemeConfig};
use crate::workbook::CellValue;
use anyhow::Result;
use ratatui::style::Color;

/// Single source of truth for the 20 customizable color fields.
///
/// Each row is `[field_name, kind, alias]` where:
///   kind:  `plain` = `Color` in ColorScheme, `opt` = `Option<Color>`
///   alias: `fg` = set by the `foreground` alias, `bg` = by `background`, `none` = excluded
///
/// This table drives `CustomTheme` (config.rs), `apply_custom_fields`, and
/// `uses_rgb` so that adding a `ColorScheme` field in one place can't silently
/// miss the others.
macro_rules! color_field_table {
    ($mac:ident) => {
        $mac! {
            [string_fg,          plain, fg]
            [number_fg,          plain, fg]
            [bool_fg,            plain, fg]
            [datetime_fg,        plain, fg]
            [error_fg,           plain, fg]
            [empty_fg,           plain, fg]
            [header_fg,          plain, fg]
            [header_bg,          opt,   bg]
            [current_cell_fg,    plain, none]
            [current_cell_bg,    plain, none]
            [current_row_bg,     plain, none]
            [current_col_fg,     plain, none]
            [alternating_row_bg, opt,   bg]
            [search_match_fg,    plain, none]
            [search_match_bg,    plain, none]
            [current_search_fg,  plain, none]
            [current_search_bg,  plain, none]
            [border_fg,          plain, fg]
            [status_bar_fg,      plain, fg]
            [status_bar_bg,      opt,   bg]
        }
    };
}
pub(crate) use color_field_table;

/// A color scheme with the name users refer to it by, in config and in the
/// status bar. Built-ins and `[[theme.custom]]` entries are the same shape, so
/// theme cycling doesn't care where a theme came from.
#[derive(Debug, Clone)]
pub struct NamedTheme {
    pub name: String,
    pub colors: ColorScheme,
    pub custom: bool,
}

use crate::utils::normalize_name;

/// Returns the built-in themes in cycle order.
pub fn builtin_themes() -> Vec<NamedTheme> {
    [
        ("Default", ColorScheme::default_theme()),
        ("Dracula", ColorScheme::dracula()),
        ("Solarized Dark", ColorScheme::solarized_dark()),
        ("Solarized Light", ColorScheme::solarized_light()),
        ("GitHub Dark", ColorScheme::github_dark()),
        ("Nord", ColorScheme::nord()),
    ]
    .into_iter()
    .map(|(name, colors)| NamedTheme {
        name: name.into(),
        colors,
        custom: false,
    })
    .collect()
}

/// Every theme available this run, plus which one is active.
///
/// Invariants: `themes` is never empty (it always contains the built-ins) and
/// `current` is always a valid index, so `current()` cannot panic.
#[derive(Debug, Clone)]
pub struct ThemeSet {
    themes: Vec<NamedTheme>,
    current: usize,
}

impl ThemeSet {
    /// Build the theme list from config and select the startup theme.
    ///
    /// When `override_name` is `Some`, it takes precedence over the config
    /// default and is a **hard error** if not found (it came from `--theme`,
    /// so the user explicitly asked for it). An unknown config default is
    /// recoverable, so it comes back as a warning instead.
    pub fn resolve(
        config: &ThemeConfig,
        override_name: Option<&str>,
    ) -> Result<(Self, Vec<String>)> {
        let themes = resolve_themes(&config.custom)?;
        let mut warnings = Vec::new();

        let startup_name = override_name.unwrap_or(&config.default);

        let current = match find_index(&themes, startup_name) {
            Some(idx) => idx,
            None if override_name.is_some() => {
                let available = Self::format_names(&themes);
                anyhow::bail!(
                    "Unknown theme '{}'. Available themes: {}",
                    startup_name,
                    available
                );
            }
            None => {
                warnings.push(format!(
                    "theme '{}' not found, falling back to 'Default'",
                    startup_name
                ));
                0
            }
        };

        Ok((Self { themes, current }, warnings))
    }

    fn format_names(themes: &[NamedTheme]) -> String {
        themes
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Colors for the active theme.
    pub fn current(&self) -> &ColorScheme {
        &self.themes[self.current].colors
    }

    /// Display name of the active theme.
    pub fn current_name(&self) -> &str {
        &self.themes[self.current].name
    }

    /// Advance to the next theme, wrapping. Built-ins come first, then customs
    /// in the order they appear in config.
    pub fn cycle(&mut self) {
        self.current = (self.current + 1) % self.themes.len();
    }
}

/// Case- and space-insensitive lookup of a theme by name.
fn find_index(themes: &[NamedTheme], name: &str) -> Option<usize> {
    let needle = normalize_name(name);
    themes
        .iter()
        .position(|t| normalize_name(&t.name) == needle)
}

/// Merge custom themes onto the built-ins.
///
/// Customs are resolved in config order, so a theme can only inherit from one
/// defined before it. That ordering requirement is what makes circular
/// `inherits` chains unrepresentable rather than something we have to detect.
/// A custom sharing a built-in's name replaces it in place, keeping cycle order
/// stable.
pub fn resolve_themes(custom_themes: &[CustomTheme]) -> Result<Vec<NamedTheme>> {
    let mut themes = builtin_themes();

    for custom in custom_themes {
        let colors = apply_custom_fields(resolve_base(&themes, custom)?, custom);
        match find_index(&themes, &custom.name) {
            Some(idx) => {
                themes[idx].colors = colors;
                themes[idx].custom = true;
            }
            None => themes.push(NamedTheme {
                name: custom.name.clone(),
                colors,
                custom: true,
            }),
        }
    }

    Ok(themes)
}

/// The scheme a custom theme starts from before its own fields are applied.
///
/// When `inherits` is absent, fall back to an existing theme with the same
/// name so that `name = "Dracula"` + one field inherits the built-in Dracula
/// palette rather than silently resetting every untouched field to Default.
fn resolve_base(themes: &[NamedTheme], custom: &CustomTheme) -> Result<ColorScheme> {
    let Some(ref parent_name) = custom.inherits else {
        return Ok(match find_index(themes, &custom.name) {
            Some(idx) => themes[idx].colors.clone(),
            None => ColorScheme::default_theme(),
        });
    };

    let idx = find_index(themes, parent_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Theme '{}' referenced in 'inherits' not found. \
             If it's a custom theme, make sure it appears earlier in [[theme.custom]].",
            parent_name
        )
    })?;

    Ok(themes[idx].colors.clone())
}

/// Apply a custom theme's fields over a base scheme.
///
/// The `foreground`/`background` aliases only touch elements tagged `fg`/`bg`
/// in `color_field_table!`. Anything tagged `none` (cursor, search, row
/// highlight) keeps the contrast the parent theme designed in.
fn apply_custom_fields(mut colors: ColorScheme, custom: &CustomTheme) -> ColorScheme {
    macro_rules! impl_apply {
        ( $( [$field:ident, $kind:ident, $alias:ident] )* ) => {
            // Broad-brush aliases first.
            if let Some(fg) = custom.foreground {
                $( impl_apply!(@fg colors, $field, fg, $alias); )*
            }
            if let Some(bg) = custom.background {
                $( impl_apply!(@bg colors, $field, bg, $alias); )*
            }
            // Per-field overrides on top.
            $( impl_apply!(@field colors, custom, $field, $kind); )*
        };
        (@fg $colors:ident, $field:ident, $val:ident, fg)   => { $colors.$field = $val; };
        (@fg $colors:ident, $field:ident, $val:ident, $t:ident) => {};
        (@bg $colors:ident, $field:ident, $val:ident, bg)   => { $colors.$field = Some($val); };
        (@bg $colors:ident, $field:ident, $val:ident, $t:ident) => {};
        (@field $c:ident, $cu:ident, $f:ident, plain) => { if let Some(v) = $cu.$f { $c.$f = v; } };
        (@field $c:ident, $cu:ident, $f:ident, opt)   => { if let Some(v) = $cu.$f { $c.$f = Some(v); } };
    }
    color_field_table!(impl_apply);
    colors
}

/// Convenience wrapper for `main.rs`: resolve themes + select the startup
/// theme, with `--theme` override support.
pub fn resolve_themes_from_config(
    config: &ThemeConfig,
    override_name: Option<&str>,
) -> Result<(ThemeSet, Vec<String>)> {
    ThemeSet::resolve(config, override_name)
}

/// Check whether the active theme needs truecolor and warn if the terminal
/// doesn't advertise it. Only fires for custom themes — every non-Default
/// built-in uses `Color::Rgb`, so warning unconditionally would nag users
/// who never touched their config.
pub fn truecolor_warning(themes: &ThemeSet, colorterm: Option<&str>) -> Option<String> {
    let theme = &themes.themes[themes.current];
    if !theme.custom || !theme.colors.uses_rgb() {
        return None;
    }
    match colorterm {
        Some(v) if v.eq_ignore_ascii_case("truecolor") || v.eq_ignore_ascii_case("24bit") => None,
        Some(v) => Some(format!(
            "Theme '{}' uses RGB colors, but COLORTERM is '{}' (expected 'truecolor' or '24bit'). \
             Colors may not display correctly.",
            theme.name, v
        )),
        None => Some(format!(
            "Theme '{}' uses RGB colors, but COLORTERM is not set. \
             Colors may not display correctly.",
            theme.name
        )),
    }
}

/// Color scheme for the TUI
#[derive(Debug, Clone)]
pub struct ColorScheme {
    // Cell type colors
    pub string_fg: Color,
    pub number_fg: Color,
    pub bool_fg: Color,
    pub datetime_fg: Color,
    pub error_fg: Color,
    pub empty_fg: Color,

    // UI element colors
    pub header_fg: Color,
    pub header_bg: Option<Color>,
    pub current_cell_fg: Color,
    pub current_cell_bg: Color,
    pub current_row_bg: Color,
    pub current_col_fg: Color,
    pub alternating_row_bg: Option<Color>,

    // Search colors
    pub search_match_fg: Color,
    pub search_match_bg: Color,
    pub current_search_fg: Color,
    pub current_search_bg: Color,

    // Border and status bar
    pub border_fg: Color,
    pub status_bar_fg: Color,
    pub status_bar_bg: Option<Color>,
}

impl ColorScheme {
    /// Default theme (current behavior with enhancements)
    pub fn default_theme() -> Self {
        Self {
            // Cell types
            string_fg: Color::White,
            number_fg: Color::Cyan,
            bool_fg: Color::Magenta,
            datetime_fg: Color::Green,
            error_fg: Color::Red,
            empty_fg: Color::DarkGray,

            // UI elements
            header_fg: Color::Yellow,
            header_bg: None,
            current_cell_fg: Color::White,
            current_cell_bg: Color::Blue,
            current_row_bg: Color::DarkGray,
            current_col_fg: Color::Cyan,
            alternating_row_bg: Some(Color::Rgb(25, 25, 28)),

            // Search
            search_match_fg: Color::Black,
            search_match_bg: Color::LightYellow,
            current_search_fg: Color::Black,
            current_search_bg: Color::Yellow,

            // Borders/status
            border_fg: Color::White,
            status_bar_fg: Color::White,
            status_bar_bg: None,
        }
    }

    /// Dracula theme (purple/pink aesthetic)
    pub fn dracula() -> Self {
        Self {
            // Cell types - Dracula palette
            string_fg: Color::Rgb(248, 248, 242),  // Foreground
            number_fg: Color::Rgb(189, 147, 249),  // Purple
            bool_fg: Color::Rgb(255, 121, 198),    // Pink
            datetime_fg: Color::Rgb(80, 250, 123), // Green
            error_fg: Color::Rgb(255, 85, 85),     // Red
            empty_fg: Color::Rgb(98, 114, 164),    // Comment

            // UI elements
            header_fg: Color::Rgb(139, 233, 253),    // Cyan
            header_bg: Some(Color::Rgb(68, 71, 90)), // Current line
            current_cell_fg: Color::Rgb(248, 248, 242),
            current_cell_bg: Color::Rgb(98, 114, 164), // Comment (darker)
            current_row_bg: Color::Rgb(68, 71, 90),    // Current line
            current_col_fg: Color::Rgb(139, 233, 253), // Cyan
            alternating_row_bg: Some(Color::Rgb(50, 52, 65)),

            // Search
            search_match_fg: Color::Rgb(40, 42, 54), // Background
            search_match_bg: Color::Rgb(241, 250, 140), // Yellow
            current_search_fg: Color::Rgb(40, 42, 54),
            current_search_bg: Color::Rgb(255, 184, 108), // Orange

            // Borders/status
            border_fg: Color::Rgb(98, 114, 164), // Comment
            status_bar_fg: Color::Rgb(248, 248, 242),
            status_bar_bg: Some(Color::Rgb(68, 71, 90)),
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            // Cell types - Solarized Dark
            string_fg: Color::Rgb(131, 148, 150), // Base0
            number_fg: Color::Rgb(38, 139, 210),  // Blue
            bool_fg: Color::Rgb(211, 54, 130),    // Magenta
            datetime_fg: Color::Rgb(133, 153, 0), // Green
            error_fg: Color::Rgb(220, 50, 47),    // Red
            empty_fg: Color::Rgb(88, 110, 117),   // Base01

            // UI elements
            header_fg: Color::Rgb(181, 137, 0),     // Yellow
            header_bg: Some(Color::Rgb(7, 54, 66)), // Base02
            current_cell_fg: Color::Rgb(253, 246, 227),
            current_cell_bg: Color::Rgb(88, 110, 117), // Base01
            current_row_bg: Color::Rgb(7, 54, 66),     // Base02
            current_col_fg: Color::Rgb(42, 161, 152),  // Cyan
            alternating_row_bg: Some(Color::Rgb(0, 43, 54)),

            // Search
            search_match_fg: Color::Rgb(0, 43, 54),
            search_match_bg: Color::Rgb(181, 137, 0), // Yellow
            current_search_fg: Color::Rgb(0, 43, 54),
            current_search_bg: Color::Rgb(203, 75, 22), // Orange

            // Borders/status
            border_fg: Color::Rgb(88, 110, 117),
            status_bar_fg: Color::Rgb(131, 148, 150),
            status_bar_bg: Some(Color::Rgb(7, 54, 66)),
        }
    }

    /// Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            // Cell types - Solarized Light
            string_fg: Color::Rgb(101, 123, 131), // Base00
            number_fg: Color::Rgb(38, 139, 210),  // Blue
            bool_fg: Color::Rgb(211, 54, 130),    // Magenta
            datetime_fg: Color::Rgb(133, 153, 0), // Green
            error_fg: Color::Rgb(220, 50, 47),    // Red
            empty_fg: Color::Rgb(147, 161, 161),  // Base1

            // UI elements
            header_fg: Color::Rgb(181, 137, 0),         // Yellow
            header_bg: Some(Color::Rgb(238, 232, 213)), // Base2
            current_cell_fg: Color::Rgb(0, 43, 54),     // Base02
            current_cell_bg: Color::Rgb(147, 161, 161), // Base1
            current_row_bg: Color::Rgb(238, 232, 213),  // Base2
            current_col_fg: Color::Rgb(42, 161, 152),   // Cyan
            alternating_row_bg: Some(Color::Rgb(253, 246, 227)),

            // Search
            search_match_fg: Color::Rgb(0, 43, 54),
            search_match_bg: Color::Rgb(181, 137, 0), // Yellow
            current_search_fg: Color::Rgb(253, 246, 227),
            current_search_bg: Color::Rgb(203, 75, 22), // Orange

            // Borders/status
            border_fg: Color::Rgb(147, 161, 161),
            status_bar_fg: Color::Rgb(101, 123, 131),
            status_bar_bg: Some(Color::Rgb(238, 232, 213)),
        }
    }

    /// GitHub Dark theme
    pub fn github_dark() -> Self {
        Self {
            // Cell types - GitHub Dark
            string_fg: Color::Rgb(201, 209, 217),   // fgDefault
            number_fg: Color::Rgb(121, 192, 255),   // prettylights-syntax-constant
            bool_fg: Color::Rgb(255, 125, 163),     // prettylights-syntax-entity
            datetime_fg: Color::Rgb(127, 219, 202), // prettylights-syntax-string
            error_fg: Color::Rgb(248, 81, 73),      // danger-fg
            empty_fg: Color::Rgb(110, 118, 129),    // fgMuted

            // UI elements
            header_fg: Color::Rgb(255, 199, 119), // prettylights-syntax-entity-tag
            header_bg: Some(Color::Rgb(33, 38, 45)), // canvas-subtle
            current_cell_fg: Color::Rgb(201, 209, 217),
            current_cell_bg: Color::Rgb(56, 139, 253), // accent-emphasis
            current_row_bg: Color::Rgb(33, 38, 45),    // canvas-subtle
            current_col_fg: Color::Rgb(121, 192, 255),
            alternating_row_bg: Some(Color::Rgb(22, 27, 34)),

            // Search
            search_match_fg: Color::Rgb(13, 17, 23),
            search_match_bg: Color::Rgb(187, 128, 9), // attention-emphasis
            current_search_fg: Color::Rgb(13, 17, 23),
            current_search_bg: Color::Rgb(242, 130, 33), // severe-emphasis

            // Borders/status
            border_fg: Color::Rgb(48, 54, 61), // border-default
            status_bar_fg: Color::Rgb(201, 209, 217),
            status_bar_bg: Some(Color::Rgb(33, 38, 45)),
        }
    }

    /// Nord theme (cool blue/cyan palette)
    pub fn nord() -> Self {
        Self {
            // Cell types - Nord
            string_fg: Color::Rgb(216, 222, 233),   // nord4
            number_fg: Color::Rgb(136, 192, 208),   // nord8
            bool_fg: Color::Rgb(180, 142, 173),     // nord15
            datetime_fg: Color::Rgb(163, 190, 140), // nord14
            error_fg: Color::Rgb(191, 97, 106),     // nord11
            empty_fg: Color::Rgb(76, 86, 106),      // nord3

            // UI elements
            header_fg: Color::Rgb(235, 203, 139),    // nord13
            header_bg: Some(Color::Rgb(59, 66, 82)), // nord1
            current_cell_fg: Color::Rgb(236, 239, 244),
            current_cell_bg: Color::Rgb(94, 129, 172), // nord9
            current_row_bg: Color::Rgb(59, 66, 82),    // nord1
            current_col_fg: Color::Rgb(136, 192, 208), // nord8
            alternating_row_bg: Some(Color::Rgb(46, 52, 64)),

            // Search
            search_match_fg: Color::Rgb(46, 52, 64),
            search_match_bg: Color::Rgb(235, 203, 139), // nord13
            current_search_fg: Color::Rgb(46, 52, 64),
            current_search_bg: Color::Rgb(208, 135, 112), // nord12

            // Borders/status
            border_fg: Color::Rgb(76, 86, 106), // nord3
            status_bar_fg: Color::Rgb(216, 222, 233),
            status_bar_bg: Some(Color::Rgb(59, 66, 82)),
        }
    }

    /// Whether any field uses `Color::Rgb`, which requires 24-bit color support.
    pub fn uses_rgb(&self) -> bool {
        macro_rules! impl_uses_rgb {
            ( $( [$field:ident, $kind:ident, $alias:ident] )* ) => {
                $( impl_uses_rgb!(@check self, $field, $kind) || )* false
            };
            (@check $s:ident, $f:ident, plain) => { matches!($s.$f, Color::Rgb(..)) };
            (@check $s:ident, $f:ident, opt)   => { matches!($s.$f, Some(Color::Rgb(..))) };
        }
        color_field_table!(impl_uses_rgb)
    }

    /// Get foreground color for a cell based on its value type
    pub fn cell_color(&self, cell: &CellValue) -> Color {
        match cell {
            CellValue::Empty => self.empty_fg,
            CellValue::String(_) => self.string_fg,
            CellValue::Int(_) | CellValue::Float(_) => self.number_fg,
            CellValue::Bool(_) => self.bool_fg,
            CellValue::Error(_) => self.error_fg,
            CellValue::DateTime(_) => self.datetime_fg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN_COUNT: usize = 6;

    /// Look a theme up by name so tests don't depend on cycle position.
    fn colors_of<'a>(themes: &'a [NamedTheme], name: &str) -> &'a ColorScheme {
        let idx = find_index(themes, name).unwrap_or_else(|| panic!("no theme named '{name}'"));
        &themes[idx].colors
    }

    fn custom(name: &str) -> CustomTheme {
        CustomTheme {
            name: name.into(),
            ..Default::default()
        }
    }

    // =========================================================================
    // Theme Resolution
    // =========================================================================

    #[test]
    fn resolve_themes_without_customs_returns_builtins() {
        let themes = resolve_themes(&[]).unwrap();
        assert_eq!(themes.len(), BUILTIN_COUNT);
        assert_eq!(themes[0].name, "Default");
        assert_eq!(themes[BUILTIN_COUNT - 1].name, "Nord");
    }

    #[test]
    fn custom_replaces_builtin_of_same_name_in_place() {
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("Dracula".into()),
            string_fg: Some(Color::Green),
            ..custom("Dracula")
        }])
        .unwrap();

        // Replaced, not appended, so cycle order is unchanged.
        assert_eq!(themes.len(), BUILTIN_COUNT);
        assert_eq!(themes[1].name, "Dracula");

        let c = colors_of(&themes, "Dracula");
        assert_eq!(c.string_fg, Color::Green);
        // Untouched fields still come from Dracula, not from Default.
        assert_eq!(c.number_fg, Color::Rgb(189, 147, 249));
    }

    #[test]
    fn custom_with_new_name_is_appended() {
        let themes = resolve_themes(&[custom("Brand New")]).unwrap();
        assert_eq!(themes.len(), BUILTIN_COUNT + 1);
        assert_eq!(themes[BUILTIN_COUNT].name, "Brand New");
    }

    #[test]
    fn custom_can_inherit_an_earlier_custom() {
        let themes = resolve_themes(&[
            CustomTheme {
                inherits: Some("Nord".into()),
                string_fg: Some(Color::Rgb(1, 2, 3)),
                ..custom("parent")
            },
            CustomTheme {
                inherits: Some("parent".into()),
                number_fg: Some(Color::Rgb(4, 5, 6)),
                ..custom("child")
            },
        ])
        .unwrap();

        let c = colors_of(&themes, "child");
        assert_eq!(c.string_fg, Color::Rgb(1, 2, 3), "inherited from parent");
        assert_eq!(c.number_fg, Color::Rgb(4, 5, 6), "own field");
        assert_eq!(c.bool_fg, Color::Rgb(180, 142, 173), "from Nord via parent");
    }

    #[test]
    fn inherits_unknown_theme_is_an_error() {
        let err = resolve_themes(&[CustomTheme {
            inherits: Some("NonExistent".into()),
            ..custom("Bad")
        }])
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn inheriting_a_later_custom_is_an_error() {
        // Forward references are rejected; that's what rules out cycles.
        let err = resolve_themes(&[
            CustomTheme {
                inherits: Some("second".into()),
                ..custom("first")
            },
            custom("second"),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("appears earlier"));
    }

    #[test]
    fn theme_names_match_loosely() {
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("solarizeddark".into()),
            ..custom("loose")
        }])
        .unwrap();
        assert_eq!(
            colors_of(&themes, "loose").number_fg,
            Color::Rgb(38, 139, 210),
            "'solarizeddark' should resolve to 'Solarized Dark'"
        );
    }

    #[test]
    fn override_without_inherits_falls_back_to_same_name() {
        let themes = resolve_themes(&[CustomTheme {
            string_fg: Some(Color::Green),
            ..custom("Dracula")
        }])
        .unwrap();

        let c = colors_of(&themes, "Dracula");
        assert_eq!(c.string_fg, Color::Green, "explicit override applied");
        assert_eq!(
            c.number_fg,
            Color::Rgb(189, 147, 249),
            "number_fg should come from built-in Dracula, not Default"
        );
    }

    #[test]
    fn theme_names_match_with_hyphens_and_underscores() {
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("solarized-dark".into()),
            ..custom("hyphen")
        }])
        .unwrap();
        assert_eq!(
            colors_of(&themes, "hyphen").number_fg,
            Color::Rgb(38, 139, 210),
        );

        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("github_dark".into()),
            ..custom("underscore")
        }])
        .unwrap();
        assert_eq!(
            colors_of(&themes, "underscore").number_fg,
            Color::Rgb(121, 192, 255),
        );
    }

    // =========================================================================
    // foreground / background aliases
    // =========================================================================

    #[test]
    fn foreground_alias_sets_uniform_fields_and_yields_to_explicit_fields() {
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("Default".into()),
            foreground: Some(Color::Blue),
            string_fg: Some(Color::Red),
            ..custom("AliasTest")
        }])
        .unwrap();

        let c = colors_of(&themes, "AliasTest");
        assert_eq!(c.number_fg, Color::Blue);
        assert_eq!(c.header_fg, Color::Blue);
        assert_eq!(c.border_fg, Color::Blue);
        assert_eq!(c.status_bar_fg, Color::Blue);
        assert_eq!(c.string_fg, Color::Red, "explicit field wins over alias");
    }

    #[test]
    fn background_alias_sets_uniform_fields_and_yields_to_explicit_fields() {
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("Default".into()),
            background: Some(Color::Rgb(10, 10, 10)),
            current_row_bg: Some(Color::Rgb(30, 30, 30)),
            ..custom("BgTest")
        }])
        .unwrap();

        let c = colors_of(&themes, "BgTest");
        assert_eq!(c.header_bg, Some(Color::Rgb(10, 10, 10)));
        assert_eq!(c.alternating_row_bg, Some(Color::Rgb(10, 10, 10)));
        assert_eq!(c.status_bar_bg, Some(Color::Rgb(10, 10, 10)));
        assert_eq!(c.current_row_bg, Color::Rgb(30, 30, 30), "explicit wins");
    }

    /// Regression test for the alias bug found in review: setting only
    /// `background` used to collapse `current_row_bg` and `current_cell_bg` onto
    /// it, so the cursor row became invisible. Generalised to every element whose
    /// job is to contrast against the bulk of the table.
    #[test]
    fn aliases_never_flatten_the_elements_that_provide_contrast() {
        let fg = Color::Rgb(192, 202, 245);
        let bg = Color::Rgb(26, 27, 38);
        let themes = resolve_themes(&[CustomTheme {
            inherits: Some("Default".into()),
            foreground: Some(fg),
            background: Some(bg),
            ..custom("NavTest")
        }])
        .unwrap();

        let c = colors_of(&themes, "NavTest");
        let default = ColorScheme::default_theme();

        for (label, got, inherited) in [
            ("current_row_bg", c.current_row_bg, default.current_row_bg),
            (
                "current_cell_bg",
                c.current_cell_bg,
                default.current_cell_bg,
            ),
            (
                "current_cell_fg",
                c.current_cell_fg,
                default.current_cell_fg,
            ),
            ("current_col_fg", c.current_col_fg, default.current_col_fg),
            (
                "search_match_fg",
                c.search_match_fg,
                default.search_match_fg,
            ),
            (
                "search_match_bg",
                c.search_match_bg,
                default.search_match_bg,
            ),
            (
                "current_search_fg",
                c.current_search_fg,
                default.current_search_fg,
            ),
            (
                "current_search_bg",
                c.current_search_bg,
                default.current_search_bg,
            ),
        ] {
            assert_eq!(
                got, inherited,
                "{label} should inherit, not follow an alias"
            );
            assert_ne!(got, fg, "{label} was flattened onto the foreground alias");
            assert_ne!(got, bg, "{label} was flattened onto the background alias");
        }

        // The specific collapse from the review: these must stay distinguishable.
        assert_ne!(c.current_row_bg, c.current_cell_bg);
    }

    // =========================================================================
    // ThemeSet
    // =========================================================================

    #[test]
    fn resolve_selects_the_configured_default() {
        let cfg = ThemeConfig {
            default: "nord".into(),
            custom: Vec::new(),
        };
        let (set, warnings) = ThemeSet::resolve(&cfg, None).unwrap();
        assert_eq!(set.current_name(), "Nord");
        assert!(warnings.is_empty());
    }

    #[test]
    fn resolve_warns_and_falls_back_when_default_is_unknown() {
        let cfg = ThemeConfig {
            default: "nope".into(),
            custom: Vec::new(),
        };
        let (set, warnings) = ThemeSet::resolve(&cfg, None).unwrap();
        assert_eq!(set.current_name(), "Default");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("nope"));
    }

    #[test]
    fn override_selects_theme_by_name() {
        let cfg = ThemeConfig {
            default: "Default".into(),
            custom: Vec::new(),
        };
        let (set, warnings) = ThemeSet::resolve(&cfg, Some("Dracula")).unwrap();
        assert_eq!(set.current_name(), "Dracula");
        assert!(warnings.is_empty());
    }

    #[test]
    fn override_unknown_theme_is_a_hard_error() {
        let cfg = ThemeConfig {
            default: "Default".into(),
            custom: Vec::new(),
        };
        let err = ThemeSet::resolve(&cfg, Some("nope")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown theme 'nope'"), "{msg}");
        assert!(
            msg.contains("Default"),
            "should list available themes: {msg}"
        );
    }

    #[test]
    fn override_beats_config_default() {
        let cfg = ThemeConfig {
            default: "Nord".into(),
            custom: Vec::new(),
        };
        let (set, _) = ThemeSet::resolve(&cfg, Some("Dracula")).unwrap();
        assert_eq!(set.current_name(), "Dracula");
    }

    #[test]
    fn cycle_visits_every_theme_and_wraps() {
        let cfg = ThemeConfig {
            default: "Default".into(),
            custom: vec![custom("mine")],
        };
        let (mut set, _) = ThemeSet::resolve(&cfg, None).unwrap();

        let mut seen = vec![set.current_name().to_string()];
        for _ in 0..BUILTIN_COUNT {
            set.cycle();
            seen.push(set.current_name().to_string());
        }

        // Built-ins first, then customs, then back to the start.
        assert_eq!(seen.first().unwrap(), "Default");
        assert_eq!(seen[BUILTIN_COUNT], "mine", "customs cycle after built-ins");
        set.cycle();
        assert_eq!(set.current_name(), "Default", "wraps around");
    }

    // =========================================================================
    // Truecolor warning
    // =========================================================================

    #[test]
    fn truecolor_warning_silent_for_builtins() {
        let cfg = ThemeConfig {
            default: "Nord".into(),
            custom: Vec::new(),
        };
        let (set, _) = ThemeSet::resolve(&cfg, None).unwrap();
        assert!(set.themes[set.current].colors.uses_rgb());
        assert!(truecolor_warning(&set, None).is_none());
    }

    #[test]
    fn truecolor_warning_fires_for_custom_with_rgb() {
        let cfg = ThemeConfig {
            default: "mine".into(),
            custom: vec![CustomTheme {
                inherits: Some("Nord".into()),
                string_fg: Some(Color::Rgb(1, 2, 3)),
                ..custom("mine")
            }],
        };
        let (set, _) = ThemeSet::resolve(&cfg, None).unwrap();
        let w = truecolor_warning(&set, None);
        assert!(w.is_some(), "should warn");
        assert!(w.as_ref().unwrap().contains("COLORTERM is not set"));
    }

    #[test]
    fn truecolor_warning_silent_when_colorterm_is_truecolor() {
        let cfg = ThemeConfig {
            default: "mine".into(),
            custom: vec![CustomTheme {
                inherits: Some("Nord".into()),
                ..custom("mine")
            }],
        };
        let (set, _) = ThemeSet::resolve(&cfg, None).unwrap();
        assert!(truecolor_warning(&set, Some("truecolor")).is_none());
        assert!(truecolor_warning(&set, Some("24bit")).is_none());
    }

    #[test]
    fn truecolor_warning_reports_actual_colorterm_value() {
        let cfg = ThemeConfig {
            default: "mine".into(),
            custom: vec![CustomTheme {
                inherits: Some("Nord".into()),
                ..custom("mine")
            }],
        };
        let (set, _) = ThemeSet::resolve(&cfg, None).unwrap();
        let w = truecolor_warning(&set, Some("256color")).unwrap();
        assert!(w.contains("256color"), "should report actual value: {w}");
    }

    // =========================================================================
    // uses_rgb
    // =========================================================================

    #[test]
    fn all_builtins_use_rgb() {
        for t in builtin_themes() {
            assert!(t.colors.uses_rgb(), "{} should use RGB", t.name);
        }
    }

    #[test]
    fn ansi_only_scheme_does_not_use_rgb() {
        let mut c = ColorScheme::default_theme();
        c.alternating_row_bg = None;
        assert!(!c.uses_rgb());
    }
}
