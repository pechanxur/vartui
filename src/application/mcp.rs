use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use toon::{Delimiter, EncodeOptions};

use std::time::Duration;

use crate::application::app::{App, AppFocus, ConfigField, FormField, InputMode};
use crate::application::input::handle_key;
use crate::utils::parsing::parse_date_range;
use crate::utils::version::build_version;

const MCP_HELP: &str = "Uso:\n  mcp\n\nInicia un servidor MCP por stdio para automatizar el TUI con respuestas compactas en TOON.";

type ArgsMap = Map<String, Value>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SnapshotView {
    None,
    Tiny,
    Normal,
    Full,
}

impl SnapshotView {
    fn level(self) -> u8 {
        match self {
            SnapshotView::None => 0,
            SnapshotView::Tiny => 1,
            SnapshotView::Normal => 2,
            SnapshotView::Full => 3,
        }
    }

    fn at_least(self, other: SnapshotView) -> bool {
        self.level() >= other.level()
    }
}

struct ResponseOptions {
    include_structured: bool,
    view: SnapshotView,
    max_days: usize,
    max_entries: usize,
}

pub fn mcp_help() -> &'static str {
    "  mcp"
}

pub fn run_mcp(args: &[String]) -> Result<(), String> {
    if args.iter().any(|value| is_help(value.as_str())) {
        println!("{MCP_HELP}");
        return Ok(());
    }

    if !args.is_empty() {
        return Err(format!(
            "Flag desconocida para mcp: {}\n\n{MCP_HELP}",
            args[0]
        ));
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut state = ServerState::default();

    loop {
        let payload = match read_framed_message(&mut reader) {
            Ok(Some(body)) => body,
            Ok(None) => break,
            Err(error) => return Err(format!("Error leyendo mensaje MCP: {error}")),
        };

        if payload.trim().is_empty() {
            continue;
        }

        let request: RpcRequest = match serde_json::from_str(&payload) {
            Ok(value) => value,
            Err(error) => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("JSON invalido: {error}")
                    }
                });
                write_framed_message(&mut writer, &response)
                    .map_err(|err| format!("Error enviando respuesta MCP: {err}"))?;
                continue;
            }
        };

        let outcome = handle_rpc_request(request, &mut state);
        if let Some(response) = outcome.response {
            write_framed_message(&mut writer, &response)
                .map_err(|error| format!("Error enviando respuesta MCP: {error}"))?;
        }

        if outcome.exit {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
struct ServerState {
    next_session_id: u64,
    sessions: HashMap<String, App>,
}

impl ServerState {
    fn create_session(&mut self) -> String {
        self.next_session_id += 1;
        let session_id = format!("session-{}", self.next_session_id);
        let app = App::new_headless();
        self.sessions.insert(session_id.clone(), app);
        session_id
    }

    fn get_session_mut(&mut self, session_id: &str) -> Result<&mut App, String> {
        self.sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Sesion no encontrada: {session_id}"))
    }

    fn close_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }
}

struct RpcOutcome {
    response: Option<Value>,
    exit: bool,
}

#[derive(Deserialize)]
struct RpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

