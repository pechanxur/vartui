mod application;
mod domain;
mod infrastructure;
mod ui;
mod utils;

use std::io;
use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::application::app::{App, InputMode};
use crate::ui::tui::{setup_terminal, restore_terminal};
use crate::ui::ui;

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, App::new());
    restore_terminal(&mut terminal)?;
    result
}

fn run_app(terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;
        
        app.check_background_load();

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }

                if app.input_mode == InputMode::Editing {
                    match key.code {
                        KeyCode::Esc => app.cancel_input(),
                        KeyCode::Enter => app.submit_input(),
                        KeyCode::Backspace => app.input_backspace(),
                        KeyCode::Char(value) => app.input_push(value),
                        _ => {}
                    }
                } else if app.input_mode == InputMode::AddingEntry {
                    match key.code {
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
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => app.next_day(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous_day(),
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Char('f') => app.start_input(),
                        KeyCode::Char('n') => app.open_add_entry(),
                        _ => {}
                    }
                }
            }
        }
    }
}
