use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::application::app::{App, FormField};
use crate::ui::helpers::centered_rect;

pub fn render_add_entry_modal(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 50, frame.area());

    // Clear the area
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Nuevo Registro")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Date
            Constraint::Length(3), // Project ID
            Constraint::Length(3), // Description
            Constraint::Length(3), // Minutes
            Constraint::Length(3), // Billable
            Constraint::Min(0),
        ])
        .split(area);

    let style_focused = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    let style_normal = Style::default().fg(Color::Reset);

    // Dropdown configuration to be rendered after fields
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
                    .border_style(style);
                let p = Paragraph::new(value).block(block).style(style);
                f.render_widget(p, area);
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

        // Billable
        let is_billable_focused = form.focused == FormField::Billable;
        let b_style = if is_billable_focused {
            style_focused
        } else {
            style_normal
        };
        let b_sym = if form.is_billable { "[x]" } else { "[ ]" };
        let b_p = Paragraph::new(format!("{} Es facturable (espacio)", b_sym))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Opciones")
                    .border_style(b_style),
            )
            .style(b_style);
        frame.render_widget(b_p, chunks[4]);

        if form.focused == FormField::ProjectId && !form.filtered_indices.is_empty() {
            dropdown_info = Some((chunks[1], form.filtered_indices.clone()));
        }

        // Cursor Logic
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
    } // Immutable borrow of app.entry_form ends here

    // Render Dropdown
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
            .map(|p| ListItem::new(format!("{} - {} [{}]", p.id, p.name, p.client_name)))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::Reset)),
            )
            .highlight_style(Style::default().bg(Color::Blue));

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(
            list,
            dropdown_area,
            &mut app.entry_form.as_mut().unwrap().list_state,
        );
    }

    frame.render_widget(
        Paragraph::new("Tab: siguiente | Enter: crear | Esc: cancelar")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[4],
    ); // wait, chunk 4 is used for Billable. The help text should be below.
    // Chunk 5 is min(0), let's render help there if possible or overlay?
    // In original code, help text was overwriting or appending?
    // Original:
    // chunks[4] was "Constraint::Min(0)".
    // Ah, I added "Billable" as new constraint in previous steps in main.rs but here I might have copied logic
    // Let's check constraints in this file:
    //     Constraint::Length(3), // Date
    //     Constraint::Length(3), // Project ID
    //     Constraint::Length(3), // Description
    //     Constraint::Length(3), // Minutes
    //     Constraint::Length(3), // Billable
    //     Constraint::Min(0),    // Space

    // So Billable is at index 4. The help text should be at index 5.

    frame.render_widget(
        Paragraph::new("Tab: siguiente | Shift+Tab: anterior | Enter: crear | Esc: cancelar")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center),
        chunks[5],
    );
}