fn handle_rpc_request(request: RpcRequest, state: &mut ServerState) -> RpcOutcome {
    let method = request.method.as_str();
    let id = request.id.clone();

    match method {
        "initialize" => {
            let response = id.map(|rpc_id| {
                rpc_result(
                    rpc_id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "vartui-mcp",
                            "version": build_version()
                        },
                        "instructions": "Servidor MCP headless para el TUI. Usa `vartui.session.action` para menos tokens y `view=tiny|none` para respuestas minimas."
                    }),
                )
            });
            RpcOutcome {
                response,
                exit: false,
            }
        }
        "notifications/initialized" => RpcOutcome {
            response: None,
            exit: false,
        },
        "ping" => RpcOutcome {
            response: id.map(|rpc_id| rpc_result(rpc_id, json!({}))),
            exit: false,
        },
        "tools/list" => {
            let result = json!({
                "tools": [
                    {
                        "name": "vartui.session.create",
                        "description": "Crea sesion TUI aislada. Salida TOON compacta (default: view=tiny).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "view": {"type": "string", "enum": ["none", "tiny", "normal", "full"]},
                                "vw": {"type": "string", "enum": ["n", "t", "f", "0"]},
                                "max_days": {"type": "integer", "minimum": 1, "maximum": 120},
                                "md": {"type": "integer", "minimum": 1, "maximum": 120},
                                "max_entries_per_day": {"type": "integer", "minimum": 1, "maximum": 300},
                                "me": {"type": "integer", "minimum": 1, "maximum": 300},
                                "structured": {"type": "boolean"},
                                "stc": {"type": "boolean"}
                            }
                        }
                    },
                    {
                        "name": "vartui.session.snapshot",
                        "description": "Obtiene estado de sesion. Para menor costo usa view=tiny o view=none.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["session_id"],
                            "properties": {
                                "session_id": {"type": "string"},
                                "sid": {"type": "string"},
                                "view": {"type": "string", "enum": ["none", "tiny", "normal", "full"]},
                                "vw": {"type": "string", "enum": ["n", "t", "f", "0"]},
                                "max_days": {"type": "integer", "minimum": 1, "maximum": 120},
                                "md": {"type": "integer", "minimum": 1, "maximum": 120},
                                "max_entries_per_day": {"type": "integer", "minimum": 1, "maximum": 300},
                                "me": {"type": "integer", "minimum": 1, "maximum": 300},
                                "structured": {"type": "boolean"},
                                "stc": {"type": "boolean"}
                            }
                        }
                    },
                    {
                        "name": "vartui.session.key",
                        "description": "Paridad 1:1 con teclado del TUI. Recomendado solo cuando necesitas emulacion exacta.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["session_id", "key"],
                            "properties": {
                                "session_id": {"type": "string"},
                                "sid": {"type": "string"},
                                "key": {"type": "string"},
                                "k": {"type": "string"},
                                "text": {"type": "string"},
                                "t": {"type": "string"},
                                "view": {"type": "string", "enum": ["none", "tiny", "normal", "full"]},
                                "vw": {"type": "string", "enum": ["n", "t", "f", "0"]},
                                "max_days": {"type": "integer", "minimum": 1, "maximum": 120},
                                "md": {"type": "integer", "minimum": 1, "maximum": 120},
                                "max_entries_per_day": {"type": "integer", "minimum": 1, "maximum": 300},
                                "me": {"type": "integer", "minimum": 1, "maximum": 300},
                                "structured": {"type": "boolean"},
                                "stc": {"type": "boolean"}
                            }
                        }
                    },
                    {
                        "name": "vartui.session.action",
                        "description": "Acciones semanticas y batch para menor consumo de tokens. Soporta aliases cortos (a,f,v,k,t,i,sid,vw).",
                        "inputSchema": {
                            "type": "object",
                            "required": ["session_id"],
                            "properties": {
                                "session_id": {"type": "string"},
                                "sid": {"type": "string"},
                                "action": {"type": "string"},
                                "a": {"type": "string"},
                                "actions": {
                                    "type": "array",
                                    "items": {"type": "object"}
                                },
                                "field": {"type": "string"},
                                "f": {"type": "string"},
                                "value": {},
                                "v": {},
                                "key": {"type": "string"},
                                "k": {"type": "string"},
                                "text": {"type": "string"},
                                "t": {"type": "string"},
                                "index": {"type": "integer", "minimum": 0},
                                "i": {"type": "integer", "minimum": 0},
                                "view": {"type": "string", "enum": ["none", "tiny", "normal", "full"]},
                                "vw": {"type": "string", "enum": ["n", "t", "f", "0"]},
                                "max_days": {"type": "integer", "minimum": 1, "maximum": 120},
                                "md": {"type": "integer", "minimum": 1, "maximum": 120},
                                "max_entries_per_day": {"type": "integer", "minimum": 1, "maximum": 300},
                                "me": {"type": "integer", "minimum": 1, "maximum": 300},
                                "structured": {"type": "boolean"},
                                "stc": {"type": "boolean"}
                            }
                        }
                    },
                    {
                        "name": "vartui.session.close",
                        "description": "Cierra una sesion TUI y libera memoria.",
                        "inputSchema": {
                            "type": "object",
                            "required": ["session_id"],
                            "properties": {
                                "session_id": {"type": "string"},
                                "sid": {"type": "string"},
                                "structured": {"type": "boolean"},
                                "stc": {"type": "boolean"}
                            }
                        }
                    }
                ]
            });

            RpcOutcome {
                response: id.map(|rpc_id| rpc_result(rpc_id, result)),
                exit: false,
            }
        }
        "tools/call" => {
            let response = id.map(|rpc_id| {
                let result = match handle_tool_call(&request.params, state) {
                    Ok(ok) => ok,
                    Err(message) => tool_error_result(&message),
                };
                rpc_result(rpc_id, result)
            });
            RpcOutcome {
                response,
                exit: false,
            }
        }
        "shutdown" => RpcOutcome {
            response: id.map(|rpc_id| rpc_result(rpc_id, json!({}))),
            exit: false,
        },
        "exit" => RpcOutcome {
            response: None,
            exit: true,
        },
        other => RpcOutcome {
            response: id
                .map(|rpc_id| rpc_error(rpc_id, -32601, &format!("Metodo no soportado: {other}"))),
            exit: false,
        },
    }
}

fn handle_tool_call(params: &Value, state: &mut ServerState) -> Result<Value, String> {
    let payload = params
        .as_object()
        .ok_or_else(|| "tools/call requiere params tipo objeto".to_string())?;
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "tools/call requiere campo name".to_string())?;
    let arguments = payload
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let args = arguments
        .as_object()
        .ok_or_else(|| "tools/call.arguments debe ser objeto".to_string())?;

    match name {
        "vartui.session.create" => tool_session_create(args, state),
        "vartui.session.snapshot" => tool_session_snapshot(args, state),
        "vartui.session.key" => tool_session_key(args, state),
        "vartui.session.action" => tool_session_action(args, state),
        "vartui.session.close" => tool_session_close(args, state),
        other => Err(format!(
            "Tool no soportada: {other}. Usa tools/list para ver opciones."
        )),
    }
}

