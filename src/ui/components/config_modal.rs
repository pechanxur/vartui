use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::application::app::{App, ConfigField};
use crate::ui::helpers::centered_rect;
use crate::ui::theme::{palette_with_override, resolve_theme_name, THEME_CATALOG};
use crate::utils::version::build_version;

pub fn render_config_modal(frame: &mut Frame, app: &mut App) {
    if app.config_form.is_none() {
        return;
    }

    let area = centered_rect(72, 60, frame.area());
    frame.render_widget(Clear, area);

    let preview_theme = app.config_form.as_ref().map(|form| form.theme.as_str());
    let palette = palette_with_override(&app.config, preview_theme);
    let version = build_version();

    let block = Block::default()
        .title(format!("Configuracion Local [{}]", version))
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
            Constraint::Min(1),
        ])
        .split(area);

    let style_focused = Style::default()
        .fg(palette.accent)
        .add_modifier(Modifier::BOLD);
    let style_normal = Style::default().fg(palette.fg);

    let (show_theme_dropdown, current_theme) = {
        let form = app.config_form.as_ref().unwrap();
        let show_theme_dropdown = form.focused == ConfigField::Theme;
        let current_theme = form.theme.clone();

        let render_field =
            |f: &mut Frame, title: &str, value: &str, field: ConfigField, field_area: Rect| {
                let is_focused = form.focused == field;
                let style = if is_focused {
                    style_focused
                } else {
                    style_normal
                };

                let display_value = if field == ConfigField::Token && !is_focused {
                    if value.len() > 4 {
                        format!("...{}", &value[value.len() - 4..])
                    } else {
                        "***".to_string()
                    }
                } else {
                    value.to_string()
                };

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(style)
                    .style(Style::default().bg(palette.bg));
                let paragraph = Paragraph::new(display_value).block(block).style(style);
                f.render_widget(paragraph, field_area);
            };

        render_field(
            frame,
            "VAR Token",
            &form.token,
            ConfigField::Token,
            chunks[0],
        );
        render_field(
            frame,
            "Base URL",
            &form.base_url,
            ConfigField::BaseUrl,
            chunks[1],
        );
        render_field(
            frame,
            "Rango Default",
            &form.default_range,
            ConfigField::DefaultRange,
            chunks[2],
        );
        render_field(
            frame,
            "Tema (lista)",
            &form.theme,
            ConfigField::Theme,
            chunks[3],
        );

        let theme_preview = resolve_theme_name(&form.theme).slug();
        let theme_catalog = THEME_CATALOG.join(", ");
        let help_text = format!(
            "Version build: {}\n\
             Formatos de rango: AUTO | AUTO-WEEK | AUTO-MONTH | YYYY-MM-DD..YYYY-MM-DD\n\
             Tema actual: {} (aplicado: {})\n\
             Catalogo: {}\n\
             Tab/Shift+Tab: campo | Up/Down: tema (cuando Tema esta activo) | Ctrl+U: limpiar | Ctrl+R: restablecer | Enter: guardar | Esc: cancelar{}",
            version,
            form.theme,
            theme_preview,
            theme_catalog,
            if app.status.contains("Error")
                || app.status.contains("guardada")
                || app.status.contains("restablecida")
                || app.status.contains("No hay token")
            {
                format!("\n{}", app.status)
            } else {
                String::new()
            }
        );

        frame.render_widget(
            Paragraph::new(help_text)
                .style(Style::default().fg(palette.muted))
                .alignment(ratatui::layout::Alignment::Center),
            chunks[4],
        );

        let (cursor_rect, text_len) = match form.focused {
            ConfigField::Token => (chunks[0], form.token.chars().count()),
            ConfigField::BaseUrl => (chunks[1], form.base_url.chars().count()),
            ConfigField::DefaultRange => (chunks[2], form.default_range.chars().count()),
            ConfigField::Theme => (chunks[3], form.theme.chars().count()),
        };

        if cursor_rect.width > 0 {
            frame.set_cursor_position((cursor_rect.x + 1 + text_len as u16, cursor_rect.y + 1));
        }
        (show_theme_dropdown, current_theme)
    };

    if show_theme_dropdown {
        let dropdown_area = Rect {
            x: chunks[3].x,
            y: chunks[3].y + 3,
            width: chunks[3].width,
            height: (THEME_CATALOG.len() as u16 + 2).min(10),
        };

        let items: Vec<ListItem> = THEME_CATALOG
            .iter()
            .map(|theme| {
                let marker = if *theme == current_theme { "*" } else { " " };
                ListItem::new(format!("{} {}", marker, theme))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Temas")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(palette.accent))
                    .style(Style::default().bg(palette.selection).fg(palette.fg)),
            )
            .highlight_style(Style::default().bg(palette.accent).fg(palette.bg))
            .highlight_symbol("-> ");

        frame.render_widget(Clear, dropdown_area);
        frame.render_stateful_widget(
            list,
            dropdown_area,
            &mut app.config_form.as_mut().unwrap().theme_list_state,
        );
    }
}
