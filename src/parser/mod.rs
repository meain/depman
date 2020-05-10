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

#[derive(Debug, Clone)]
pub enum DepVersionReq {
    Error,
    Version(semver::VersionReq),
    // might have to add stuff like guthub repo or file here
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub specified_version: DepVersionReq, // from config files
    pub current_version: DepVersion,      // parsed from lockfiles
    pub available_versions: Option<Vec<DepVersion>>,
    pub latest_version: Option<DepVersion>,
    pub latest_semver_version: Option<DepVersion>,
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
