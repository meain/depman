#[allow(dead_code)]
use serde_json::{Result, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// use std::error::Error;

#[derive(Debug, Clone)]
pub enum DepVersion {
    Error,
    Version(semver::Version),
    // might have to add stuff like guthub repo or file here
}

impl DepVersion {
    pub fn to_string(&self) -> String {
        match self {
            DepVersion::Error => "<error>".to_string(),
            DepVersion::Version(v) => v.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DepVersionReq {
    Error,
    Version(semver::VersionReq),
    // might have to add stuff like guthub repo or file here
}

impl DepVersionReq {
    pub fn to_string(&self) -> String {
        match self {
            DepVersionReq::Error => "<error>".to_string(),
            DepVersionReq::Version(v) => v.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub author: String,
    pub description: String,
    pub homepage: String,
    pub license: String,
    pub specified_version: DepVersionReq, // from config files
    pub current_version: DepVersion,      // parsed from lockfiles
    pub available_versions: Option<Vec<DepVersion>>,
    pub latest_version: Option<DepVersion>,
    pub latest_semver_version: Option<DepVersion>,
}

impl Dep {
    pub fn get_specified_version(&self) -> String {
        self.specified_version.to_string()
    }
    pub fn get_current_version(&self) -> String {
        self.specified_version.to_string()
    }
    pub fn get_latest_version(&self) -> String {
        match &self.latest_version {
            Some(v) => v.to_string(),
            None => "<unknown>".to_string(),
        }
    }
    pub fn get_latest_semver_version(&self) -> String {
        match &self.latest_version {
            Some(v) => v.to_string(),
            None => "<unknown>".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepList {
    pub name: String,
    pub deps: Vec<Dep>, // Could be hashmap, but that might cause if someone lets multiple versions to exist
}

#[derive(Debug, Clone)]
pub struct DepListList {
    pub lists: Vec<DepList>,
}

impl DepListList {
    pub fn get_dep_kinds(&self) -> Vec<String> {
        let mut kinds = vec![];
        for dep_list in &self.lists {
            let kind = dep_list.name.clone();
            kinds.push(kind)
        }
        kinds
    }

    pub fn get_dep(&mut self, dep_name: &str) -> Option<Dep> {
        for dep_list in &self.lists {
            for dep in &dep_list.deps {
                if dep_name == dep.name {
                    return Some(dep.clone());
                }
            }
        }
        None
    }
    pub fn get_dep_names(&self) -> Vec<String> {
        let mut deps = vec![];
        for dep_list in &self.lists {
            for dep in &dep_list.deps {
                let name = dep.name.to_string();
                deps.push(name);
            }
        }
        deps
    }
    pub fn get_dep_names_of_kind(&self, kind: &str) -> Vec<String> {
        let mut deps = vec![];
        for dep_list in &self.lists {
            if kind != &dep_list.name {
                continue;
            }
            for dep in &dep_list.deps {
                let name = dep.name.to_string();
                deps.push(name);
            }
        }
        deps
    }
}

pub fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("Unable to open file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

pub fn get_parsed_json_file(filename: &str) -> Result<Value> {
    let lines = lines_from_file(filename);
    let config: Value = serde_json::from_str(&lines.join("\n"))?;
    Ok(config)
}

pub fn get_lockfile_version(lockfile: &Value, name: &str) -> DepVersion {
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

pub fn get_dep_list(data: &Value, name: &str, lockfile: &Value) -> Option<DepList> {
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
                            author: "<unknown>".to_string(),
                            description: "<unknown>".to_string(),
                            homepage: "<unknown>".to_string(),
                            license: "<unknown>".to_string(),
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
                            author: "<unknown>".to_string(),
                            description: "<unknown>".to_string(),
                            homepage: "<unknown>".to_string(),
                            license: "<unknown>".to_string(),
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
