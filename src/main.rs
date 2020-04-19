mod utils;

use serde_json::Value;
use futures::future::try_join_all;
use std::error::Error;
use tokio;
use humanesort::prelude::*;

#[derive(Debug, Clone)]
enum DepVersion {
    None,
    Error,
    Version(String),
    // might have to add stuff like guthub repo or file here
}

#[derive(Debug, Clone)]
struct Dep {
    name: String,
    specified_version: DepVersion, // from config files
    current_version: DepVersion,   // parsed from lockfiles
    available_versions: Option<Vec<String>>,
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
                if let Value::String(value) = &value["version"] {
                    return DepVersion::Version(value.to_string());
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
                        let d = Dep {
                            name: key.to_string(),
                            specified_version: DepVersion::Version(v.to_string()),
                            current_version: get_lockfile_version(&lockfile, &key),
                            available_versions: None,
                        };
                        dep_list.deps.push(d);
                    }
                    _ => {
                        let d = Dep {
                            name: key.to_string(),
                            specified_version: DepVersion::Error,
                            current_version: get_lockfile_version(&lockfile, &key),
                            available_versions: None,
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
    let resp = reqwest::get(&url)
        .await?
        .json()
        .await?;
    Ok(resp)

}

async fn fetch_dep_infos(dep_list_list: &mut DepListList) -> Result<(), Box<dyn Error + 'static>> {
    let mut gets = vec!();
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
                    for key in versions.keys(){
                        key_list.push(key.to_string());
                    }
                    key_list.humane_sort();
                    dep.available_versions = Some(key_list);
                }
            }
            counter += 1;
        }
    }

    Ok(())
}

fn printer(dep_list_list: &DepListList){
    for dep_list in &dep_list_list.lists {
        let kind = dep_list.name.to_string();
        for dep in &dep_list.deps {
            let name = dep.name.to_string();
            let specified_version = match &dep.specified_version {
                DepVersion::None => "unknown".to_string(),
                DepVersion::Error => "invalid".to_string(),
                DepVersion::Version(v) => v.to_string()
            };
            let current_version = match &dep.current_version {
                DepVersion::None => "unknown".to_string(),
                DepVersion::Error => "invalid".to_string(),
                DepVersion::Version(v) => v.to_string()
            };
            let latest_version = match &dep.available_versions{
                None => "unknown".to_string(),
                Some(versions) => {
                    match versions.last() {
                        Some(v) => v.to_string(),
                        None => "unknown".to_string()
                    }
                }
            };
            println!("{}: [{}] {}({}) => {}", kind, name, specified_version, current_version, latest_version);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dep_list_list = DepListList { lists: vec![] };
    let config: Value = utils::get_parsed_json_file("package.json")?;
    let lockfile: Value = utils::get_parsed_json_file("package-lock.json")?;

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
