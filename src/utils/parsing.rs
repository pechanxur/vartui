use crate::domain::models::*;
use std::collections::HashMap;

pub fn build_days(
    time_entries: Vec<TimeEntry>,
    projects: Vec<Project>,
    start_date: &str,
    end_date: &str,
) -> Vec<Day> {
    let project_map: HashMap<i32, String> = projects
        .into_iter()
        .map(|project| (project.id, project.name))
        .collect();

    let mut grouped: HashMap<String, Vec<Entry>> = HashMap::new();
    for entry in time_entries {
        let project = if entry.project_id != 0 {
            project_map
                .get(&entry.project_id)
                .cloned()
                .unwrap_or_else(|| format!("Proyecto {}", entry.project_id))
        } else if let Some(project) = &entry.project {
            if !project.name.trim().is_empty() {
                project.name.trim().to_string()
            } else if project.id != 0 {
                project_map
                    .get(&project.id)
                    .cloned()
                    .unwrap_or_else(|| format!("Proyecto {}", project.id))
            } else {
                "Proyecto".to_string()
            }
        } else if !entry.project_name.trim().is_empty() {
            entry.project_name.trim().to_string()
        } else {
            "Proyecto".to_string()
        };
        let date = if entry.date.trim().is_empty() {
            "sin-fecha".to_string()
        } else {
            normalize_date(&entry.date)
        };
        let note = if entry.description.trim().is_empty() {
            "sin descripcion".to_string()
        } else {
            entry.description
        };
        let hours = entry.minutes as f32 / 60.0;
        grouped.entry(date).or_default().push(Entry {
            project,
            hours,
            note,
        });
    }

    if let (Some(start), Some(end)) = (parse_date(start_date), parse_date(end_date)) {
        return build_range_days(grouped, start, end);
    }

    let mut days: Vec<Day> = grouped
        .into_iter()
        .map(|(date, entries)| Day { date, entries })
        .collect();
    days.sort_by(|a, b| b.date.cmp(&a.date));
    days
}

fn normalize_date(date_str: &str) -> String {
    if let Some(date) = parse_date(date_str) {
        return date.format("%Y-%m-%d").to_string();
    }
    date_str.to_string()
}

pub fn parse_date(date_str: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%Y/%m/%d"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%d-%m-%Y"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%d/%m/%Y"))
        .ok()
}

fn build_range_days(
    mut grouped: HashMap<String, Vec<Entry>>,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Vec<Day> {
    let mut days = Vec::new();
    let mut current = start;
    while current <= end {
        let date_key = current.format("%Y-%m-%d").to_string();
        let entries = grouped.remove(&date_key).unwrap_or_default();
        days.push(Day {
            date: date_key,
            entries,
        });
        current += chrono::Duration::days(1);
    }
    days.sort_by(|a, b| b.date.cmp(&a.date));
    days
}

pub fn build_empty_days(range: &crate::domain::models::DateRange) -> Vec<Day> {
    if let (Some(start), Some(end)) = (parse_date(&range.start), parse_date(&range.end)) {
        return build_range_days(HashMap::new(), start, end);
    }
    Vec::new()
}

pub fn initial_date_range() -> crate::domain::models::DateRange {
    let now = chrono::Local::now().date_naive();
    use chrono::Datelike;
    // Default to start of current month
    let start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
    let end = now;
    crate::domain::models::DateRange {
        start: start.format("%Y-%m-%d").to_string(),
        end: end.format("%Y-%m-%d").to_string(),
    }
}

pub fn parse_date_range(input: &str) -> Result<crate::domain::models::DateRange, String> {
    use chrono::Datelike;
    let now = chrono::Local::now().date_naive();
    
    // Handle special keywords
    match input.trim().to_uppercase().as_str() {
        "AUTO" | "AUTO-MONTH" | "MONTH" => {
            let start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
            return Ok(crate::domain::models::DateRange {
                start: start.format("%Y-%m-%d").to_string(),
                end: now.format("%Y-%m-%d").to_string(),
            });
        }
        "AUTO-WEEK" | "WEEK" => {
            // Start of week (Monday)
            let weekday = now.weekday().num_days_from_monday();
            let start = now - chrono::Duration::days(weekday as i64);
            return Ok(crate::domain::models::DateRange {
                start: start.format("%Y-%m-%d").to_string(),
                end: now.format("%Y-%m-%d").to_string(),
            });
        }
        _ => {}
    }
    
    // Standard format: YYYY-MM-DD..YYYY-MM-DD
    let parts: Vec<&str> = input.split("..").collect();
    if parts.len() != 2 {
        return Err("Formato incorrecto. Use YYYY-MM-DD..YYYY-MM-DD, AUTO, AUTO-WEEK, o AUTO-MONTH".to_string());
    }
    let start_str = parts[0].trim();
    let end_str = parts[1].trim();
    
    // basic validation
    if parse_date(start_str).is_none() {
        return Err(format!("Fecha inicio invalida: {}", start_str));
    }
    if parse_date(end_str).is_none() {
        return Err(format!("Fecha fin invalida: {}", end_str));
    }
    
    Ok(crate::domain::models::DateRange {
        start: start_str.to_string(),
        end: end_str.to_string(),
    })
}
