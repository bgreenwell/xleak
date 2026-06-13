use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub ui: UiConfig,
    pub keybindings: KeybindingsConfig,
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Default theme to use on startup
    pub default: String,
    /// Custom theme definitions
    #[serde(default, skip_serializing)]
    pub custom: Vec<CustomTheme>,
}

/// A user-defined custom theme from `[[theme.custom]]` in config.toml
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomTheme {
    pub name: String,
    pub inherits: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub foreground: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub background: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub string_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub number_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub bool_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub datetime_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub error_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub empty_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub header_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub header_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_cell_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_cell_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_row_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_col_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub alternating_row_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub search_match_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub search_match_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_search_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub current_search_bg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub border_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub status_bar_fg: Option<Color>,
    #[serde(default, deserialize_with = "deserialize_opt_color")]
    pub status_bar_bg: Option<Color>,
}

/// 3-digit hex shorthand (#fff) is not supported.
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }
    match s.to_lowercase().replace([' ', '_', '-'], "").as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

fn deserialize_opt_color<'de, D>(deserializer: D) -> std::result::Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_color(&s)
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid color: '{s}'"))),
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Default maximum rows to display in non-interactive mode
    pub max_rows: usize,
    /// Default maximum column width
    pub column_width: usize,
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    /// Keybinding profile: "default", "vim", or "custom"
    pub profile: String,
    /// Custom keybindings (overrides profile)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            default: "Default".to_string(),
            custom: Vec::new(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            max_rows: 50,
            column_width: 30,
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            profile: "default".to_string(),
            custom: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from XDG config directory or custom path
    pub fn load(custom_path: Option<PathBuf>) -> Result<Self> {
        let config_path = if let Some(path) = custom_path {
            path
        } else {
            Self::default_config_path()?
        };

        if !config_path.exists() {
            // No config file, return defaults
            return Ok(Self::default());
        }

        let config_str = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(config)
    }

    /// Get the default config file path
    /// Checks XDG location first (~/.config/xleak/config.toml), then OS-specific location
    pub fn default_config_path() -> Result<PathBuf> {
        // First, try XDG-compliant location (~/.config/xleak/config.toml)
        if let Some(home) = dirs::home_dir() {
            let xdg_path = home.join(".config").join("xleak").join("config.toml");
            if xdg_path.exists() {
                return Ok(xdg_path);
            }
        }

        // Fall back to OS-specific config directory
        // macOS: ~/Library/Application Support/xleak/config.toml
        // Linux: ~/.config/xleak/config.toml (same as XDG)
        // Windows: %APPDATA%\xleak\config.toml
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?
            .join("xleak");

        Ok(config_dir.join("config.toml"))
    }

    /// Create an example config file at the default location
    #[allow(dead_code)]
    pub fn create_example() -> Result<PathBuf> {
        let config_path = Self::default_config_path()?;
        let config_dir = config_path
            .parent()
            .context("Failed to get config directory")?;

        // Create config directory if it doesn't exist
        fs::create_dir_all(config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        // Generate example config
        let example = Self::example_toml();
        fs::write(&config_path, example).with_context(|| {
            format!("Failed to write example config: {}", config_path.display())
        })?;

        Ok(config_path)
    }

    /// Generate example TOML config
    fn example_toml() -> String {
        r#"# xleak configuration file
# Location: $XDG_CONFIG_HOME/xleak/config.toml (usually ~/.config/xleak/config.toml)

[theme]
# Default theme to use on startup
# Options: "Default", "Dracula", "Solarized Dark", "Solarized Light", "GitHub Dark", "Nord"
default = "Default"

[ui]
# Default maximum rows to display in non-interactive mode (0 = all)
max_rows = 50
# Default maximum column width in characters
column_width = 30

[keybindings]
# Keybinding profile: "default" or "vim"
profile = "default"

# Custom keybindings (optional - overrides profile)
# Uncomment and modify to customize individual keys
# [keybindings.custom]
# quit = "q"
# help = "?"
# theme_toggle = "t"
# search = "/"
# next_match = "n"
# prev_match = "N"
# copy_cell = "c"
# copy_row = "C"
# jump = "Ctrl+g"
# show_cell_detail = "Enter"

# VIM-style navigation (when profile = "vim")
# up = "k"
# down = "j"
# left = "h"
# right = "l"
# page_up = "Ctrl+u"
# page_down = "Ctrl+d"
# jump_to_top = "g"
# jump_to_bottom = "G"
# jump_to_row_start = "0"
# jump_to_row_end = "$"
"#
        .to_string()
    }

    /// Get keybinding for an action based on profile and custom overrides
    pub fn get_keybinding(&self, action: &str) -> Option<(KeyCode, KeyModifiers)> {
        // Check custom bindings first
        if let Some(key_str) = self.keybindings.custom.get(action) {
            return parse_key_string(key_str);
        }

        // Fall back to profile defaults
        match self.keybindings.profile.as_str() {
            "vim" => get_vim_keybinding(action),
            _ => get_default_keybinding(action),
        }
    }
}

/// Parse a key string like "q", "Ctrl+g", "Enter" into KeyCode and KeyModifiers
fn parse_key_string(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = KeyModifiers::empty();
    let key_part = parts.last()?;

    // Parse modifiers
    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            _ => return None,
        }
    }

    let code = match *key_part {
        k if k.eq_ignore_ascii_case("enter") => KeyCode::Enter,
        k if k.eq_ignore_ascii_case("esc") => KeyCode::Esc,
        k if k.eq_ignore_ascii_case("escape") => KeyCode::Esc,
        k if k.eq_ignore_ascii_case("tab") => KeyCode::Tab,
        k if k.eq_ignore_ascii_case("backtab") => KeyCode::BackTab,
        k if k.eq_ignore_ascii_case("backspace") => KeyCode::Backspace,
        k if k.eq_ignore_ascii_case("delete") => KeyCode::Delete,
        k if k.eq_ignore_ascii_case("del") => KeyCode::Delete,
        k if k.eq_ignore_ascii_case("insert") => KeyCode::Insert,
        k if k.eq_ignore_ascii_case("ins") => KeyCode::Insert,
        k if k.eq_ignore_ascii_case("home") => KeyCode::Home,
        k if k.eq_ignore_ascii_case("end") => KeyCode::End,
        k if k.eq_ignore_ascii_case("pageup") => KeyCode::PageUp,
        k if k.eq_ignore_ascii_case("pgup") => KeyCode::PageUp,
        k if k.eq_ignore_ascii_case("pagedown") => KeyCode::PageDown,
        k if k.eq_ignore_ascii_case("pgdn") => KeyCode::PageDown,
        k if k.eq_ignore_ascii_case("up") => KeyCode::Up,
        k if k.eq_ignore_ascii_case("down") => KeyCode::Down,
        k if k.eq_ignore_ascii_case("left") => KeyCode::Left,
        k if k.eq_ignore_ascii_case("right") => KeyCode::Right,
        s if s.len() == 1 => KeyCode::Char(s.chars().next()?),
        _ => return None,
    };
    Some((code, modifiers))
}

