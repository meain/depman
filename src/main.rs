mod events;
#[allow(dead_code)]
mod parser;
mod render;

use crate::events::event::{Event, Events};
use render::App;

use std::io;
use std::error::Error;
use termion::event::Key;
use termion::raw::IntoRawMode;
use tui::backend::{TermionBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;

use tokio;

use parser::{DepListList};

fn printer(dep_list_list: &DepListList) {
    for dep_list in &dep_list_list.lists {
        let kind = dep_list.name.to_string();
        for dep in &dep_list.deps {
            let name = dep.name.to_string();
            let specified_version = &dep.get_specified_version();
            let current_version = &dep.get_current_version();
            let latest_version = &dep.get_latest_version();
            let latest_semver_version = &dep.get_latest_semver_version();
            println!(
                "{}: [{}] {}({}) => {}({})",
                kind,
                name,
                specified_version,
                current_version,
                latest_semver_version,
                latest_version
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let dep_list_list = DepListList::new("tests/node/npm", "javascript-package-json").await;
    printer(&dep_list_list);

    if true {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        terminal.hide_cursor()?;

        let events = Events::new();
        let mut app = App::new(dep_list_list);
        app.next();

        loop {
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
                // app.render_dependency_list(&mut f, chunks[0]);
                app.render_dependency_info(&mut f, chunks[1]);
                app.render_version_selector(&mut f);
                app.render_help_menu(&mut f);
            })?;
            match events.next()? {
                Event::Input(input) => match input {
                    Key::Char('q') => {
                        terminal.clear()?;
                        break;
                    }
                    Key::Char('o') => app.open_homepage(),
                    Key::Char('?') => app.toggle_help_menu(), // h is for next tab
                    Key::Esc => app.hide_popup(),
                    Key::Char('v') | Key::Char(' ') => app.toggle_popup(),
                    Key::Left | Key::Char('h') => app.tab_previous(),
                    Key::Right | Key::Char('l') => app.tab_next(),
                    Key::Down | Key::Char('j') => app.next(),
                    Key::Up | Key::Char('k') => app.previous(),
                    Key::Char('g') => app.top(),
                    Key::Char('G') => app.bottom(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(())
}
