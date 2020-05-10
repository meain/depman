#[allow(dead_code)]
mod parser;
mod events;
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

use parser::{DepListList, DepVersion, DepVersionReq};

async fn fetch_resp(dep: &str) -> Result<Value, Box<dyn Error>> {
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

    let mut counter = 0;
    for dep_list in &mut dep_list_list.lists {
        for dep in &mut dep_list.deps {
            if !results[counter].is_null() {
                dep.author = match &results[counter]["author"] {
                    Value::String(res) => res.to_string(),
                    _ => "<unknown>".to_string()
                };
                dep.desciption = match &results[counter]["desciption"] {
                    Value::String(res) => res.to_string(),
                    _ => "<unknown>".to_string()
                };
                dep.license = match &results[counter]["libcore"] {
                    Value::String(res) => res.to_string(),
                    _ => "<unknown>".to_string()
                };
                if let Value::Object(versions) = &results[counter]["versions"] {
                    let mut key_list: Vec<String> = Vec::new();
                    for key in versions.keys() {
                        // maybe reverse and lookup?
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
                                    latest_semantic_version =
                                        Some(DepVersion::Version(valid_version.clone()));
                                }
                            }
                        };
                    }
                    dep.available_versions = Some(parsed_versions);
                    dep.latest_version = latest_version;
                    dep.latest_semver_version = latest_semantic_version;
                }
            }
            counter += 1;
        }
    }

    Ok(())
}

fn printer(dep_list_list: &DepListList) {
    for dep_list in &dep_list_list.lists {
        let kind = dep_list.name.to_string();
        for dep in &dep_list.deps {
            let name = dep.name.to_string();
            let specified_version = match &dep.specified_version {
                DepVersionReq::Error => "invalid".to_string(),
                DepVersionReq::Version(v) => v.to_string(),
            };
            let current_version = match &dep.current_version {
                DepVersion::Error => "invalid".to_string(),
                DepVersion::Version(v) => v.to_string(),
            };
            let latest_version = match &dep.latest_version {
                Some(version) => match version {
                    DepVersion::Version(ver) => ver.to_string(),
                    DepVersion::Error => "error".to_string(),
                },
                None => "unknown".to_string(),
            };
            let latest_semver_version = match &dep.latest_semver_version {
                Some(version) => match version {
                    DepVersion::Version(ver) => ver.to_string(),
                    DepVersion::Error => "error".to_string(),
                },
                None => "unknown".to_string(),
            };
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
    let mut dep_list_list = DepListList { lists: vec![] };
    let config: Value = parser::get_parsed_json_file("package.json")?;
    let lockfile: Value = parser::get_parsed_json_file("package-lock.json")?;
    // let config: Value = parser::get_parsed_json_file("tests/node/npm/package.json")?;
    // let lockfile: Value = parser::get_parsed_json_file("tests/node/npm/package-lock.json")?;

    let dl = parser::get_dep_list(&config, "dependencies", &lockfile);
    if let Some(d) = dl {
        dep_list_list.lists.push(d);
    }

    let dl = parser::get_dep_list(&config, "devDependencies", &lockfile);
    if let Some(d) = dl {
        dep_list_list.lists.push(d);
    }

    fetch_dep_infos(&mut dep_list_list).await?;
    printer(&dep_list_list);
    // println!("dep_list_list: {:?}", dep_list_list);


    let stdout = io::stdout().into_raw_mode()?;
    let mut backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let events = Events::new();
    let mut app = App::new(dep_list_list.get_dep_names());
    app.next();

    // loop {
    terminal.draw(|mut f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(f.size());

        app.render_dependency_list(&mut f, chunks[0]);
        app.render_dependency_info(&mut f, chunks[1]);
        app.render_version_selector(&mut f);
    })?;
    match events.next()? {
        Event::Input(input) => match input {
            Key::Char('q') => {
                terminal.clear()?;
                // break;
            }
            Key::Esc => app.hide_popup(),
            Key::Char('v') | Key::Char(' ') => app.toggle_popup(),
            Key::Down | Key::Char('j') => app.next(),
            Key::Up | Key::Char('k') => app.previous(),
            _ => {}
        },
        _ => {}
    }
    // }

    Ok(())
}