/// Get default keybinding for an action
fn get_default_keybinding(action: &str) -> Option<(KeyCode, KeyModifiers)> {
    let binding = match action {
        "quit" => ("q", KeyModifiers::empty()),
        "help" => ("?", KeyModifiers::SHIFT),
        "theme_toggle" => ("t", KeyModifiers::empty()),
        "search" => ("/", KeyModifiers::empty()),
        "next_match" => ("n", KeyModifiers::empty()),
        "prev_match" => ("N", KeyModifiers::SHIFT),
        "copy_cell" => ("c", KeyModifiers::empty()),
        "copy_row" => ("C", KeyModifiers::SHIFT),
        "jump" => ("g", KeyModifiers::CONTROL),
        "show_cell_detail" => ("Enter", KeyModifiers::empty()),
        "next_sheet" => ("Tab", KeyModifiers::empty()),
        "prev_sheet" => ("Tab", KeyModifiers::SHIFT),
        "up" => ("Up", KeyModifiers::empty()),
        "down" => ("Down", KeyModifiers::empty()),
        "left" => ("Left", KeyModifiers::empty()),
        "right" => ("Right", KeyModifiers::empty()),
        "page_up" => ("PageUp", KeyModifiers::empty()),
        "page_down" => ("PageDown", KeyModifiers::empty()),
        "jump_to_top" => ("Home", KeyModifiers::CONTROL),
        "jump_to_bottom" => ("End", KeyModifiers::CONTROL),
        "jump_to_row_start" => ("Home", KeyModifiers::empty()),
        "jump_to_row_end" => ("End", KeyModifiers::empty()),
        _ => return None,
    };

    parse_key_string(binding.0).map(|(code, _)| (code, binding.1))
}