fn tool_session_create(args: &ArgsMap, state: &mut ServerState) -> Result<Value, String> {
    let options = parse_response_options(args, SnapshotView::Tiny)?;
    let session_id = state.create_session();
    let app = state.get_session_mut(&session_id)?;
    app.wait_background_load(Duration::from_secs(10));

    let snapshot = build_snapshot(&session_id, app, &options);
    let content = json!({
        "e": "sc",
        "sid": session_id,
        "s": snapshot
    });
    Ok(build_tool_result(content, options.include_structured))
}

fn tool_session_snapshot(args: &ArgsMap, state: &mut ServerState) -> Result<Value, String> {
    let options = parse_response_options(args, SnapshotView::Normal)?;
    let session_id = parse_session_id(args)?;
    let app = state.get_session_mut(&session_id)?;
    app.check_background_load();

    let snapshot = build_snapshot(&session_id, app, &options);
    let content = json!({
        "e": "ss",
        "sid": session_id,
        "s": snapshot
    });
    Ok(build_tool_result(content, options.include_structured))
}

fn tool_session_key(args: &ArgsMap, state: &mut ServerState) -> Result<Value, String> {
    let options = parse_response_options(args, SnapshotView::Tiny)?;
    let session_id = parse_session_id(args)?;
    let key = parse_required_string_alias(args, &["key", "k"])?;
    let text = arg(args, &["text", "t"]).and_then(Value::as_str);
    let sequence = parse_key_sequence(&key, text)?;

    let mut exit_requested = false;
    {
        let app = state.get_session_mut(&session_id)?;
        for (code, modifiers) in &sequence {
            if handle_key(app, *code, *modifiers) {
                exit_requested = true;
                break;
            }
            app.check_background_load();
        }
    }

    let snapshot = if exit_requested {
        None
    } else {
        state
            .sessions
            .get(&session_id)
            .and_then(|app| build_snapshot(&session_id, app, &options))
    };

    if exit_requested {
        state.close_session(&session_id);
    }

    let content = json!({
        "e": "sk",
        "sid": session_id,
        "k": key,
        "n": sequence.len(),
        "x": exit_requested,
        "s": snapshot
    });
    Ok(build_tool_result(content, options.include_structured))
}

fn tool_session_action(args: &ArgsMap, state: &mut ServerState) -> Result<Value, String> {
    let options = parse_response_options(args, SnapshotView::Tiny)?;
    let session_id = parse_session_id(args)?;
    let steps = parse_action_steps(args)?;

    let mut applied = 0usize;
    let mut exit_requested = false;
    let mut last_action = String::new();
    let mut actions_applied: Vec<String> = Vec::new();

    {
        let app = state.get_session_mut(&session_id)?;
        for (action, step_args) in &steps {
            let normalized = normalize_action(action);
            let step_exit = apply_action(app, normalized, step_args)?;
            applied += 1;
            last_action = normalized.to_string();
            if options.view.at_least(SnapshotView::Normal) {
                actions_applied.push(normalized.to_string());
            }
            if step_exit {
                exit_requested = true;
                break;
            }
        }
    }

    let snapshot = if exit_requested {
        None
    } else {
        state
            .sessions
            .get(&session_id)
            .and_then(|app| build_snapshot(&session_id, app, &options))
    };

    if exit_requested {
        state.close_session(&session_id);
    }

    let mut content = json!({
        "e": "sa",
        "sid": session_id,
        "n": applied,
        "la": last_action,
        "x": exit_requested,
        "s": snapshot
    });

    if options.view.at_least(SnapshotView::Normal)
        && let Some(map) = content.as_object_mut()
    {
        map.insert("as".to_string(), json!(actions_applied));
    }

    Ok(build_tool_result(content, options.include_structured))
}

fn tool_session_close(args: &ArgsMap, state: &mut ServerState) -> Result<Value, String> {
    let include_structured = parse_bool_alias(args, &["structured", "stc"], false)?;
    let session_id = parse_session_id(args)?;
    let removed = state.close_session(&session_id);
    let content = json!({
        "e": "sx",
        "sid": session_id,
        "c": removed
    });
    Ok(build_tool_result(content, include_structured))
}

fn parse_action_steps(args: &ArgsMap) -> Result<Vec<(String, ArgsMap)>, String> {
    if let Some(raw_actions) = args.get("actions") {
        let list = raw_actions
            .as_array()
            .ok_or_else(|| "actions debe ser un arreglo".to_string())?;
        if list.is_empty() {
            return Err("actions no puede estar vacio".to_string());
        }

        let mut steps = Vec::with_capacity(list.len());
        for item in list {
            let map = item
                .as_object()
                .ok_or_else(|| "cada item en actions debe ser objeto".to_string())?
                .clone();
            let action = parse_required_string_alias(&map, &["action", "a"])?;
            steps.push((action, map));
        }
        return Ok(steps);
    }

    let action = parse_required_string_alias(args, &["action", "a"])?;
    Ok(vec![(action, args.clone())])
}

