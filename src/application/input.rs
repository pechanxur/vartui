use crossterm::event::{KeyCode, KeyModifiers};

use crate::application::app::{App, AppFocus, InputMode};

pub fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        return true;
    }

    if app.input_mode == InputMode::Editing {
        match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.submit_input(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(value) => app.input_push(value),
            _ => {}
        }
        return false;
    }

    if app.input_mode == InputMode::AddingEntry {
        match code {
            KeyCode::Esc => app.close_add_entry(),
            KeyCode::BackTab => app.form_prev_field(),
            KeyCode::Tab => app.form_next_field(),
            KeyCode::Enter => app.form_enter(),
            KeyCode::Up => app.form_nav_up(),
            KeyCode::Down => app.form_nav_down(),
            KeyCode::Backspace => app.form_input_backspace(),
            KeyCode::Char(value) => app.form_input_push(value),
            _ => {}
        }
        return false;
    }

    if app.input_mode == InputMode::Configuring {
        match code {
            KeyCode::Esc => app.close_config(),
            KeyCode::BackTab => app.config_prev_field(),
            KeyCode::Tab => app.config_next_field(),
            KeyCode::Up => app.config_theme_previous(),
            KeyCode::Down => app.config_theme_next(),
            KeyCode::Enter => app.save_config_form(),
            KeyCode::Backspace => app.config_backspace(),
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.config_clear_field()
            }
            KeyCode::Char('r') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.config_reset_defaults()
            }
            KeyCode::Char(value) => app.config_input(value),
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Down | KeyCode::Char('j') => {
            if app.focus == AppFocus::Entries {
                app.next_entry();
            } else {
                app.next_day();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.focus == AppFocus::Entries {
                app.previous_entry();
            } else {
                app.previous_day();
            }
        }
        KeyCode::Char('l') => app.focus_entries(),
        KeyCode::Char('h') | KeyCode::Esc => app.focus_days(),
        KeyCode::Char('d') => app.open_duplicate_entry(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('f') => app.start_input(),
        KeyCode::Char('n') => app.open_add_entry(),
        KeyCode::Char('c') => app.open_config(),
        _ => {}
    }

    false
}
