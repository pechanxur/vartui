mod application;
mod domain;
mod infrastructure;
mod ui;
mod utils;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::env;
use std::io;
use std::time::Duration;

use crate::application::app::{App, AppFocus, InputMode};
use crate::application::cli::{api_help, run_api};
use crate::ui::tui::{restore_terminal, setup_terminal};
use crate::ui::ui;

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("tui") => run_tui(),
        Some("api") => {
            if let Err(error) = run_api(&args[2..]) {
                eprintln!("{error}");
                std::process::exit(1);
            }
            Ok(())
        }
        Some("-h") | Some("--help") | Some("help") => {
            print_help(&args[0]);
            Ok(())
        }
        Some(other) => {
            eprintln!("Comando desconocido: {other}\n");
            print_help(&args[0]);
            std::process::exit(1);
        }
    }
}

fn run_tui() -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, App::new());
    restore_terminal(&mut terminal)?;
    result
}

fn print_help(bin: &str) {
    println!(
        "Uso:\n  {bin} tui\n  {bin} api <subcomando>\n\nSubcomandos API:\n{}",
        api_help()
    );
}

fn run_app(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
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
                } else if app.input_mode == InputMode::Configuring {
                    match key.code {
                        KeyCode::Esc => app.close_config(),
                        KeyCode::Tab => app.config_next_field(),
                        KeyCode::Enter => app.save_config_form(),
                        KeyCode::Backspace => app.config_backspace(),
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.config_clear_field()
                        }
                        KeyCode::Char(value) => app.config_input(value),
                        _ => {}
                    }
                } else {
                    // Normal mode - handle focus-dependent navigation
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
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
                }
            }
        }
    }
}