fn apply_action(app: &mut App, action: &str, args: &ArgsMap) -> Result<bool, String> {
    match action {
        "noop" => {}
        "refresh" => app.refresh(),
        "focus_days" => app.focus_days(),
        "focus_entries" => app.focus_entries(),
        "next_day" => app.next_day(),
        "previous_day" => app.previous_day(),
        "next_entry" => app.next_entry(),
        "previous_entry" => app.previous_entry(),
        "open_duplicate_entry" => app.open_duplicate_entry(),
        "open_add_entry" => app.open_add_entry(),
        "close_add_entry" => app.close_add_entry(),
        "submit_entry" => app.submit_entry(),
        "entry_next_field" => app.form_next_field(),
        "entry_prev_field" => app.form_prev_field(),
        "entry_enter" => app.form_enter(),
        "entry_nav_up" => app.form_nav_up(),
        "entry_nav_down" => app.form_nav_down(),
        "entry_backspace" => app.form_input_backspace(),
        "toggle_billable" => toggle_billable(app)?,
        "set_entry_field" => set_entry_field(app, args)?,
        "select_project" => select_project(app, args)?,
        "open_config" => app.open_config(),
        "close_config" => app.close_config(),
        "save_config" => app.save_config_form(),
        "config_next_field" => app.config_next_field(),
        "config_backspace" => app.config_backspace(),
        "config_reset_defaults" => app.config_reset_defaults(),
        "config_clear_field" => clear_config_field(app, args)?,
        "set_config_field" => set_config_field(app, args)?,
        "open_range_editor" => app.start_input(),
        "submit_range" => app.submit_input(),
        "cancel_range_editor" => app.cancel_input(),
        "set_range" => {
            let value = parse_required_string_alias(args, &["value", "v", "range", "r"])?;
            parse_date_range(&value)
                .map_err(|error| format!("Rango invalido ({value}): {error}"))?;
            app.start_input();
            app.input = value;
            app.submit_input();
        }
        "send_key" => {
            let key = parse_required_string_alias(args, &["key", "k"])?;
            let text = arg(args, &["text", "t"]).and_then(Value::as_str);
            return execute_key_sequence(app, &key, text);
        }
        "type_text" => {
            let text = parse_required_string_alias(args, &["text", "t"])?;
            return execute_key_sequence(app, "text", Some(text.as_str()));
        }
        other => {
            return Err(format!("Accion no soportada: {other}"));
        }
    }

    app.check_background_load();
    Ok(false)
}

fn execute_key_sequence(app: &mut App, key: &str, text: Option<&str>) -> Result<bool, String> {
    let sequence = parse_key_sequence(key, text)?;
    let mut exit_requested = false;

    for (code, modifiers) in &sequence {
        if handle_key(app, *code, *modifiers) {
            exit_requested = true;
            break;
        }
        app.check_background_load();
    }

    Ok(exit_requested)
}

fn normalize_action(action: &str) -> &str {
    match action {
        "n" => "next_day",
        "p" => "previous_day",
        "nd" => "next_day",
        "pd" => "previous_day",
        "ne" => "next_entry",
        "pe" => "previous_entry",
        "fd" => "focus_days",
        "fe" => "focus_entries",
        "rf" => "refresh",
        "oa" => "open_add_entry",
        "ca" => "close_add_entry",
        "se" => "submit_entry",
        "sf" => "set_entry_field",
        "sp" => "select_project",
        "tb" => "toggle_billable",
        "oc" => "open_config",
        "cc" => "close_config",
        "sv" => "save_config",
        "scf" => "set_config_field",
        "clf" => "config_clear_field",
        "sr" => "set_range",
        "sk" => "send_key",
        "tt" => "type_text",
        "dup" => "open_duplicate_entry",
        _ => action,
    }
}

fn toggle_billable(app: &mut App) -> Result<(), String> {
    if app.entry_form.is_none() {
        app.open_add_entry();
    }

    let form = app
        .entry_form
        .as_mut()
        .ok_or_else(|| "No se pudo abrir formulario de entrada".to_string())?;
    form.is_billable = !form.is_billable;
    Ok(())
}

