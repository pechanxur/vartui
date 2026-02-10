use crate::domain::config::AppConfig;
use crate::log;
use confy;

const APP_NAME: &str = "vartui";

pub fn load_config() -> AppConfig {
    match confy::load(APP_NAME, "config") {
        Ok(cfg) => {
            log!("Config loaded successfully");
            cfg
        }
        Err(e) => {
            log!("Error loading config: {}. Using default.", e);
            AppConfig::default()
        }
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    confy::store(APP_NAME, "config", cfg).map_err(|e| e.to_string())
}
