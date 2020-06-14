pub mod determinekind;
mod parsers;

use semver::{Version, VersionReq};
use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;
use std::string::ToString;
use futures::future::try_join_all;

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
#[derive(Debug, Clone)]
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

pub fn stringify<T: ToString>(value: &Option<T>) -> String {
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
        let dep_names: Vec<String> = config.groups.keys().into_iter()
            .flat_map(|x| config.groups[x].keys())
            .map(|x| x.to_string())
            .collect();
        // let mut fetchers = vec![];
        // for item in dep_names {
        //     fetchers.push(parsers::fetch_dep_info(item.to_string(), kind));
        // }
        let fetchers = dep_names.clone().into_iter().map(|x| parsers::fetch_dep_info(x.to_string(), kind));
        let results = try_join_all(fetchers).await.unwrap_or(vec![]);
        let mut metadata = HashMap::new();

        if &results.len() == &dep_names.len() {
            let mut count = 0;
            for item in dep_names {
                metadata.insert(item, results[count].clone());
                count += 1;
            }
        }

        Project {
            config,
            lockfile,
            metadata
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

    pub fn is_versions_available(&self, name: &str) -> bool {
        if let Some(_) = &self.metadata.get(name) {
            true
        } else {
            false
        }
    }

    pub fn get_dep_versions(&self, name: &str) -> Option<Vec<String>> {
        if let Some(meta) = &self.metadata.get(name) {
            Some(meta.versions.clone().into_iter().map(|x| x.to_string()).collect())
        } else {
            None
        }
    }

    pub fn get_current_version(&self, name: &str) -> String {
        stringify(&self.lockfile.get(name))
    }
    pub fn get_semver_version(&self, name: &str) -> String {
        "nio".to_string()  // TODO
    }
    pub fn get_specified_version(&self, group: &str, name: &str) -> String {
        stringify(self.config.groups.get(group).unwrap().get(name).unwrap())
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
