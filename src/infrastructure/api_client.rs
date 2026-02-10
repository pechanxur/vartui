use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

use crate::domain::models::*;
use crate::log;

pub struct ApiClient {
    pub base_url: String,
    pub token: String,
    pub client: Client,
}

#[derive(Clone, Copy)]
pub enum QueryStyle {
    Snake,
    Camel,
}

pub struct FetchResult {
    pub days: Vec<Day>,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client,
        })
    }

    pub fn create_time_entry(
        &self,
        date: &str,
        project_id: i32,
        description: &str,
        minutes: i32,
        is_billable: bool,
    ) -> Result<(), String> {
        let url = format!("{}/time-entries", self.base_url);
        log!("POST Request URL: {}", url);
        log!("POST Token Len: {}", self.token.len());
        log!(
            "POST Body: date={}, pid={}, desc={}, mins={}, billable={}",
            date,
            project_id,
            description,
            minutes,
            is_billable
        );

        let body = CreateEntryRequest {
            date: date.to_string(),
            project_id,
            description: description.to_string(),
            minutes,
            is_billable,
            tag_ids: Vec::new(),
        };

        let body_json = serde_json::to_string(&body).map_err(|e| e.to_string())?;
        log!("POST Body JSON: {}", body_json);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .map_err(|e| format!("Reqwest Error (builder/send): {}", e))?;

        let status = response.status();
        log!("POST Response Status: {}", status);
        if status.is_success() || status.as_u16() == 201 {
            Ok(())
        } else {
            let text = response.text().unwrap_or_default();
            log!("POST Error Body: {}", text);
            Err(format!("{} {}", status.as_u16(), text))
        }
    }

    pub fn fetch_days(&self, start_date: &str, end_date: &str) -> Result<FetchResult, String> {
        log!("Fetching days: {} to {}", start_date, end_date);
        let projects = self.fetch_projects_list()?;
        let (time_entries, _) = self.get_time_entries(start_date, end_date)?;
        let entries_count = time_entries.len();
        log!("Fetched {} entries", entries_count);

        let days = crate::utils::parsing::build_days(time_entries, projects, start_date, end_date);
        Ok(FetchResult { days })
    }

    pub fn fetch_projects_list(&self) -> Result<Vec<Project>, String> {
        let url = format!("{}/projects", self.base_url);
        log!("Fetching projects from: {}", url);
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| e.to_string())?;

        let status = response.status();
        log!("Projects response status: {}", status);
        let text = response.text().map_err(|e| e.to_string())?;

        // Parse as Map<String, Vec<Project>>
        let map: HashMap<String, Vec<Project>> = serde_json::from_str(&text).map_err(|e| {
            log!("Error parsing projects JSON: {}", e);
            format!(
                "Error parsing projects: {} | Response start: {:.50}",
                e, text
            )
        })?;

        let mut all_projects = Vec::new();
        for (client, mut projects) in map {
            for p in &mut projects {
                p.client_name = client.clone();
            }
            all_projects.extend(projects);
        }
        // Sort by Client then Name
        all_projects.sort_by(|a, b| a.client_name.cmp(&b.client_name).then(a.name.cmp(&b.name)));

        Ok(all_projects)
    }

    fn get_time_entries(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<(Vec<TimeEntry>, QueryStyle), String> {
        let primary = self.get_time_entries_with_params(start_date, end_date, QueryStyle::Snake)?;
        if should_try_alt_dates(start_date, end_date, primary.len()) {
            if let Ok(alt) =
                self.get_time_entries_with_params(start_date, end_date, QueryStyle::Camel)
            {
                if alt.len() > primary.len() {
                    return Ok((alt, QueryStyle::Camel));
                }
            }
        }

        Ok((primary, QueryStyle::Snake))
    }

    fn get_time_entries_with_params(
        &self,
        start_date: &str,
        end_date: &str,
        style: QueryStyle,
    ) -> Result<Vec<TimeEntry>, String> {
        let url = format!("{}/time-entries", self.base_url);
        let request = self.client.get(url).bearer_auth(&self.token);
        let request = match style {
            QueryStyle::Snake => {
                request.query(&[("start_date", start_date), ("end_date", end_date)])
            }
            QueryStyle::Camel => request.query(&[("startDate", start_date), ("endDate", end_date)]),
        };
        let response = request.send().map_err(|error| error.to_string())?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "{} {}",
                status.as_u16(),
                body.lines().next().unwrap_or("")
            ));
        }

        let body = response.text().unwrap_or_default();
        // Try parsing as HashMap<String, Vec<TimeEntry>>
        if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<TimeEntry>>>(&body) {
            let mut all_entries = Vec::new();
            for entries in map.into_values() {
                all_entries.extend(entries);
            }
            return Ok(all_entries);
        }

        // Fallback to old list parsing
        parse_list_from_body(
            &body,
            &["data", "time_entries", "timeEntries", "entries", "items"],
        )
    }
}

fn should_try_alt_dates(_start: &str, _end: &str, count: usize) -> bool {
    if count > 0 {
        return false;
    }
    // Simple heuristic: if range is > 1 day and we got 0 entries, maybe format issue?
    // But keeping it simple as per original code
    true
}

fn parse_list_from_body<T: DeserializeOwned>(body: &str, keys: &[&str]) -> Result<Vec<T>, String> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        let snippet = body.lines().next().unwrap_or("");
        if snippet.is_empty() {
            format!("json invalido ({})", error)
        } else {
            format!("json invalido ({}) {}", error, snippet)
        }
    })?;

    if let Some(list) = extract_list(&value, keys) {
        return serde_json::from_value(list).map_err(|error| error.to_string());
    }

    Err(format!("json sin lista (keys: {})", keys.join(", ")))
}

fn extract_list(value: &Value, keys: &[&str]) -> Option<Value> {
    match value {
        Value::Array(_) if is_object_array(value) => Some(value.clone()),
        Value::Object(map) => {
            for key in keys {
                if let Some(list) = map.get(*key) {
                    if is_object_array(list) {
                        return Some(list.clone());
                    }
                }
            }
            for entry in map.values() {
                if let Some(found) = extract_list(entry, keys) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn is_object_array(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().all(|item| item.is_object()),
        _ => false,
    }
}
