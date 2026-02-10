use std::env;

use serde::Serialize;

use crate::domain::config::AppConfig;
use crate::domain::models::{DateRange, Day};
use crate::infrastructure::api_client::ApiClient;
use crate::infrastructure::config::load_config;
use crate::utils::parsing::parse_date_range;

const DEFAULT_API_BASE: &str = "https://var.elaniin.com/api";

const API_HELP: &str = "  api projects [--pretty]\n  api days [--range <AUTO|AUTO-WEEK|AUTO-MONTH|YYYY-MM-DD..YYYY-MM-DD>] [--pretty]\n  api entries [--range <AUTO|AUTO-WEEK|AUTO-MONTH|YYYY-MM-DD..YYYY-MM-DD>] [--pretty]\n  api create-entry --date <YYYY-MM-DD> --project-id <ID> --description <TEXTO> --minutes <MINUTOS> [--billable <true|false>] [--pretty]";

const PROJECTS_HELP: &str = "Uso:\n  api projects [--pretty]";

const DAYS_HELP: &str =
    "Uso:\n  api days [--range <AUTO|AUTO-WEEK|AUTO-MONTH|YYYY-MM-DD..YYYY-MM-DD>] [--pretty]";

const ENTRIES_HELP: &str =
    "Uso:\n  api entries [--range <AUTO|AUTO-WEEK|AUTO-MONTH|YYYY-MM-DD..YYYY-MM-DD>] [--pretty]";

const CREATE_ENTRY_HELP: &str = "Uso:\n  api create-entry --date <YYYY-MM-DD> --project-id <ID> --description <TEXTO> --minutes <MINUTOS> [--billable <true|false>] [--pretty]";

#[derive(Serialize)]
struct ProjectOutput {
    id: i32,
    name: String,
    client_name: String,
}

#[derive(Serialize)]
struct DaysOutput {
    range: String,
    days: Vec<Day>,
}

#[derive(Serialize)]
struct EntryOutput {
    date: String,
    project: String,
    hours: f32,
    note: String,
}

#[derive(Serialize)]
struct EntriesOutput {
    range: String,
    entries: Vec<EntryOutput>,
}

#[derive(Serialize)]
struct CreateEntryOutput {
    ok: bool,
    date: String,
    project_id: i32,
    minutes: i32,
    is_billable: bool,
}

pub fn run_api(args: &[String]) -> Result<(), String> {
    if args.is_empty() || is_help(args[0].as_str()) {
        println!("Subcomandos:\n{API_HELP}");
        return Ok(());
    }

    match args[0].as_str() {
        "projects" => cmd_projects(&args[1..]),
        "days" => cmd_days(&args[1..]),
        "entries" => cmd_entries(&args[1..]),
        "create-entry" => cmd_create_entry(&args[1..]),
        other => Err(format!("Comando API desconocido: {other}\n\n{API_HELP}")),
    }
}

pub fn api_help() -> &'static str {
    API_HELP
}

fn cmd_projects(args: &[String]) -> Result<(), String> {
    if contains_help(args) {
        println!("{PROJECTS_HELP}");
        return Ok(());
    }

    let pretty = parse_pretty_flag(args)?;
    let (_, client) = build_client_and_config()?;
    let projects = client.fetch_projects_list()?;
    let output: Vec<ProjectOutput> = projects
        .into_iter()
        .map(|project| ProjectOutput {
            id: project.id,
            name: project.name,
            client_name: project.client_name,
        })
        .collect();

    print_json(&output, pretty)
}

fn cmd_days(args: &[String]) -> Result<(), String> {
    if contains_help(args) {
        println!("{DAYS_HELP}");
        return Ok(());
    }

    let options = parse_list_options(args, DAYS_HELP)?;
    let (config, client) = build_client_and_config()?;
    let range = resolve_range(options.range, &config)?;
    let fetch = client.fetch_days(&range.start, &range.end)?;
    let output = DaysOutput {
        range: range.label(),
        days: fetch.days,
    };

    print_json(&output, options.pretty)
}

fn cmd_entries(args: &[String]) -> Result<(), String> {
    if contains_help(args) {
        println!("{ENTRIES_HELP}");
        return Ok(());
    }

    let options = parse_list_options(args, ENTRIES_HELP)?;
    let (config, client) = build_client_and_config()?;
    let range = resolve_range(options.range, &config)?;
    let fetch = client.fetch_days(&range.start, &range.end)?;

    let mut entries = Vec::new();
    for day in fetch.days {
        for entry in day.entries {
            entries.push(EntryOutput {
                date: day.date.clone(),
                project: entry.project,
                hours: entry.hours,
                note: entry.note,
            });
        }
    }

    let output = EntriesOutput {
        range: range.label(),
        entries,
    };

    print_json(&output, options.pretty)
}

