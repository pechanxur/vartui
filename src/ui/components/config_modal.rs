use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::application::app::{App, ConfigField};
use crate::ui::helpers::centered_rect;

pub fn render_config_modal(frame: &mut Frame, app: &mut App) {
    if app.config_form.is_none() { return; }
    
    let area = centered_rect(60, 45, frame.size());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title("Configuracion Local")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Token
            Constraint::Length(3), // Base URL
            Constraint::Length(3), // Default Range
            Constraint::Min(1),    // Help
        ])
        .split(area);
        
    let style_focused = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let style_normal = Style::default().fg(Color::Reset);
    
    let form = app.config_form.as_ref().unwrap();

    let render_field = |f: &mut Frame, title: &str, value: &str, field: ConfigField, area: Rect| {
        let is_focused = form.focused == field;
        let style = if is_focused { style_focused } else { style_normal };
        
        // Mask token if not focused ? Or always mask? Let's show last 4 chars if focused
        let display_value = if field == ConfigField::Token && !is_focused {
             if value.len() > 4 {
                 format!("...{}", &value[value.len()-4..])
             } else {
                 "***".to_string()
             }
        } else {
            value.to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(style);
        let p = Paragraph::new(display_value).block(block).style(style);
        f.render_widget(p, area);
    };

    render_field(frame, "VAR Token", &form.token, ConfigField::Token, chunks[0]);
    render_field(frame, "Base URL", &form.base_url, ConfigField::BaseUrl, chunks[1]);
    render_field(frame, "Rango Default", &form.default_range, ConfigField::DefaultRange, chunks[2]);
    
    // Help with format examples
    let help_text = format!(
        "Formatos: AUTO | AUTO-WEEK | AUTO-MONTH | YYYY-MM-DD..YYYY-MM-DD\n\
         Ejemplos: AUTO (mes actual), AUTO-WEEK (semana actual)\n\n\
         Tab: siguiente | Ctrl+U: limpiar | Enter: guardar | Esc: cancelar{}",
        if app.status.contains("Error") || app.status.contains("guardada") { format!("\n{}", app.status) } else { String::new() }
    );
    frame.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[3],
    );
    
    // Cursor
    let (cursor_rect, text_len) = match form.focused {
         ConfigField::Token => (chunks[0], form.token.chars().count()),
         ConfigField::BaseUrl => (chunks[1], form.base_url.chars().count()),
         ConfigField::DefaultRange => (chunks[2], form.default_range.chars().count()),
    };
    
    if cursor_rect.width > 0 {
         frame.set_cursor(
             cursor_rect.x + 1 + text_len as u16,
             cursor_rect.y + 1
         );
    }
}