/// Get VIM-style keybinding for an action
fn get_vim_keybinding(action: &str) -> Option<(KeyCode, KeyModifiers)> {
    let binding = match action {
        // VIM navigation
        "up" => ("k", KeyModifiers::empty()),
        "down" => ("j", KeyModifiers::empty()),
        "left" => ("h", KeyModifiers::empty()),
        "right" => ("l", KeyModifiers::empty()),
        "page_up" => ("u", KeyModifiers::CONTROL),
        "page_down" => ("d", KeyModifiers::CONTROL),
        "jump_to_top" => ("g", KeyModifiers::empty()),
        "jump_to_bottom" => ("G", KeyModifiers::SHIFT),
        "jump_to_row_start" => ("0", KeyModifiers::empty()),
        "jump_to_row_end" => ("$", KeyModifiers::SHIFT),
        // VIM-style actions
        "quit" => ("q", KeyModifiers::empty()),
        "copy_cell" => ("y", KeyModifiers::empty()),
        "copy_row" => ("Y", KeyModifiers::SHIFT),
        // Keep standard bindings for non-VIM actions
        _ => return get_default_keybinding(action),
    };

    parse_key_string(binding.0).map(|(code, _)| (code, binding.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_string() {
        assert_eq!(
            parse_key_string("q"),
            Some((KeyCode::Char('q'), KeyModifiers::empty()))
        );
        assert_eq!(
            parse_key_string("Ctrl+g"),
            Some((KeyCode::Char('g'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key_string("Enter"),
            Some((KeyCode::Enter, KeyModifiers::empty()))
        );
        assert_eq!(
            parse_key_string("Shift+Tab"),
            Some((KeyCode::Tab, KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.theme.default, "Default");
        assert_eq!(config.ui.max_rows, 50);
        assert_eq!(config.keybindings.profile, "default");
    }

    #[test]
    fn test_vim_keybindings() {
        assert_eq!(
            get_vim_keybinding("up"),
            Some((KeyCode::Char('k'), KeyModifiers::empty()))
        );
        assert_eq!(
            get_vim_keybinding("down"),
            Some((KeyCode::Char('j'), KeyModifiers::empty()))
        );
    }

    // =========================================================================
    // TOML Parsing Tests (Cross-Platform Line Endings)
    // =========================================================================

    #[test]
    fn test_toml_parsing_unix_line_endings() {
        let config_str = "[theme]\ndefault = \"Dracula\"\n\n[ui]\nmax_rows = 100\n\n[keybindings]\nprofile = \"vim\"";
        let config: Config = toml::from_str(config_str).expect("Failed to parse TOML");
        assert_eq!(config.theme.default, "Dracula");
        assert_eq!(config.ui.max_rows, 100);
        assert_eq!(config.keybindings.profile, "vim");
    }

    #[test]
    fn test_toml_parsing_windows_line_endings() {
        let config_str = "[theme]\r\ndefault = \"Nord\"\r\n\r\n[ui]\r\nmax_rows = 200\r\n\r\n[keybindings]\r\nprofile = \"default\"";
        let config: Config = toml::from_str(config_str).expect("Failed to parse TOML");
        assert_eq!(config.theme.default, "Nord");
        assert_eq!(config.ui.max_rows, 200);
        assert_eq!(config.keybindings.profile, "default");
    }

    #[test]
    fn test_toml_parsing_mixed_line_endings() {
        let config_str = "[theme]\r\ndefault = \"GitHub Dark\"\n\n[ui]\r\nmax_rows = 75\n[keybindings]\r\nprofile = \"vim\"";
        let config: Config = toml::from_str(config_str).expect("Failed to parse TOML");
        assert_eq!(config.theme.default, "GitHub Dark");
        assert_eq!(config.ui.max_rows, 75);
        assert_eq!(config.keybindings.profile, "vim");
    }

    // =========================================================================
    // Theme Name Tests (Case Sensitivity)
    // =========================================================================

    #[test]
    fn test_theme_name_case_insensitive() {
        // Theme config parsing stores the string as-is
        // TuiState::parse_theme_name handles case-insensitive matching
        let config_str = "[theme]\ndefault = \"dracula\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "dracula");

        let config_str = "[theme]\ndefault = \"DRACULA\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "DRACULA");

        let config_str = "[theme]\ndefault = \"Dracula\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "Dracula");
    }

    #[test]
    fn test_theme_name_with_spaces() {
        let config_str = "[theme]\ndefault = \"Solarized Dark\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "Solarized Dark");

        let config_str = "[theme]\ndefault = \"GitHub Dark\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "GitHub Dark");
    }

    #[test]
    fn test_invalid_theme_stored_as_is() {
        // Config stores the theme name as-is; TuiState handles fallback to Default
        let config_str = "[theme]\ndefault = \"NonexistentTheme\"";
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "NonexistentTheme");
    }

    // =========================================================================
    // Keybinding Override Tests
    // =========================================================================

    #[test]
    fn test_custom_keybindings_override_profile() {
        let config_str = r#"
[keybindings]
profile = "default"

[keybindings.custom]
quit = "x"
search = "?"
"#;
        let config: Config = toml::from_str(config_str).unwrap();

        // Custom binding should override
        assert_eq!(
            config.get_keybinding("quit"),
            Some((KeyCode::Char('x'), KeyModifiers::empty()))
        );
        assert_eq!(
            config.get_keybinding("search"),
            Some((KeyCode::Char('?'), KeyModifiers::empty()))
        );

        // Non-overridden should use profile default
        assert_eq!(
            config.get_keybinding("help"),
            Some((KeyCode::Char('?'), KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn test_vim_profile_with_custom_overrides() {
        let config_str = r#"
[keybindings]
profile = "vim"

[keybindings.custom]
quit = "x"
page_up = "Ctrl+b"
"#;
        let config: Config = toml::from_str(config_str).unwrap();

        // Custom overrides
        assert_eq!(
            config.get_keybinding("quit"),
            Some((KeyCode::Char('x'), KeyModifiers::empty()))
        );
        assert_eq!(
            config.get_keybinding("page_up"),
            Some((KeyCode::Char('b'), KeyModifiers::CONTROL))
        );

        // VIM profile bindings (not overridden)
        assert_eq!(
            config.get_keybinding("up"),
            Some((KeyCode::Char('k'), KeyModifiers::empty()))
        );
        assert_eq!(
            config.get_keybinding("down"),
            Some((KeyCode::Char('j'), KeyModifiers::empty()))
        );
    }

    #[test]
    fn test_get_keybinding_returns_none_for_unknown_action() {
        let config = Config::default();
        assert_eq!(config.get_keybinding("nonexistent_action"), None);
        assert_eq!(config.get_keybinding(""), None);
        assert_eq!(config.get_keybinding("random_string_12345"), None);
    }

    // =========================================================================
    // Key Parsing Edge Cases
    // =========================================================================

    #[test]
    fn test_parse_key_multiple_modifiers() {
        // Note: crossterm doesn't support more than 2 modifiers simultaneously,
        // but we should parse them correctly
        assert_eq!(
            parse_key_string("Ctrl+Shift+Tab"),
            Some((KeyCode::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT))
        );

        assert_eq!(
            parse_key_string("Ctrl+Alt+g"),
            Some((
                KeyCode::Char('g'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ))
        );
    }

    #[test]
    fn test_parse_key_case_insensitive_modifiers() {
        assert_eq!(
            parse_key_string("ctrl+g"),
            Some((KeyCode::Char('g'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key_string("CTRL+g"),
            Some((KeyCode::Char('g'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key_string("Ctrl+g"),
            Some((KeyCode::Char('g'), KeyModifiers::CONTROL))
        );

        assert_eq!(
            parse_key_string("shift+tab"),
            Some((KeyCode::Tab, KeyModifiers::SHIFT))
        );
        assert_eq!(
            parse_key_string("SHIFT+TAB"),
            Some((KeyCode::Tab, KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn test_parse_key_invalid_strings() {
        assert_eq!(parse_key_string(""), None);
        assert_eq!(parse_key_string("InvalidKey"), None);
        assert_eq!(parse_key_string("Ctrl+"), None);
        assert_eq!(parse_key_string("+g"), None);
        assert_eq!(parse_key_string("Ctrl+InvalidKey"), None);
        assert_eq!(parse_key_string("Unknown+g"), None);
    }

    // =========================================================================
    // Profile Behavior Tests
    // =========================================================================

    #[test]
    fn test_vim_profile_falls_back_to_default() {
        let config_str = "[keybindings]\nprofile = \"vim\"";
        let config: Config = toml::from_str(config_str).unwrap();

        // VIM-specific bindings
        assert_eq!(
            config.get_keybinding("up"),
            Some((KeyCode::Char('k'), KeyModifiers::empty()))
        );

        // Non-VIM actions should fall back to default profile
        assert_eq!(
            config.get_keybinding("help"),
            Some((KeyCode::Char('?'), KeyModifiers::SHIFT))
        );
        assert_eq!(
            config.get_keybinding("theme_toggle"),
            Some((KeyCode::Char('t'), KeyModifiers::empty()))
        );
        assert_eq!(
            config.get_keybinding("search"),
            Some((KeyCode::Char('/'), KeyModifiers::empty()))
        );
    }

    // =========================================================================
    // Color Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_color_hex() {
        assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_color("#0000ff"), Some(Color::Rgb(0, 0, 255)));
        assert_eq!(parse_color("#1a1b26"), Some(Color::Rgb(26, 27, 38)));
    }

    #[test]
    fn test_parse_color_named() {
        assert_eq!(parse_color("red"), Some(Color::Red));
        assert_eq!(parse_color("cyan"), Some(Color::Cyan));
        assert_eq!(parse_color("White"), Some(Color::White));
        assert_eq!(parse_color("dark_gray"), Some(Color::DarkGray));
        assert_eq!(parse_color("DarkGray"), Some(Color::DarkGray));
        assert_eq!(parse_color("light-yellow"), Some(Color::LightYellow));
    }

    #[test]
    fn test_parse_color_invalid() {
        assert_eq!(parse_color("#fff"), None);
        assert_eq!(parse_color("#gggggg"), None);
        assert_eq!(parse_color("notacolor"), None);
        assert_eq!(parse_color(""), None);
    }

    // =========================================================================
    // Custom Theme Config Tests
    // =========================================================================

    #[test]
    fn test_custom_theme_parsing() {
        let config_str = r##"
[theme]
default = "tokyonight"

[[theme.custom]]
name = "tokyonight"
inherits = "Dracula"
foreground = "#c0caf5"
background = "#1a1b26"
header_fg = "#7aa2f7"
"##;
        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.theme.default, "tokyonight");
        assert_eq!(config.theme.custom.len(), 1);
        assert_eq!(config.theme.custom[0].name, "tokyonight");
        assert_eq!(config.theme.custom[0].inherits.as_deref(), Some("Dracula"));
        assert_eq!(
            config.theme.custom[0].foreground,
            Some(Color::Rgb(192, 202, 245))
        );
        assert_eq!(
            config.theme.custom[0].header_fg,
            Some(Color::Rgb(122, 162, 247))
        );
    }

    #[test]
    fn test_invalid_color_in_theme() {
        let config_str = r#"
[theme]
default = "Default"

[[theme.custom]]
name = "bad"
foreground = "notacolor"
"#;
        let result: std::result::Result<Config, _> = toml::from_str(config_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_field_rejected() {
        let config_str = r#"
[theme]
default = "Default"

[[theme.custom]]
name = "typo"
forground = "red"
"#;
        let result: std::result::Result<Config, _> = toml::from_str(config_str);
        assert!(result.is_err());
    }
}