fn set_entry_field(app: &mut App, args: &ArgsMap) -> Result<(), String> {
    if app.entry_form.is_none() {
        app.open_add_entry();
    }

    let field = parse_required_string_alias(args, &["field", "f"])?;
    match field.as_str() {
        "date" | "d" => {
            let value = parse_required_string_alias(args, &["value", "v"])?;
            let form = app
                .entry_form
                .as_mut()
                .ok_or_else(|| "No hay formulario de entrada".to_string())?;
            form.date = value;
        }
        "project" | "project_search" | "p" => {
            let value = parse_required_string_alias(args, &["value", "v"])?;
            if let Some(form) = app.entry_form.as_mut() {
                form.project_search = value;
                form.selected_project = None;
            }
            app.update_project_filter();
        }
        "project_id" | "pid" => {
            let id = parse_required_i32_alias(args, &["value", "v"])?;
            let selected = app
                .projects
                .iter()
                .find(|project| project.id == id)
                .cloned();
            let has_selected = selected.is_some();

            if let Some(form) = app.entry_form.as_mut() {
                match selected {
                    Some(project) => {
                        form.project_search = project.name.clone();
                        form.selected_project = Some(project);
                        form.filtered_indices.clear();
                    }
                    None => {
                        form.project_search = id.to_string();
                        form.selected_project = None;
                    }
                }
            }

            if !has_selected {
                app.update_project_filter();
            }
        }
        "description" | "desc" | "n" => {
            let value = parse_required_string_alias(args, &["value", "v"])?;
            let form = app
                .entry_form
                .as_mut()
                .ok_or_else(|| "No hay formulario de entrada".to_string())?;
            form.description = value;
        }
        "minutes" | "m" => {
            let value = parse_required_string_alias(args, &["value", "v"])?;
            let form = app
                .entry_form
                .as_mut()
                .ok_or_else(|| "No hay formulario de entrada".to_string())?;
            form.minutes = value;
        }
        "billable" | "b" => {
            let value = parse_bool_alias(args, &["value", "v"], true)?;
            let form = app
                .entry_form
                .as_mut()
                .ok_or_else(|| "No hay formulario de entrada".to_string())?;
            form.is_billable = value;
        }
        "focused" | "focus" => {
            let value = parse_required_string_alias(args, &["value", "v"])?;
            let focused = parse_form_field(value.as_str())?;
            let form = app
                .entry_form
                .as_mut()
                .ok_or_else(|| "No hay formulario de entrada".to_string())?;
            form.focused = focused;
        }
        other => {
            return Err(format!("Campo de entrada no soportado: {other}"));
        }
    }

    Ok(())
}

fn select_project(app: &mut App, args: &ArgsMap) -> Result<(), String> {
    if app.entry_form.is_none() {
        app.open_add_entry();
    }

    if let Some(form) = app.entry_form.as_ref()
        && form.filtered_indices.is_empty()
    {
        app.update_project_filter();
    }

    let index = parse_usize_alias(args, &["index", "i"]).unwrap_or(0);
    let move_next = parse_bool_alias(args, &["move_next", "mn"], true)?;

    let project = {
        let form = app
            .entry_form
            .as_ref()
            .ok_or_else(|| "No hay formulario de entrada".to_string())?;
        let project_idx = *form
            .filtered_indices
            .get(index)
            .ok_or_else(|| format!("No existe proyecto filtrado en indice {index}"))?;
        app.projects
            .get(project_idx)
            .cloned()
            .ok_or_else(|| "Proyecto no encontrado".to_string())?
    };

    let form = app
        .entry_form
        .as_mut()
        .ok_or_else(|| "No hay formulario de entrada".to_string())?;
    form.selected_project = Some(project.clone());
    form.project_search = project.name;
    form.filtered_indices.clear();
    form.list_state.select(None);
    if move_next {
        form.next_field();
    }

    Ok(())
}

fn set_config_field(app: &mut App, args: &ArgsMap) -> Result<(), String> {
    if app.config_form.is_none() {
        app.open_config();
    }

    let field = parse_required_string_alias(args, &["field", "f"])?;
    let value = parse_required_string_alias(args, &["value", "v"])?;

    if matches!(field.as_str(), "theme" | "th") {
        app.config_set_theme_value(value);
        return Ok(());
    }

    let form = app
        .config_form
        .as_mut()
        .ok_or_else(|| "No hay formulario de config".to_string())?;

    match field.as_str() {
        "token" | "t" => form.token = value,
        "base_url" | "url" | "u" => form.base_url = value,
        "default_range" | "range" | "r" => form.default_range = value,
        "focused" | "focus" => {
            form.focused = parse_config_field(value.as_str())?;
        }
        "theme" | "th" => form.theme = value,
        other => {
            return Err(format!("Campo de config no soportado: {other}"));
        }
    }

    Ok(())
}

fn clear_config_field(app: &mut App, args: &ArgsMap) -> Result<(), String> {
    if app.config_form.is_none() {
        app.open_config();
    }

    if let Some(field) = arg(args, &["field", "f"]).and_then(Value::as_str) {
        let target = parse_config_field(field)?;

        if target == ConfigField::Theme {
            app.config_set_theme_value(String::new());
            return Ok(());
        }

        let form = app
            .config_form
            .as_mut()
            .ok_or_else(|| "No hay formulario de config".to_string())?;
        match target {
            ConfigField::Token => form.token.clear(),
            ConfigField::BaseUrl => form.base_url.clear(),
            ConfigField::DefaultRange => form.default_range.clear(),
            ConfigField::Theme => form.theme.clear(),
        }
        return Ok(());
    }

    app.config_clear_field();
    Ok(())
}

