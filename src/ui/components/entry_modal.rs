use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::application::app::{App, FormField};
use crate::ui::helpers::centered_rect;
use crate::ui::theme::palette_from_config;

pub fn render_add_entry_modal(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 50, frame.area());
    let palette = palette_from_config(&app.config);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Nuevo Registro")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .style(Style::default().bg(palette.bg).fg(palette.fg));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let style_focused = Style::default()
        .fg(palette.accent)
        .add_modifier(Modifier::BOLD);
    let style_normal = Style::default().fg(palette.fg);

    let mut dropdown_info = None;

    {
        let form = app.entry_form.as_ref().unwrap();

        let render_field =
            |f: &mut Frame, title: &str, value: &str, field: FormField, area: Rect| {
                let is_focused = form.focused == field;
                let style = if is_focused {
                    style_focused
                } else {
                    style_normal
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style)
                    .style(Style::default().bg(palette.bg));
                let paragraph = Paragraph::new(value).block(block).style(style);
                f.render_widget(paragraph, area);
            };

        render_field(
            frame,
            "Fecha (YYYY-MM-DD)",
            &form.date,
            FormField::Date,
            chunks[0],
        );
        render_field(
            frame,
            "Proyecto (Busca...)",
            &form.project_search,
            FormField::ProjectId,
            chunks[1],
        );
        render_field(
            frame,
            "Descripcion",
            &form.description,
            FormField::Description,
            chunks[2],
        );
        render_field(
            frame,
            "Duracion (HH:MM)",
            &form.minutes,
            FormField::Minutes,
            chunks[3],
        );

        let is_billable_focused = form.focused == FormField::Billable;
        let billable_style = if is_billable_focused {
            style_focused
        } else {
            style_normal
        };
        let billable_symbol = if form.is_billable { "[x]" } else { "[ ]" };
        let billable = Paragraph::new(format!("{} Es facturable (espacio)", billable_symbol))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Opciones")
                    .border_style(billable_style)
                    .style(Style::default().bg(palette.bg)),
            )
            .style(billable_style);
        frame.render_widget(billable, chunks[4]);

        if form.focused == FormField::ProjectId && !form.filtered_indices.is_empty() {
            dropdown_info = Some((chunks[1], form.filtered_indices.clone()));
        }

        let (cursor_rect, text_len) = match form.focused {
            FormField::Date => (chunks[0], form.date.chars().count()),
            FormField::ProjectId => (chunks[1], form.project_search.chars().count()),
            FormField::Description => (chunks[2], form.description.chars().count()),
            FormField::Minutes => (chunks[3], form.minutes.chars().count()),
            _ => (Rect::default(), 0),
        };

        if cursor_rect.width > 0 {
            frame.set_cursor_position((cursor_rect.x + 1 + text_len as u16, cursor_rect.y + 1));
        }
    }

    if let Some((area_ref, indices)) = dropdown_info {
        let dropdown_area = Rect {
            x: area_ref.x,
            y: area_ref.y + 3,
            width: area_ref.width,
            height: 10.min(indices.len() as u16 + 2),
        };

        let items: Vec<ListItem> = indices
            .iter()
            .filter_map(|&idx| app.projects.get(idx))
            .map(|project| {
                ListItem::new(format!(
                    "{} - {} [{}]",
                    project.id, project.name, project.client_name
                ))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.accent))
                    .style(Style::default().bg(palette.selection).fg(palette.fg)),
            )
            .highlight_style(Style::default().bg(palette.accent).fg(palette.bg));

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(
            list,
            dropdown_area,
            &mut app.entry_form.as_mut().unwrap().list_state,
        );
    }

    frame.render_widget(
        Paragraph::new("Tab: siguiente | Shift+Tab: anterior | Enter: crear | Esc: cancelar")
            .style(Style::default().fg(palette.muted))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[5],
    );
}
