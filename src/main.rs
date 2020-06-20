mod events;
mod parser;
mod render;

use crate::events::event::{Event, Events};
use render::{App, PopupKind};

use std::error::Error;
use std::{env, io};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;

use parser::determinekind::ParserKind;
use parser::{stringify, Project};

#[allow(dead_code)]
fn printer(config: &Project) {
    for group in config.get_groups().iter() {
        for name in config.get_deps_in_group(group) {
            let name = &name;
            let specified_version = &config.get_specified_version(group, name);
            let current_version = &config.get_current_version(name);
            let latest_version = &config.get_latest_version(name);
            let latest_semver_version = &config.get_semver_version(group, name);
            println!(
                "[{}] {} : {}({}) => {}({})",
                group,
                name,
                stringify(specified_version),
                stringify(current_version),
                stringify(latest_semver_version),
                stringify(latest_version)
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let folder = if args.len() > 1 { &args[1] } else { "." };
    let kind = ParserKind::determine_kind(&folder).expect("Unsupported package manager");
    println!("Fetching dependency info...");
    let project = Project::parse(folder, &kind).await;
    // printer(&project);

    if true {
        // let stdout = io::stdout();

        let stdout = io::stdout().into_raw_mode()?;
        let stdout = MouseTerminal::from(stdout);
        let stdout = AlternateScreen::from(stdout);

        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        let mut events = Events::new();
        let mut app = App::new(project.clone(), kind.clone(), folder);
        app.next();

        let mut search_in_next_iter: Option<String> = None;
        let mut reload = false;

        loop {
            if reload {
                let project = project.reparse(&folder, &kind).await;
                let state = app.get_state();
                app = App::new(project, kind.clone(), folder);
                app.set_state(state);
                reload = false;
                continue;
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
                let result = app.search(&term).await;
                app.remove_message();
                if result {
                    app.show_searches();
                } else {
                    app.set_message("Search failed");
                }
                continue;
            }

            if let Event::Input(input) = events.next()? {
                match app.popup {
                    PopupKind::SearchInput => match input {
                        Key::Char('\n') => {
                            events.enable_exit_key();
                            app.set_message(&format!("Searching {}...", &app.search_string));
                            app.popup = PopupKind::Message;
                            search_in_next_iter = Some(app.search_string.to_string());
                        }
                        Key::Char(_) | Key::Backspace => app.search_update(input),
                        Key::Esc => {
                            events.enable_exit_key();
                            app.search_string = "".to_string();
                            app.popup = PopupKind::None;
                        }
                        _ => {}
                    },
                    _ => match input {
                        Key::Char('q') => {
                            break;
                        }
                        Key::Ctrl('c') => {
                            drop(terminal);
                            std::process::exit(0);
                        }
                        Key::Char('s') => {
                            events.disable_exit_key();
                            app.popup = PopupKind::SearchInput;
                        }
                        Key::Char('D') => {
                            if app.delete_current_dep() {
                                app.set_message("Dependency removed");
                                reload = true;
                            }
                        }
                        Key::Char('o') => app.open_homepage(),
                        Key::Char('p') => app.open_repository(),
                        Key::Char('?') => app.toggle_help_menu(), // h is for next tab
                        Key::Esc => {
                            app.unwrap_popup();
                        }
                        Key::Char('v') | Key::Char(' ') => app.toggle_versions_menu(),
                        Key::Left | Key::Char('h') | Key::BackTab => app.tab_previous(),
                        Key::Right | Key::Char('l') | Key::Char('\t') => app.tab_next(),
                        Key::Down | Key::Char('j') => app.next(),
                        Key::Up | Key::Char('k') => app.previous(),
                        Key::Char('\n') => {
                            let is_installed = app.install_dep();
                            if is_installed {
                                app.set_message("Dependency updated!");
                            } else {
                                app.set_message("Update failed.");
                            }
                            reload = true;
                        }
                        Key::Char('g') => app.top(),
                        Key::Char('G') => app.bottom(),
                        Key::Char('R') => reload = true,
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}
