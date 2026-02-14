use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub var_token: String,
    pub base_url: String,
    // Default to None means "use default logic" (e.g. current month)
    // "MONTH" -> Current Month
    // "WEEK" -> Current week
    // "YYYY-MM-DD..YYYY-MM-DD" -> Specific range
    pub default_date_range: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            var_token: String::new(),
            base_url: "https://var.elaniin.com/api".to_string(),
            default_date_range: None,
            theme: default_theme(),
        }
    }
}

fn default_theme() -> String {
    "tokyo-night".to_string()
}
