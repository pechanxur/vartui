use chrono::Local;
use ratatui::widgets::ListState;
use std::env;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crate::domain::config::AppConfig;
use crate::domain::models::*;
use crate::infrastructure::api_client::ApiClient;
use crate::infrastructure::config::{load_config, save_config};
use crate::utils::parsing::*;

const API_BASE: &str = "https://var.elaniin.com/api";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    AddingEntry,
    Configuring,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppFocus {
    Days,
    Entries,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FormField {
    Date,
    ProjectId,
    Description,
    Minutes,
    Billable,
}

pub struct EntryForm {
    pub date: String,
    pub description: String,
    pub minutes: String,
    pub is_billable: bool,
    pub focused: FormField,
    pub project_search: String,
    pub filtered_indices: Vec<usize>,
    pub list_state: ListState,
    pub selected_project: Option<Project>,
}

impl EntryForm {
    pub fn new(default_date: String) -> Self {
        Self {
            date: default_date,
            description: String::new(),
            minutes: String::new(),
            is_billable: true,
            focused: FormField::Date,
            project_search: String::new(),
            filtered_indices: Vec::new(),
            list_state: ListState::default(),
            selected_project: None,
        }
    }

    pub fn with_entry_data(
        date: String,
        project_name: String,
        description: String,
        minutes: i32,
        is_billable: bool,
    ) -> Self {
        let hours = minutes / 60;
        let mins = minutes % 60;
        Self {
            date,
            description,
            minutes: format!("{:02}:{:02}", hours, mins),
            is_billable,
            focused: FormField::Description, // Start at description for easy editing
            project_search: project_name,
            filtered_indices: Vec::new(),
            list_state: ListState::default(),
            selected_project: None,
        }
    }

    pub fn next_field(&mut self) {
        self.focused = match self.focused {
            FormField::Date => FormField::ProjectId,
            FormField::ProjectId => FormField::Description,
            FormField::Description => FormField::Minutes,
            FormField::Minutes => FormField::Billable,
            FormField::Billable => FormField::Date,
        };
    }

    pub fn prev_field(&mut self) {
        self.focused = match self.focused {
            FormField::Date => FormField::Billable,
            FormField::ProjectId => FormField::Date,
            FormField::Description => FormField::ProjectId,
            FormField::Minutes => FormField::Description,
            FormField::Billable => FormField::Minutes,
        };
    }
}

// Helper struct to return data from background thread
pub struct BackgroundResult {
    pub days: Vec<Day>,
    pub status: String,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ConfigField {
    Token,
    BaseUrl,
    DefaultRange,
}

pub struct ConfigForm {
    pub token: String,
    pub base_url: String,
    pub default_range: String,
    pub focused: ConfigField,
}

pub struct App {
    pub days: Vec<Day>,
    pub day_state: ListState,
    pub entry_state: ListState,
    pub focus: AppFocus,
    pub status: String,
    pub date_range: DateRange,
    pub input_mode: InputMode,
    pub input: String,
    pub rx: Option<Receiver<BackgroundResult>>,
    pub rx_projects: Option<Receiver<Result<Vec<Project>, String>>>,
    pub entry_form: Option<EntryForm>,
    pub projects: Vec<Project>,
    pub config: AppConfig,
    pub config_form: Option<ConfigForm>,
}

impl App {
    pub fn new() -> Self {
        let config = load_config();

        let mut date_range = initial_date_range();
        // Apply config overrides if present
        if let Some(range_str) = &config.default_date_range {
            if let Ok(r) = parse_date_range(range_str) {
                date_range = r;
            }
        }

        let days = build_empty_days(&date_range);
        let rx = spawn_load_projects(&config);
        let rx_load = spawn_load(date_range.clone(), &config);

        let mut app = Self {
            days,
            day_state: ListState::default(),
            entry_state: ListState::default(),
            focus: AppFocus::Days,
            status: "cargando...".to_string(),
            date_range,
            input_mode: InputMode::Normal,
            input: String::new(),
            rx: Some(rx_load),
            rx_projects: Some(rx),
            entry_form: None,
            projects: Vec::new(),
            config,
            config_form: None,
        };
        // Ensure valid selection on init
        if !app.days.is_empty() {
            app.day_state.select(Some(0));
        }
        app
    }

    pub fn set_days(&mut self, days: Vec<Day>) {
        self.days = days;
        if self.days.is_empty() {
            self.day_state.select(None);
        } else {
            let idx = self
                .day_state
                .selected()
                .unwrap_or(0)
                .min(self.days.len() - 1);
            self.day_state.select(Some(idx));
        }
    }

    pub fn selected_day(&self) -> Option<&Day> {
        self.day_state.selected().and_then(|idx| self.days.get(idx))
    }

    pub fn selected_index(&self) -> usize {
        self.day_state.selected().unwrap_or(0)
    }

    pub fn next_day(&mut self) {
        if self.days.is_empty() {
            return;
        }
        let next = match self.day_state.selected() {
            Some(idx) if idx + 1 < self.days.len() => idx + 1,
            _ => 0,
        };
        self.day_state.select(Some(next));
    }

    pub fn previous_day(&mut self) {
        if self.days.is_empty() {
            return;
        }
        let prev = match self.day_state.selected() {
            Some(0) | None => self.days.len() - 1,
            Some(idx) => idx - 1,
        };
        self.day_state.select(Some(prev));
    }

    // Entry navigation methods
    pub fn focus_entries(&mut self) {
        if let Some(day) = self.selected_day() {
            if !day.entries.is_empty() {
                self.focus = AppFocus::Entries;
                self.entry_state.select(Some(0));
            }
        }
    }

    pub fn focus_days(&mut self) {
        self.focus = AppFocus::Days;
        self.entry_state.select(None);
    }

    pub fn next_entry(&mut self) {
        if let Some(day) = self.selected_day() {
            if day.entries.is_empty() {
                return;
            }
            let next = match self.entry_state.selected() {
                Some(idx) if idx + 1 < day.entries.len() => idx + 1,
                _ => 0,
            };
            self.entry_state.select(Some(next));
        }
    }

    pub fn previous_entry(&mut self) {
        if let Some(day) = self.selected_day() {
            if day.entries.is_empty() {
                return;
            }
            let prev = match self.entry_state.selected() {
                Some(0) | None => day.entries.len() - 1,
                Some(idx) => idx - 1,
            };
            self.entry_state.select(Some(prev));
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected_day().and_then(|day| {
            self.entry_state
                .selected()
                .and_then(|idx| day.entries.get(idx))
        })
    }

    pub fn open_duplicate_entry(&mut self) {
        if self.focus != AppFocus::Entries {
            return;
        }

        // Get data from selected entry
        let (date, project_name, description, minutes) = if let Some(day) = self.selected_day() {
            if let Some(entry) = self
                .entry_state
                .selected()
                .and_then(|idx| day.entries.get(idx))
            {
                let mins = (entry.hours * 60.0) as i32;
                (
                    day.date.clone(),
                    entry.project.clone(),
                    entry.note.clone(),
                    mins,
                )
            } else {
                return;
            }
        } else {
            return;
        };

        // Create form with pre-filled data (default is_billable to true since we don't store it)
        self.entry_form = Some(EntryForm::with_entry_data(
            date,
            project_name,
            description,
            minutes,
            true,
        ));
        self.input_mode = InputMode::AddingEntry;
        self.update_project_filter();
    }

    pub fn refresh(&mut self) {
        self.status = "actualizando...".to_string();
        self.rx = Some(spawn_load(self.date_range.clone(), &self.config));
    }

    pub fn start_input(&mut self) {
        self.input_mode = InputMode::Editing;
        self.input = format!("{}..{}", self.date_range.start, self.date_range.end);
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input.clear();
    }

    pub fn submit_input(&mut self) {
        match parse_date_range(&self.input) {
            Ok(range) => {
                self.date_range = range;
                self.input_mode = InputMode::Normal;
                self.input.clear();
                self.set_days(build_empty_days(&self.date_range));
                self.refresh();
            }
            Err(error) => {
                self.status = format!("estado: {}", error);
            }
        }
    }

    pub fn input_push(&mut self, value: char) {
        if value.is_ascii() && self.input.len() < 64 {
            self.input.push(value);
        }
    }

    pub fn input_backspace(&mut self) {
        self.input.pop();
    }

    pub fn check_background_load(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.set_days(result.days);
                    self.status = result.status;
                    done = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(_) => {
                    done = true;
                }
            }
        }
        if done {
            self.rx = None;
        }

        let mut done_projects = false;
        if let Some(rx) = &self.rx_projects {
            match rx.try_recv() {
                Ok(Ok(projects)) => {
                    self.projects = projects;
                    self.status = format!("proyectos cargados: {}", self.projects.len());
                    done_projects = true;
                }
                Ok(Err(e)) => {
                    self.status = format!("error proyectos: {}", e);
                    done_projects = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(_) => {
                    done_projects = true;
                }
            }
        }
        if done_projects {
            self.rx_projects = None;
        }
    }

    pub fn open_add_entry(&mut self) {
        let default_date = if let Some(day) = self.selected_day() {
            day.date.clone()
        } else {
            Local::now().format("%Y-%m-%d").to_string()
        };
        self.entry_form = Some(EntryForm::new(default_date));
        self.input_mode = InputMode::AddingEntry;
        self.update_project_filter();
    }

    pub fn close_add_entry(&mut self) {
        self.entry_form = None;
        self.input_mode = InputMode::Normal;
    }

    pub fn form_next_field(&mut self) {
        if let Some(form) = &mut self.entry_form {
            form.next_field();
        }
    }

    pub fn form_prev_field(&mut self) {
        if let Some(form) = &mut self.entry_form {
            form.prev_field();
        }
    }

    pub fn form_input_push(&mut self, ch: char) {
        if let Some(form) = &mut self.entry_form {
            match form.focused {
                FormField::Date => form.date.push(ch),
                FormField::ProjectId => {
                    form.project_search.push(ch);
                    self.update_project_filter();
                }
                FormField::Description => form.description.push(ch),
                FormField::Minutes => form.minutes.push(ch),
                FormField::Billable => {
                    if ch == ' ' {
                        form.is_billable = !form.is_billable;
                    }
                }
            }
        }
    }

    pub fn form_input_backspace(&mut self) {
        if let Some(form) = &mut self.entry_form {
            match form.focused {
                FormField::Date => {
                    form.date.pop();
                }
                FormField::ProjectId => {
                    form.project_search.pop();
                    self.update_project_filter();
                }
                FormField::Description => {
                    form.description.pop();
                }
                FormField::Minutes => {
                    form.minutes.pop();
                }
                FormField::Billable => {}
            }
        }
    }

    pub fn update_project_filter(&mut self) {
        if let Some(form) = &mut self.entry_form {
            let query = form.project_search.to_lowercase();
            if query.is_empty() {
                form.filtered_indices = (0..self.projects.len()).take(20).collect();
            } else {
                form.filtered_indices = self
                    .projects
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.name.to_lowercase().contains(&query))
                    .map(|(i, _)| i)
                    .take(20)
                    .collect();
            }
            if !form.filtered_indices.is_empty() {
                form.list_state.select(Some(0));
            } else {
                form.list_state.select(None);
            }
        }
    }

    pub fn form_nav_up(&mut self) {
        if let Some(form) = &mut self.entry_form {
            if form.focused == FormField::ProjectId && !form.filtered_indices.is_empty() {
                let i = form.list_state.selected().unwrap_or(0);
                if i > 0 {
                    form.list_state.select(Some(i - 1));
                }
            }
        }
    }

    pub fn form_nav_down(&mut self) {
        if let Some(form) = &mut self.entry_form {
            if form.focused == FormField::ProjectId && !form.filtered_indices.is_empty() {
                let i = form.list_state.selected().unwrap_or(0);
                if i + 1 < form.filtered_indices.len() {
                    form.list_state.select(Some(i + 1));
                }
            }
        }
    }

    pub fn form_enter(&mut self) {
        if self.entry_form.is_none() {
            return;
        }

        let is_project_focused = self.entry_form.as_ref().unwrap().focused == FormField::ProjectId;
        if is_project_focused {
            let form = self.entry_form.as_mut().unwrap();
            if let Some(idx) = form.list_state.selected() {
                if let Some(&project_idx) = form.filtered_indices.get(idx) {
                    if let Some(project) = self.projects.get(project_idx) {
                        form.selected_project = Some(project.clone());
                        form.project_search = project.name.clone();
                        form.filtered_indices.clear();
                        form.next_field();
                        return;
                    }
                }
            }
        }

        let is_last = self.entry_form.as_ref().unwrap().focused == FormField::Billable;
        if is_last {
            self.submit_entry();
        } else {
            self.form_next_field();
        }
    }

    pub fn submit_entry(&mut self) {
        let (d, p_id, desc, m_str, is_billable) = if let Some(form) = &self.entry_form {
            let pid = if let Some(p) = &form.selected_project {
                p.id
            } else {
                form.project_search.parse().unwrap_or(0)
            };
            (
                form.date.clone(),
                pid,
                form.description.clone(),
                form.minutes.clone(),
                form.is_billable,
            )
        } else {
            return;
        };

        if d.is_empty() || p_id == 0 || desc.is_empty() || m_str.is_empty() {
            self.status = "error: campos vacios o proyecto invalido".to_string();
            return;
        }

        let minutes: i32 = if m_str.contains(':') {
            let parts: Vec<&str> = m_str.split(':').collect();
            if parts.len() == 2 {
                let h: i32 = parts[0].trim().parse().unwrap_or(0);
                let m: i32 = parts[1].trim().parse().unwrap_or(0);
                h * 60 + m
            } else {
                0
            }
        } else {
            m_str.parse().unwrap_or(0)
        };

        if minutes == 0 {
            self.status = "error: tiempo invalido (0 o formato incorrecto)".to_string();
            return;
        }

        self.status = "creando registro...".to_string();

        let token = env::var("VAR_TOKEN")
            .unwrap_or_default()
            .replace('"', "")
            .trim()
            .to_string();
        let mut base_url = env::var("VAR_BASE_URL")
            .ok()
            .map(|v| v.replace('"', "").trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| API_BASE.to_string());
        if base_url.ends_with('/') {
            base_url.pop();
        }

        let client = match ApiClient::new(base_url, token) {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("error cliente: {}", e);
                return;
            }
        };

        match client.create_time_entry(&d, p_id, &desc, minutes, is_billable) {
            Ok(_) => {
                self.close_add_entry();
                self.status = "registro creado!".to_string();
                self.refresh();
            }
            Err(e) => {
                self.status = format!("error crear: {}", e);
            }
        }
    }

    // Config Modal Methods
    pub fn open_config(&mut self) {
        self.config_form = Some(ConfigForm {
            token: self.get_effective_token(),
            base_url: self.get_effective_base_url(),
            default_range: self.config.default_date_range.clone().unwrap_or_default(),
            focused: ConfigField::Token,
        });
        self.input_mode = InputMode::Configuring;
        self.status = "Configurando...".to_string();
    }

    pub fn close_config(&mut self) {
        self.config_form = None;
        self.input_mode = InputMode::Normal;
        self.status = "Cancelado".to_string();
    }

    pub fn save_config_form(&mut self) {
        if let Some(form) = &self.config_form {
            let mut new_config = self.config.clone();
            new_config.var_token = form.token.trim().to_string();
            new_config.base_url = form.base_url.trim().to_string();

            let dr = form.default_range.trim();
            new_config.default_date_range = if dr.is_empty() {
                None
            } else {
                Some(dr.to_string())
            };

            match save_config(&new_config) {
                Ok(_) => {
                    self.config = new_config;

                    // Apply new date range if set
                    if let Some(range_str) = &self.config.default_date_range {
                        if let Ok(r) = parse_date_range(range_str) {
                            self.date_range = r;
                            self.set_days(build_empty_days(&self.date_range));
                        }
                    }

                    self.status = "Configuracion guardada!".to_string();
                    self.config_form = None;
                    self.input_mode = InputMode::Normal;
                    self.refresh();
                }
                Err(e) => {
                    self.status = format!("Error guardando: {}", e);
                }
            }
        }
    }

    pub fn config_next_field(&mut self) {
        if let Some(form) = &mut self.config_form {
            form.focused = match form.focused {
                ConfigField::Token => ConfigField::BaseUrl,
                ConfigField::BaseUrl => ConfigField::DefaultRange,
                ConfigField::DefaultRange => ConfigField::Token,
            };
        }
    }

    pub fn config_input(&mut self, ch: char) {
        if let Some(form) = &mut self.config_form {
            match form.focused {
                ConfigField::Token => form.token.push(ch),
                ConfigField::BaseUrl => form.base_url.push(ch),
                ConfigField::DefaultRange => form.default_range.push(ch),
            }
        }
    }

    pub fn config_backspace(&mut self) {
        if let Some(form) = &mut self.config_form {
            match form.focused {
                ConfigField::Token => {
                    form.token.pop();
                }
                ConfigField::BaseUrl => {
                    form.base_url.pop();
                }
                ConfigField::DefaultRange => {
                    form.default_range.pop();
                }
            }
        }
    }

    pub fn config_clear_field(&mut self) {
        if let Some(form) = &mut self.config_form {
            match form.focused {
                ConfigField::Token => form.token.clear(),
                ConfigField::BaseUrl => form.base_url.clear(),
                ConfigField::DefaultRange => form.default_range.clear(),
            }
        }
    }

    // Helpers to resolve Env vs Config
    pub fn get_effective_token(&self) -> String {
        if !self.config.var_token.is_empty() {
            self.config.var_token.clone()
        } else {
            env::var("VAR_TOKEN")
                .unwrap_or_default()
                .replace('"', "")
                .trim()
                .to_string()
        }
    }

    pub fn get_effective_base_url(&self) -> String {
        if !self.config.base_url.is_empty() && self.config.base_url != "https://var.elaniin.com/api"
        {
            self.config.base_url.clone()
        } else {
            env::var("VAR_BASE_URL")
                .ok()
                .map(|v| v.replace('"', "").trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| API_BASE.to_string())
        }
    }
}

