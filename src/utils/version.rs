const DEVELOPMENT_VERSION: &str = "development";

pub fn build_version() -> &'static str {
    match option_env!("VARTUI_RELEASE_VERSION") {
        Some(version) if !version.trim().is_empty() => version,
        _ => DEVELOPMENT_VERSION,
    }
}
