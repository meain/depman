mod utils;

use futures::future::try_join_all;
use humanesort::prelude::*;
use semver;
use serde_json::Value;
use std::error::Error;
use tokio;

#[derive(Debug, Clone)]
enum DepVersion {
    Error,
    Version(semver::Version),
    // might have to add stuff like guthub repo or file here
}

#[derive(Debug, Clone)]
enum DepVersionReq {
    Error,
    Version(semver::VersionReq),
    // might have to add stuff like guthub repo or file here
}

#[derive(Debug, Clone)]
struct Dep {
    name: String,
    specified_version: DepVersionReq, // from config files
    current_version: DepVersion,      // parsed from lockfiles
    available_versions: Option<Vec<DepVersion>>,
    latest_version: Option<DepVersion>,
    latest_semver_version: Option<DepVersion>,
}

#[derive(Debug, Clone)]
struct DepList {
    name: String,
    deps: Vec<Dep>, // Could be hashmap, but that might cause if someone lets multiple versions to exist
}

#[derive(Debug, Clone)]
struct DepListList {
    lists: Vec<DepList>,
}

fn get_lockfile_version(lockfile: &Value, name: &str) -> DepVersion {
    if let Value::Object(deps) = &lockfile["dependencies"] {
        if deps.contains_key(name) {
            if let Value::Object(value) = &deps[name] {
                if let Value::String(ver) = &value["version"] {
                    if let Ok(sv) = semver::Version::parse(ver) {
                        return DepVersion::Version(sv);
                    }
                }
            }
        }
    }
    DepVersion::Error
}

fn get_dep_list(data: &Value, name: &str, lockfile: &Value) -> Option<DepList> {
    if !data[name].is_null() {
        let mut dep_list = DepList {
            name: name.to_string(),
            deps: vec![],
        };

        let deps = &data[name];
        if let Value::Object(dl) = deps {
            for (key, value) in dl {
                match value {
                    Value::String(v) => {
                        let specified_version = match semver::VersionReq::parse(v) {
                            Ok(ver) => DepVersionReq::Version(ver),
                            Err(_) => DepVersionReq::Error,
                        };
                        let d = Dep {
                            name: key.to_string(),
                            specified_version: specified_version,
                            current_version: get_lockfile_version(&lockfile, &key),
                            available_versions: None,
                            latest_version: None,
                            latest_semver_version: None,
                        };
                        dep_list.deps.push(d);
                    }
                    _ => {
                        let d = Dep {
                            name: key.to_string(),
                            specified_version: DepVersionReq::Error,
                            current_version: get_lockfile_version(&lockfile, &key),
                            available_versions: None,
                            latest_version: None,
                            latest_semver_version: None,
                        };
                        dep_list.deps.push(d);
                    }
                }
            }
        }
        return Some(dep_list);
    }
    None
}

async fn fetch_resp(dep: &str) -> Result<Value, Box<dyn Error>> {
    let url = format!("https://registry.npmjs.org/{}", dep);
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dep_list_list = DepListList { lists: vec![] };
    // let config: Value = utils::get_parsed_json_file("package.json")?;
    // let lockfile: Value = utils::get_parsed_json_file("package-lock.json")?;
    let config: Value = utils::get_parsed_json_file("tests/node/npm/package.json")?;
    let lockfile: Value = utils::get_parsed_json_file("tests/node/npm/package-lock.json")?;

    let dl = get_dep_list(&config, "dependencies", &lockfile);
    if let Some(d) = dl {
        dep_list_list.lists.push(d);
    }

    let dl = get_dep_list(&config, "devDependencies", &lockfile);
    if let Some(d) = dl {
        dep_list_list.lists.push(d);
    }

    fetch_dep_infos(&mut dep_list_list).await?;
    printer(&dep_list_list);
    // println!("dep_list_list: {:?}", dep_list_list);

    Ok(())
}