// Background Task functions
pub fn spawn_load(range: DateRange, config: &AppConfig) -> Receiver<BackgroundResult> {
    let (tx, rx) = mpsc::channel();
    let token = if !config.var_token.is_empty() {
        config.var_token.clone()
    } else {
        env::var("VAR_TOKEN")
            .unwrap_or_default()
            .replace('"', "")
            .trim()
            .to_string()
    };
    let mut base_url =
        if !config.base_url.is_empty() && config.base_url != "https://var.elaniin.com/api" {
            config.base_url.clone()
        } else {
            env::var("VAR_BASE_URL")
                .ok()
                .map(|v| v.replace('"', "").trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| API_BASE.to_string())
        };
    if base_url.ends_with('/') {
        base_url.pop();
    }

    thread::spawn(move || {
        let result = match ApiClient::new(base_url, token) {
            Ok(client) => match client.fetch_days(&range.start, &range.end) {
                Ok(fetch_res) => {
                    let count = fetch_res.days.len();
                    BackgroundResult {
                        days: fetch_res.days,
                        status: format!("actualizado: {} dias", count),
                    }
                }
                Err(e) => BackgroundResult {
                    days: Vec::new(),
                    status: e,
                },
            },
            Err(e) => BackgroundResult {
                days: Vec::new(),
                status: e,
            },
        };
        let _ = tx.send(result);
    });
    rx
}

pub fn spawn_load_projects(config: &AppConfig) -> Receiver<Result<Vec<Project>, String>> {
    let (tx, rx) = mpsc::channel();
    let token = if !config.var_token.is_empty() {
        config.var_token.clone()
    } else {
        env::var("VAR_TOKEN")
            .unwrap_or_default()
            .replace('"', "")
            .trim()
            .to_string()
    };
    let mut base_url =
        if !config.base_url.is_empty() && config.base_url != "https://var.elaniin.com/api" {
            config.base_url.clone()
        } else {
            env::var("VAR_BASE_URL")
                .ok()
                .map(|v| v.replace('"', "").trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| API_BASE.to_string())
        };
    if base_url.ends_with('/') {
        base_url.pop();
    }

    thread::spawn(move || {
        let result = match ApiClient::new(base_url, token) {
            Ok(client) => client.fetch_projects_list().map_err(|e| e),
            Err(e) => Err(e),
        };
        let _ = tx.send(result);
    });
    rx
}
