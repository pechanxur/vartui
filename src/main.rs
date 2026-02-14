mod application;
mod domain;
mod infrastructure;
mod ui;
mod utils;

use crossterm::event::{self, Event};
use std::env;
use std::io;
use std::time::Duration;

use crate::application::app::App;
use crate::application::cli::{api_help, run_api};
use crate::application::input::handle_key;
use crate::application::mcp::{mcp_help, run_mcp};
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
        Some("mcp") => {
            if let Err(error) = run_mcp(&args[2..]) {
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
        "Uso:\n  {bin} tui\n  {bin} api <subcomando>\n  {bin} mcp\n\nSubcomandos API:\n{}\n\nSubcomandos MCP:\n{}",
        api_help(),
        mcp_help()
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
                if handle_key(&mut app, key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
    }
}