fn parse_form_field(value: &str) -> Result<FormField, String> {
    match value {
        "date" | "d" => Ok(FormField::Date),
        "project" | "project_id" | "p" => Ok(FormField::ProjectId),
        "description" | "desc" | "n" => Ok(FormField::Description),
        "minutes" | "m" => Ok(FormField::Minutes),
        "billable" | "b" => Ok(FormField::Billable),
        _ => Err(format!("Campo de formulario no soportado: {value}")),
    }
}

fn parse_config_field(value: &str) -> Result<ConfigField, String> {
    match value {
        "token" | "t" => Ok(ConfigField::Token),
        "base_url" | "url" | "u" => Ok(ConfigField::BaseUrl),
        "default_range" | "range" | "r" => Ok(ConfigField::DefaultRange),
        "theme" | "th" => Ok(ConfigField::Theme),
        _ => Err(format!("Campo de config no soportado: {value}")),
    }
}

fn parse_response_options(
    args: &ArgsMap,
    default_view: SnapshotView,
) -> Result<ResponseOptions, String> {
    let include_structured = parse_bool_alias(args, &["structured", "stc"], false)?;
    let view = parse_snapshot_view(arg(args, &["view", "vw"]), default_view)?;
    let max_days = parse_limit(arg(args, &["max_days", "md"]), 14, 120)?;
    let max_entries = parse_limit(arg(args, &["max_entries_per_day", "me"]), 20, 300)?;
    Ok(ResponseOptions {
        include_structured,
        view,
        max_days,
        max_entries,
    })
}

fn parse_snapshot_view(raw: Option<&Value>, default: SnapshotView) -> Result<SnapshotView, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };

    let value = raw
        .as_str()
        .ok_or_else(|| "view debe ser string".to_string())?
        .trim()
        .to_ascii_lowercase();

    match value.as_str() {
        "none" | "0" => Ok(SnapshotView::None),
        "tiny" | "t" => Ok(SnapshotView::Tiny),
        "normal" | "n" => Ok(SnapshotView::Normal),
        "full" | "f" => Ok(SnapshotView::Full),
        _ => Err(format!("view invalido: {value}")),
    }
}

fn build_snapshot(session_id: &str, app: &App, options: &ResponseOptions) -> Option<Value> {
    match options.view {
        SnapshotView::None => None,
        SnapshotView::Tiny => Some(build_tiny_snapshot(session_id, app)),
        SnapshotView::Normal => Some(build_normal_snapshot(session_id, app)),
        SnapshotView::Full => Some(build_full_snapshot(
            session_id,
            app,
            options.max_days,
            options.max_entries,
        )),
    }
}

fn build_tiny_snapshot(session_id: &str, app: &App) -> Value {
    json!({
        "sid": session_id,
        "im": input_mode_code(app.input_mode),
        "fc": focus_code(app.focus),
        "dr": app.date_range.label(),
        "di": app.day_state.selected(),
        "ei": app.entry_state.selected(),
        "dc": app.days.len(),
        "pc": app.projects.len(),
        "st": clip_text(&app.status, 120)
    })
}

fn build_normal_snapshot(session_id: &str, app: &App) -> Value {
    let mut snapshot = build_tiny_snapshot(session_id, app);

    if let Some(map) = snapshot.as_object_mut() {
        map.insert(
            "sd".to_string(),
            app.selected_day()
                .map(|day| {
                    json!({
                        "d": day.date,
                        "ec": day.entries.len(),
                        "th": day.total_hours()
                    })
                })
                .unwrap_or(Value::Null),
        );

        map.insert(
            "se".to_string(),
            app.selected_entry()
                .map(|entry| {
                    json!({
                        "p": clip_text(&entry.project, 48),
                        "h": entry.hours,
                        "n": clip_text(&entry.note, 140)
                    })
                })
                .unwrap_or(Value::Null),
        );

        map.insert(
            "ef".to_string(),
            app.entry_form
                .as_ref()
                .map(|form| {
                    json!({
                        "f": form_field_code(form.focused),
                        "d": form.date,
                        "p": clip_text(&form.project_search, 48),
                        "m": form.minutes,
                        "b": form.is_billable,
                        "fc": form.filtered_indices.len()
                    })
                })
                .unwrap_or(Value::Null),
        );

        map.insert(
            "cf".to_string(),
            app.config_form
                .as_ref()
                .map(|form| {
                    json!({
                        "f": config_field_code(form.focused),
                        "u": clip_text(&form.base_url, 96),
                        "r": clip_text(&form.default_range, 48),
                        "th": clip_text(&form.theme, 40),
                        "v": build_version(),
                        "t": mask_secret(&form.token)
                    })
                })
                .unwrap_or(Value::Null),
        );
    }

    snapshot
}

