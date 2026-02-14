use std::str::FromStr;
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
use std::process::Command;

use ratatui_themes::{ThemeName, ThemePalette};

use crate::domain::config::AppConfig;

pub const THEME_CATALOG: &[&str] = &[
    "dracula",
    "one-dark-pro",
    "nord",
    "catppuccin-mocha",
    "catppuccin-latte",
    "gruvbox-dark",
    "gruvbox-light",
    "tokyo-night",
    "solarized-dark",
    "solarized-light",
    "monokai-pro",
    "rose-pine",
    "kanagawa",
    "everforest",
    "cyberpunk",
];

pub fn palette_from_config(config: &AppConfig) -> ThemePalette {
    resolve_theme_name(&config.theme).palette()
}

pub fn palette_with_override(config: &AppConfig, override_theme: Option<&str>) -> ThemePalette {
    let key = override_theme.unwrap_or(&config.theme);
    resolve_theme_name(key).palette()
}

pub fn resolve_theme_slug_with_override(
    config: &AppConfig,
    override_theme: Option<&str>,
) -> &'static str {
    let key = override_theme.unwrap_or(&config.theme);
    resolve_theme_name(key).slug()
}

pub fn resolve_theme_name(raw: &str) -> ThemeName {
    let value = normalize_theme_key(raw);
    if value == "auto" {
        return detect_system_theme_name();
    }
    ThemeName::from_str(&value).unwrap_or(ThemeName::TokyoNight)
}

fn normalize_theme_key(raw: &str) -> String {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return "tokyo-night".to_string();
    }

    match value.as_str() {
        "default" | "system" => "auto".to_string(),
        "tokyo" => "tokyo-night".to_string(),
        "catppuccin" | "catppuccin-dark" | "mocha" => "catppuccin-mocha".to_string(),
        "catppuccin-light" | "latte" => "catppuccin-latte".to_string(),
        "gruvbox" => "gruvbox-dark".to_string(),
        "solarized" => "solarized-dark".to_string(),
        other => other.to_string(),
    }
}

fn detect_system_theme_name() -> ThemeName {
    static DETECTED_THEME: OnceLock<ThemeName> = OnceLock::new();
    *DETECTED_THEME.get_or_init(detect_system_theme_uncached)
}

fn detect_system_theme_uncached() -> ThemeName {
    if let Some(is_dark) = parse_theme_hint(std::env::var("VARTUI_SYSTEM_THEME").ok().as_deref()) {
        return if is_dark {
            ThemeName::TokyoNight
        } else {
            ThemeName::CatppuccinLatte
        };
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(is_dark) = detect_macos_dark_mode() {
            return if is_dark {
                ThemeName::TokyoNight
            } else {
                ThemeName::CatppuccinLatte
            };
        }
    }

    ThemeName::TokyoNight
}

#[cfg(target_os = "macos")]
fn detect_macos_dark_mode() -> Option<bool> {
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(true)
    } else {
        Some(false)
    }
}

fn parse_theme_hint(raw: Option<&str>) -> Option<bool> {
    let raw = raw?.trim().to_ascii_lowercase();
    match raw.as_str() {
        "dark" | "1" | "true" => Some(true),
        "light" | "0" | "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_aliases() {
        assert_eq!(resolve_theme_name("catppuccin").slug(), "catppuccin-mocha");
        assert_eq!(resolve_theme_name("tokyo").slug(), "tokyo-night");
    }

    #[test]
    fn falls_back_to_default_theme() {
        assert_eq!(resolve_theme_name("unknown-theme").slug(), "tokyo-night");
    }
}