fn cmd_create_entry(args: &[String]) -> Result<(), String> {
    if contains_help(args) {
        println!("{CREATE_ENTRY_HELP}");
        return Ok(());
    }

    let mut date: Option<String> = None;
    let mut project_id: Option<i32> = None;
    let mut description: Option<String> = None;
    let mut minutes: Option<i32> = None;
    let mut is_billable = true;
    let mut pretty = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--date" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --date")?;
                date = Some(value.clone());
            }
            "--project-id" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --project-id")?;
                let parsed = value
                    .parse::<i32>()
                    .map_err(|_| format!("project-id invalido: {value}"))?;
                if parsed <= 0 {
                    return Err("project-id debe ser mayor a 0".to_string());
                }
                project_id = Some(parsed);
            }
            "--description" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --description")?;
                description = Some(value.clone());
            }
            "--minutes" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --minutes")?;
                let parsed = value
                    .parse::<i32>()
                    .map_err(|_| format!("minutes invalido: {value}"))?;
                if parsed <= 0 {
                    return Err("minutes debe ser mayor a 0".to_string());
                }
                minutes = Some(parsed);
            }
            "--billable" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --billable")?;
                is_billable = parse_bool(value)
                    .ok_or_else(|| format!("Valor invalido para --billable: {value}"))?;
            }
            "--pretty" => pretty = true,
            unknown => {
                return Err(format!(
                    "Flag desconocida para create-entry: {unknown}\n\n{CREATE_ENTRY_HELP}"
                ));
            }
        }
        i += 1;
    }

    let date = date.ok_or_else(|| "Falta --date".to_string())?;
    let project_id = project_id.ok_or_else(|| "Falta --project-id".to_string())?;
    let description = description.ok_or_else(|| "Falta --description".to_string())?;
    let minutes = minutes.ok_or_else(|| "Falta --minutes".to_string())?;

    let (_, client) = build_client_and_config()?;
    client.create_time_entry(&date, project_id, &description, minutes, is_billable)?;

    let output = CreateEntryOutput {
        ok: true,
        date,
        project_id,
        minutes,
        is_billable,
    };

    print_json(&output, pretty)
}

fn build_client_and_config() -> Result<(AppConfig, ApiClient), String> {
    let config = load_config();

    let token = if !config.var_token.is_empty() {
        config.var_token.trim().to_string()
    } else {
        env::var("VAR_TOKEN")
            .unwrap_or_default()
            .replace('"', "")
            .trim()
            .to_string()
    };

    if token.is_empty() {
        return Err(
            "No hay token configurado. Define VAR_TOKEN o guarda var_token en la configuracion."
                .to_string(),
        );
    }

    let mut base_url = if !config.base_url.is_empty() && config.base_url != DEFAULT_API_BASE {
        config.base_url.trim().to_string()
    } else {
        env::var("VAR_BASE_URL")
            .ok()
            .map(|value| value.replace('"', "").trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string())
    };

    if base_url.ends_with('/') {
        base_url.pop();
    }

    let client = ApiClient::new(base_url, token)?;
    Ok((config, client))
}

fn resolve_range(input: Option<String>, config: &AppConfig) -> Result<DateRange, String> {
    let raw = input
        .or_else(|| config.default_date_range.clone())
        .unwrap_or_else(|| "AUTO".to_string());

    parse_date_range(&raw).map_err(|error| format!("Rango invalido ({raw}): {error}"))
}

fn parse_pretty_flag(args: &[String]) -> Result<bool, String> {
    let mut pretty = false;
    for arg in args {
        match arg.as_str() {
            "--pretty" => pretty = true,
            unknown => {
                return Err(format!("Flag desconocida: {unknown}\n\n{PROJECTS_HELP}"));
            }
        }
    }
    Ok(pretty)
}

struct ListOptions {
    range: Option<String>,
    pretty: bool,
}

fn parse_list_options(args: &[String], help_text: &str) -> Result<ListOptions, String> {
    let mut range: Option<String> = None;
    let mut pretty = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--range" => {
                i += 1;
                let value = args.get(i).ok_or("Falta valor para --range")?;
                range = Some(value.clone());
            }
            "--pretty" => pretty = true,
            unknown => {
                return Err(format!("Flag desconocida: {unknown}\n\n{help_text}"));
            }
        }
        i += 1;
    }

    Ok(ListOptions { range, pretty })
}

fn print_json<T: Serialize>(value: &T, pretty: bool) -> Result<(), String> {
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|error| error.to_string())?;
    println!("{json}");
    Ok(())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" => Some(true),
        "0" | "false" | "no" | "n" => Some(false),
        _ => None,
    }
}

fn is_help(value: &str) -> bool {
    matches!(value, "-h" | "--help" | "help")
}

fn contains_help(args: &[String]) -> bool {
    args.iter().any(|value| is_help(value.as_str()))
}
