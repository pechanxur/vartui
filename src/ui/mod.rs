pub mod components;
pub mod tui;
pub mod helpers;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use chrono::{Datelike, Local};

use crate::application::app::{App, AppFocus, InputMode};
use crate::utils::parsing::parse_date;
use crate::ui::components::entry_modal::render_add_entry_modal;
use crate::ui::components::config_modal::render_config_modal;

pub fn ui(frame: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(3)])
        .split(frame.size());

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(layout[0]);

    let day_items: Vec<ListItem> = app
        .days
        .iter()
        .map(|day| {
             let hours = day.total_hours();
             let date_parsed = parse_date(&day.date).unwrap_or_else(|| Local::now().date_naive());
             let weekday = date_parsed.weekday();
             
             let target = match weekday {
                 chrono::Weekday::Fri => 8.0,
                 chrono::Weekday::Sat | chrono::Weekday::Sun => 0.0,
                 _ => 9.0, // Mon-Thu
             };

             let is_future = date_parsed > Local::now().date_naive();
             
             let color = if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
                 if hours > 0.0 { Color::Green } else { Color::White }
             } else if is_future {
                 Color::White
             } else if hours >= target {
                 Color::Green
             } else {
                 Color::Red 
             };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{}  ", day.date), Style::default()),
                Span::styled(format!("{:>4.1}h", hours), Style::default().fg(color)),
            ]))
        })
        .collect();

    let range_label = app.date_range.label();
    let days_title = if app.days.is_empty() {
        format!("Dias (0/0) {}", range_label)
    } else {
        format!(
            "Dias ({}/{}) {}",
            app.selected_index() + 1,
            app.days.len(),
            range_label
        )
    };
    let days_list = List::new(day_items)
        .block(Block::default().title(days_title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("-> ");

    frame.render_stateful_widget(days_list, top[0], &mut app.day_state);

    let (detail_title, entries) = match app.selected_day() {
        Some(day) => (format!("Registros - {}", day.date), day.entries.as_slice()),
        None => ("Registros".to_string(), &[][..]),
    };

    let entry_items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            ListItem::new(format!(
                "{:<14} {:>4.1}h  {}",
                entry.project, entry.hours, entry.note
            ))
        })
        .collect();

    let entries_list = List::new(entry_items)
        .block(Block::default().title(detail_title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(entries_list, top[1], &mut app.entry_state);

    let (actions_text, prompt_len) = if app.input_mode == InputMode::Editing {
        let prompt = "Rango (YYYY-MM-DD..YYYY-MM-DD): ";
        let text = format!(
            "{}{}  {}  |  Enter: aplicar  Esc: cancelar",
            prompt, app.input, app.status
        );
        (text, Some(prompt.len()))
    } else {
        let actions = if app.focus == AppFocus::Entries {
            format!(
                "j/k: mover | h: volver | d: duplicar | q: salir |  {}",
                app.status
            )
        } else {
            format!(
                "j/k: mover | l: entries | f: rango | r: refrescar | n: nuevo | c: config | q: salir |  {}",
                app.status
            )
        };
        (actions, None)
    };

    let actions_block = Block::default().title("Acciones").borders(Borders::ALL);
    let actions_area = actions_block.inner(layout[1]);
    let actions = Paragraph::new(actions_text)
        .block(actions_block)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(actions, layout[1]);

    if let Some(prompt_len) = prompt_len {
        let cursor_x = actions_area.x + (prompt_len + app.input.chars().count()) as u16;
        let cursor_y = actions_area.y;
        let max_x = actions_area.x + actions_area.width.saturating_sub(1);
        frame.set_cursor(cursor_x.min(max_x), cursor_y);
    }

    if app.input_mode == InputMode::AddingEntry {
        render_add_entry_modal(frame, app);
    }
    
    if app.input_mode == InputMode::Configuring {
        render_config_modal(frame, app);
    }
}