fn build_full_snapshot(session_id: &str, app: &App, max_days: usize, max_entries: usize) -> Value {
    let mut snapshot = build_normal_snapshot(session_id, app);

    let days = app
        .days
        .iter()
        .take(max_days)
        .map(|day| {
            let entries = day
                .entries
                .iter()
                .take(max_entries)
                .map(|entry| {
                    json!({
                        "p": clip_text(&entry.project, 64),
                        "h": entry.hours,
                        "n": clip_text(&entry.note, 220)
                    })
                })
                .collect::<Vec<Value>>();

            json!({
                "d": day.date,
                "th": day.total_hours(),
                "ec": day.entries.len(),
                "e": entries
            })
        })
        .collect::<Vec<Value>>();

    if let Some(map) = snapshot.as_object_mut() {
        map.insert("ds".to_string(), Value::Array(days));
    }

    snapshot
}

fn build_tool_result(content: Value, include_structured: bool) -> Value {
    let text = encode_toon_compact(&content);
    if include_structured {
        json!({
            "content": [{
                "type": "text",
                "text": text
            }],
            "structuredContent": content
        })
    } else {
        json!({
            "content": [{
                "type": "text",
                "text": text
            }]
        })
    }
}

fn tool_error_result(message: &str) -> Value {
    let payload = json!({
        "e": "er",
        "m": clip_text(message, 220)
    });

    json!({
        "content": [{
            "type": "text",
            "text": encode_toon_compact(&payload)
        }],
        "isError": true
    })
}

fn encode_toon_compact(value: &Value) -> String {
    let mut options = EncodeOptions::default();
    options.indent = 1;
    options.delimiter = Delimiter::Tab;
    toon::encode(value, Some(options))
}

fn parse_session_id(args: &ArgsMap) -> Result<String, String> {
    parse_required_string_alias(args, &["session_id", "sid"])
}

fn parse_required_string_alias(args: &ArgsMap, keys: &[&str]) -> Result<String, String> {
    let value = arg(args, keys).ok_or_else(|| format!("Falta campo requerido: {}", keys[0]))?;
    parse_string_value(value, keys[0])
}

fn parse_required_i32_alias(args: &ArgsMap, keys: &[&str]) -> Result<i32, String> {
    let value = arg(args, keys).ok_or_else(|| format!("Falta campo requerido: {}", keys[0]))?;
    parse_i32_value(value, keys[0])
}

fn parse_string_value(value: &Value, field: &str) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(flag) => Ok(if *flag { "true" } else { "false" }.to_string()),
        _ => Err(format!("{field} debe ser string/number/bool")),
    }
}

fn parse_i32_value(value: &Value, field: &str) -> Result<i32, String> {
    if let Some(raw) = value.as_i64() {
        return i32::try_from(raw).map_err(|_| format!("{field} fuera de rango"));
    }

    if let Some(raw) = value.as_u64() {
        return i32::try_from(raw).map_err(|_| format!("{field} fuera de rango"));
    }

    if let Some(raw) = value.as_str() {
        return raw
            .trim()
            .parse::<i32>()
            .map_err(|_| format!("{field} debe ser entero"));
    }

    Err(format!("{field} debe ser entero"))
}

fn parse_usize_alias(args: &ArgsMap, keys: &[&str]) -> Option<usize> {
    let value = arg(args, keys)?;
    if let Some(raw) = value.as_u64() {
        return usize::try_from(raw).ok();
    }

    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
}

fn parse_bool_alias(args: &ArgsMap, keys: &[&str], default: bool) -> Result<bool, String> {
    let Some(raw) = arg(args, keys) else {
        return Ok(default);
    };

    parse_boolish(raw).ok_or_else(|| format!("{} debe ser bool", keys[0]))
}

fn parse_boolish(raw: &Value) -> Option<bool> {
    match raw {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => {
            if value.as_i64() == Some(1) || value.as_u64() == Some(1) {
                Some(true)
            } else if value.as_i64() == Some(0) || value.as_u64() == Some(0) {
                Some(false)
            } else {
                None
            }
        }
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" => Some(true),
            "0" | "false" | "no" | "n" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn parse_limit(raw: Option<&Value>, default: usize, max: usize) -> Result<usize, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };

    let value = if let Some(number) = raw.as_u64() {
        number
    } else if let Some(text) = raw.as_str() {
        text.trim()
            .parse::<u64>()
            .map_err(|_| "Los limites deben ser enteros positivos".to_string())?
    } else {
        return Err("Los limites deben ser enteros positivos".to_string());
    };

    if value == 0 {
        return Err("Los limites deben ser mayores a 0".to_string());
    }

    Ok(value.min(max as u64) as usize)
}

