use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Entry {
    pub project: String,
    pub hours: f32,
    pub note: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Day {
    pub date: String,
    pub entries: Vec<Entry>,
}

impl Day {
    pub fn total_hours(&self) -> f32 {
        self.entries.iter().map(|e| e.hours).sum()
    }
}

#[derive(Clone, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(skip)]
    pub client_name: String,
}

#[derive(Clone, Deserialize)]
pub struct TimeEntry {
    // pub id: i32, // Unused
    pub date: String,
    pub description: String,
    #[serde(rename = "projectId")]
    #[serde(default)]
    pub project_id: i32,
    pub project: Option<ProjectRef>,
    #[serde(rename = "projectName")]
    #[serde(default)]
    pub project_name: String,
    pub minutes: i32,
}

#[derive(Clone, Deserialize)]
pub struct ProjectRef {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateEntryRequest {
    pub date: String,
    pub project_id: i32,
    pub description: String,
    pub minutes: i32,
    pub is_billable: bool,
    pub tag_ids: Vec<i32>,
}

#[derive(Clone)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

impl DateRange {
    pub fn label(&self) -> String {
        format!("{}..{}", self.start, self.end)
    }
}
