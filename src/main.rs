mod events;
mod parser;
mod render;

use crate::events::event::{Event, Events};
use render::App;

use std::error::Error;
use std::path::Path;
use std::{env, io};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;

use tokio;

use parser::{Config, ParserKind};

#[allow(dead_code)]
fn printer(config: &Config) {
    for (gn, group) in config.dep_groups.iter() {
        for (_, dep) in group.iter() {
            let name = &dep.name;
            let specified_version = &dep.get_specified_version();
            let current_version = &dep.get_current_version();
            let latest_version = &dep.get_latest_version();
            let latest_semver_version = &dep.get_latest_semver_version();
            println!(
                "{}: [{}] {}({}) => {}({})",
                gn, name, specified_version, current_version, latest_semver_version, latest_version
            );
        }
    }
}

fn find_type(folder: &str) -> Option<ParserKind> {
    if Path::new(&format!("{}/package-lock.json", folder)).exists() {
        return Some(ParserKind::JavascriptNpm);
    } else if Path::new(&format!("{}/Cargo.lock", folder)).exists() {
        return Some(ParserKind::RustCargo);
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let folder = match args.len() > 1 {
        true => &args[1],
        false => ".",
    };
    let kind = find_type(&folder).expect("Unsupported package manager");
    println!("Fetching dependency info...");
    let config = Config::new(folder, kind).await;
    // printer(&config);

    if true {
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        let events = Events::new();
        let mut app = App::new(config);
        app.next();

        let mut search_in_next_iter: Option<String> = None;
        let mut reload = true;

        loop {
            if reload {
                let config = Config::new(folder, kind).await;
                app = App::new(config);
                app.next();
                reload = false;
            }
            terminal.draw(|mut f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                    .split(f.size());

                let tabl = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
                    .split(chunks[0]);

                app.render_tabs(&mut f, tabl);
                app.render_dependency_info(&mut f, chunks[1]);
                app.render_version_selector(&mut f);
                app.render_help_menu(&mut f);
                app.display_message(&mut f);
                app.display_search_input(&mut f);
                app.render_search_results(&mut f);
            })?;

            if let Some(term) = search_in_next_iter {
                search_in_next_iter = None;
                let result = Config::search_deps(kind, &term).await;
                app.remove_message();
                match result {
                    Ok(r) => app.show_searches(r),
                    _ => app.set_message("Search failed"),
                }
                continue;
            }

            match events.next()? {
                Event::Input(input) => {
                    match app.search_input_mode {
                        true => match input {
                            Key::Char('\n') => {
                                app.search_input_mode = false;
                                app.set_message("Searching...");
                                search_in_next_iter = Some(app.search_string.to_string());
                            }
                            Key::Char(_) | Key::Backspace => app.search_update(input),
                            Key::Esc => {
                                app.search_string = "".to_string();
                                app.search_input_mode = false;
                            }
                            _ => {}
                        },
                        false => match input {
                            Key::Char('q') => {
                                break;
                            }
                            Key::Ctrl('c') => {
                                drop(terminal);
                                std::process::exit(0);
                            }
                            Key::Char('s') => app.search_input_mode = true,
                            Key::Char('o') => app.open_homepage(),
                            Key::Char('p') => app.open_package_repo(),
                            Key::Char('?') => app.toggle_help_menu(), // h is for next tab
                            Key::Esc => {
                                app.hide_popup();
                                app.remove_message();
                                app.show_searches = false;
                            }
                            Key::Char('v') | Key::Char(' ') => app.toggle_popup(),
                            Key::Left | Key::Char('h') => app.tab_previous(),
                            Key::Right | Key::Char('l') => app.tab_next(),
                            Key::Down | Key::Char('j') => app.next(),
                            Key::Up | Key::Char('k') => app.previous(),
                            Key::Char('\n') => {
                                let is_installed =
                                    Config::install_dep(kind, app.get_install_candidate(), folder);
                                if is_installed {
                                    app.set_message("Dependency version updated!");
                                    reload = true;
                                }
                            }
                            Key::Char('g') => app.top(),
                            Key::Char('G') => app.bottom(),
                            _ => {}
                        },
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
