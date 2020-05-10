mod events;
#[allow(dead_code)]
mod parser;
mod render;

use crate::events::event::{Event, Events};
use render::App;
use std::io;
use termion::event::Key;
use termion::raw::IntoRawMode;
use tui::backend::{Backend, TermionBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;

use futures::future::try_join_all;
use humanesort::prelude::*;
use semver;
use serde_json::Value;
use std::error::Error;
use tokio;

use parser::{Dep, DepListList, DepVersion, DepVersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Author {
    name: String,
    url: Option<String>,
}

impl Author {
    pub fn to_string(&self) -> String {
        self.name.to_string()
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct MockVersionRight {
    version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmResponse {
    name: String,
    author: Option<Author>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    versions: HashMap<String, MockVersionRight>, // TODO: remove this Value from here
}

impl NpmResponse {
    pub fn get_versions_list(&self) -> Vec<DepVersion> {
        let mut versions = vec![];
        for key in self.versions.keys() {
            versions.push(DepVersion::from(Some(key.clone())))
        }
        versions
    }

    pub fn inject_inportant_versions(&self, dep: &mut Dep) {
        let mut key_list: Vec<String> = Vec::new();
        for key in self.versions.keys() {
            key_list.push(key.to_string());
        }
        key_list.humane_sort();

        let mut parsed_versions: Vec<DepVersion> = Vec::new();
        let mut latest_semantic_version: Option<DepVersion> = None;
        let mut latest_version: Option<DepVersion> = None;
        for key in key_list {
            if let Ok(valid_version) = semver::Version::parse(&key) {
                parsed_versions.push(DepVersion::Version(valid_version.clone()));
                latest_version = Some(DepVersion::Version(valid_version.clone()));
                if let DepVersionReq::Version(spec) = &dep.specified_version {
                    if spec.matches(&valid_version) {
                        latest_semantic_version = Some(DepVersion::Version(valid_version.clone()));
                    }
                }
            };
        }
        dep.available_versions = Some(parsed_versions);
        dep.latest_version = latest_version;
        dep.latest_semver_version = latest_semantic_version;
    }
}

async fn fetch_resp(dep: &str) -> Result<NpmResponse, Box<dyn Error>> {
    // let url = format!("https://registry.npmjs.org/{}", dep);
    let url = format!("http://localhost:8000/{}.json", dep);
    let resp = reqwest::get(&url).await?.json().await?;
    Ok(resp)
}

async fn fetch_dep_infos(dep_list_list: &mut DepListList) -> Result<(), Box<dyn Error + 'static>> {
    let mut gets = vec![];
    for dep_list in &dep_list_list.lists {
        for dep in &dep_list.deps {
            let get = fetch_resp(&dep.name);
            gets.push(get);
        }
    }
    let results = try_join_all(gets).await?;

    for dep_list in &mut dep_list_list.lists {
        for mut dep in &mut dep_list.deps {
            for result in &results {
                if &result.name == &dep.name {
                    dep.description = result.description.clone();
                    dep.available_versions = Some(result.get_versions_list());
                    dep.license = result.license.clone();
                    dep.homepage = result.homepage.clone();
                    result.inject_inportant_versions(&mut dep);
                }
            }
        }
    }

    Ok(())
}

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
    let mut dep_list_list = DepListList::new("tests/node/npm");
    fetch_dep_infos(&mut dep_list_list).await?;
    printer(&dep_list_list);

    let stdout = io::stdout().into_raw_mode()?;
    let mut backend = TermionBackend::new(stdout);
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
                Key::Char('?') => app.toggle_help_menu(),  // h is for next tab
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

    Ok(())
}