fn parse_key_sequence(
    key: &str,
    text: Option<&str>,
) -> Result<Vec<(KeyCode, KeyModifiers)>, String> {
    if key == "text" {
        let text = text.ok_or_else(|| "key=text requiere argumento text".to_string())?;
        if text.is_empty() {
            return Err("text no puede estar vacio".to_string());
        }

        let sequence = text
            .chars()
            .map(|ch| (KeyCode::Char(ch), KeyModifiers::NONE))
            .collect::<Vec<(KeyCode, KeyModifiers)>>();
        return Ok(sequence);
    }

    if key.starts_with("char:") {
        let mut chars = key.trim_start_matches("char:").chars();
        let Some(ch) = chars.next() else {
            return Err("char: requiere un caracter".to_string());
        };
        return Ok(vec![(KeyCode::Char(ch), KeyModifiers::NONE)]);
    }

    let mapped = match key {
        "up" => Some((KeyCode::Up, KeyModifiers::NONE)),
        "down" => Some((KeyCode::Down, KeyModifiers::NONE)),
        "left" => Some((KeyCode::Char('h'), KeyModifiers::NONE)),
        "right" => Some((KeyCode::Char('l'), KeyModifiers::NONE)),
        "enter" => Some((KeyCode::Enter, KeyModifiers::NONE)),
        "esc" => Some((KeyCode::Esc, KeyModifiers::NONE)),
        "tab" => Some((KeyCode::Tab, KeyModifiers::NONE)),
        "backtab" => Some((KeyCode::BackTab, KeyModifiers::NONE)),
        "backspace" => Some((KeyCode::Backspace, KeyModifiers::NONE)),
        "space" => Some((KeyCode::Char(' '), KeyModifiers::NONE)),
        "ctrl+c" => Some((KeyCode::Char('c'), KeyModifiers::CONTROL)),
        "ctrl+r" => Some((KeyCode::Char('r'), KeyModifiers::CONTROL)),
        "ctrl+u" => Some((KeyCode::Char('u'), KeyModifiers::CONTROL)),
        "j" | "k" | "h" | "l" | "q" | "r" | "f" | "n" | "d" | "c" => {
            let ch = key.chars().next().ok_or_else(|| "key vacia".to_string())?;
            Some((KeyCode::Char(ch), KeyModifiers::NONE))
        }
        _ => None,
    };

    if let Some(key_event) = mapped {
        Ok(vec![key_event])
    } else {
        Err(format!(
            "Key no soportada: {key}. Usa text, char:<x>, o teclas del TUI."
        ))
    }
}

fn input_mode_code(value: InputMode) -> &'static str {
    match value {
        InputMode::Normal => "n",
        InputMode::Editing => "e",
        InputMode::AddingEntry => "a",
        InputMode::Configuring => "c",
    }
}

fn focus_code(value: AppFocus) -> &'static str {
    match value {
        AppFocus::Days => "d",
        AppFocus::Entries => "e",
    }
}

fn form_field_code(value: FormField) -> &'static str {
    match value {
        FormField::Date => "d",
        FormField::ProjectId => "p",
        FormField::Description => "n",
        FormField::Minutes => "m",
        FormField::Billable => "b",
    }
}

fn config_field_code(value: ConfigField) -> &'static str {
    match value {
        ConfigField::Token => "t",
        ConfigField::BaseUrl => "u",
        ConfigField::DefaultRange => "r",
        ConfigField::Theme => "h",
    }
}

fn clip_text(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            out.push('~');
            break;
        }
        out.push(ch);
    }
    out
}

fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let keep = 4usize.min(value.len());
    let suffix = &value[value.len() - keep..];
    format!("***{suffix}")
}

fn arg<'a>(args: &'a ArgsMap, keys: &[&str]) -> Option<&'a Value> {
    for key in keys {
        if let Some(value) = args.get(*key) {
            return Some(value);
        }
    }
    None
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn read_framed_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        if line == "\r\n" || line == "\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = trimmed.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let length = content_length.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "No se encontro header Content-Length",
        )
    })?;

    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer)?;
    let payload = String::from_utf8(buffer)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    Ok(Some(payload))
}

fn write_framed_message<W: Write>(writer: &mut W, message: &Value) -> io::Result<()> {
    let payload = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

fn is_help(value: &str) -> bool {
    matches!(value, "-h" | "--help" | "help")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_key_sequence() {
        let sequence = parse_key_sequence("text", Some("abc")).expect("expected valid sequence");
        assert_eq!(sequence.len(), 3);
    }

    #[test]
    fn parse_single_char_sequence() {
        let sequence = parse_key_sequence("char:x", None).expect("expected valid sequence");
        assert_eq!(sequence.len(), 1);
    }

    #[test]
    fn parse_key_sequence_fails_on_unknown() {
        let result = parse_key_sequence("invalid", None);
        assert!(result.is_err());
    }

    #[test]
    fn parse_view_aliases() {
        assert!(matches!(
            parse_snapshot_view(Some(&Value::String("t".to_string())), SnapshotView::Normal)
                .expect("view should parse"),
            SnapshotView::Tiny
        ));
        assert!(matches!(
            parse_snapshot_view(Some(&Value::String("0".to_string())), SnapshotView::Normal)
                .expect("view should parse"),
            SnapshotView::None
        ));
    }

    #[test]
    fn normalize_action_aliases() {
        assert_eq!(normalize_action("nd"), "next_day");
        assert_eq!(normalize_action("sf"), "set_entry_field");
    }
}
