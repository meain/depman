pub mod determinekind;
mod parsers;

use semver::{Version, VersionReq};
use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;
use std::string::ToString;

use determinekind::ParserKind;



pub enum UpgradeType {
    None,
    Patch,
    Minor,
    Major,
    Breaking,
}

type DependencyGroup = BTreeMap<String, Option<VersionReq>>;
pub struct Config {
    pub name: Option<String>,
    pub version: Option<Version>,
    pub groups: BTreeMap<String, DependencyGroup>,
}
struct DepInfo {
    author: Option<String>,
    homepage: Option<String>,
    repository: Option<String>, // package repo
    license: Option<String>,
    description: Option<String>,
    versions: Vec<Version>,
}
type Lockfile = HashMap<String, Version>;
type MetaData = HashMap<String, DepInfo>;
pub struct Project {
    config: Config,
    lockfile: Lockfile,
    metadata: MetaData,
}

pub fn stringify<T: ToString>(value: Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    }
}

// Mostly for derived values
impl Project {
    pub async fn parse(folder: &str, kind: &ParserKind) -> Project {
        let config = parsers::parse_config(folder, kind);
        let lockfile = parsers::parse_lockfile(folder, kind);
        Project {
            config,
            lockfile,
            metadata: HashMap::new()
        }
    }
    // pub async fn search_deps(kind: &ParserKind, query: &str) {}
    pub fn get_groups(&self) -> Vec<String> {
        let mut groups = vec![];
        for key in self.config.groups.keys() {
            groups.push(key.to_string())
        }
        groups
    }

    pub fn get_deps_in_group(&self, group: &str) -> Vec<String> {
        let mut deps = vec![];
        if let Some(dps) = &self.config.groups.get(group) {
            for key in dps.keys() {
                deps.push(key.to_string());
            }
        }
        deps
    }

    pub fn get_dep_versions(&self, name: &str) -> Option<Vec<String>> {
        None
        // if let Some(dpg) = &self.config.groups.get(group) {
        //     if let Some(dep) = dpg.get(name) {
        //         return Some(dep);
        //     }
        // }
        // None
    }

    pub fn get_current_version(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }
    pub fn get_semver_version(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }
    pub fn get_specified_version(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }
    pub fn get_latest_version(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }

    pub fn get_author(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }

    pub fn get_upgrade_type(&self, group: &str, name: &str) -> UpgradeType {
        UpgradeType::None
    }
}
