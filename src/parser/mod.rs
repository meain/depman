pub mod determinekind;
mod parsers;

use futures::future::try_join_all;
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
        let dep_names: Vec<String> = config
            .groups
            .keys()
            .into_iter()
            .flat_map(|x| config.groups[x].keys())
            .map(|x| x.to_string())
            .collect();

        let fetchers = dep_names
            .clone()
            .into_iter()
            .map(|x| parsers::fetch_dep_info(x.to_string(), kind));
        let results = try_join_all(fetchers).await.unwrap_or(vec![]);
        let mut metadata = HashMap::new();
        if &results.len() == &dep_names.len() {
            let mut count = 0;
            for item in dep_names {
                let mut api_data = results[count].clone();
                api_data.versions.sort();
                api_data.versions = api_data.versions.into_iter().rev().collect();
                metadata.insert(item, api_data);
                count += 1;
            }
        }

        Project {
            config,
            lockfile,
            metadata,
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

    pub fn get_dep_versions(&self, name: &str) -> Option<&Vec<Version>> {
        if let Some(meta) = &self.metadata.get(name) {
            Some(&meta.versions)
        // Some(meta.versions.clone().into_iter().map(|x| x.to_string()).collect())
        } else {
            None
        }
    }

    pub fn get_current_version(&self, name: &str) -> Option<&Version> {
        self.lockfile.get(name)
    }
    pub fn get_semver_version(&self, group: &str, name: &str) -> Option<&Version> {
        let current_version = self.get_current_version(&name);
        let specified_version = self.get_specified_version(&group, &name);
        let versions = self.get_dep_versions(&name);
        if let Some(sv) = specified_version {
            if let Some(vers) = versions {
                if let Some(cv) = current_version {
                    let current_pos = vers.iter().position(|r| &r == &cv);
                    if let Some(cp) = current_pos {
                        let mut last = cv;
                        for i in (0..cp).rev() {
                            if sv.matches(&vers[i]) {
                                last = &vers[i];
                            } else {
                                break;
                            }
                        }
                        return Some(last);
                    }
                }
            }
        }
        None
    }
    pub fn get_specified_version(&self, group: &str, name: &str) -> Option<&VersionReq> {
        self.config
            .groups
            .get(group)
            .unwrap()
            .get(name)
            .unwrap()
            .as_ref()
    }
    pub fn get_latest_version(&self, name: &str) -> Option<&Version> {
        let versions = self.get_dep_versions(&name);
        match versions {
            Some(v) => Some(&v[0]),
            None => None,
        }
    }

    pub fn get_upgrade_type(&self, group: &str, name: &str) -> UpgradeType {
        let current_version = self.get_current_version(&name);
        let specified_version = self.get_specified_version(&group, &name);
        let semver_version = self.get_semver_version(&group, &name);
        let latest_version = self.get_latest_version(&name);
        if let Some(cv) = &current_version {
            if let Some(sv) = &semver_version {
                if let Some(lv) = &latest_version {
                    if cv.major < sv.major {
                        return UpgradeType::Major;
                    } else if cv.minor < sv.minor {
                        return UpgradeType::Minor;
                    } else if cv.patch < sv.patch {
                        return UpgradeType::Patch;
                    } else if lv > cv {
                        return UpgradeType::Breaking;
                    }
                }
            }
        }
        UpgradeType::None
    }

    pub fn get_author(&self, name: &str) -> Option<String> {
        self.metadata.get(name).unwrap().author.clone()
    }
    pub fn get_homepage(&self, name: &str) -> Option<String> {
        self.metadata.get(name).unwrap().homepage.clone()
    }
    pub fn get_repository(&self, name: &str) -> Option<String> {
        self.metadata.get(name).unwrap().repository.clone()
    }
    pub fn get_license(&self, name: &str) -> Option<String> {
        self.metadata.get(name).unwrap().license.clone()
    }
    pub fn get_description(&self, name: &str) -> Option<String> {
        self.metadata.get(name).unwrap().description.clone()
    }
}
