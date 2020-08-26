mod parsers;

use futures::{future::try_join_all, stream, StreamExt};
use semver::{Version, VersionReq};
use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;
use std::string::ToString;

use crate::{events::TabItem, render::InstallCandidate};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub enum ParserKind {
    JavascriptNpm,
    RustCargo,
}

pub enum UpgradeType {
    None,
    Patch,
    Minor,
    Major,
    Breaking,
}

#[derive(Debug, Clone)]
pub struct SearchDep {
    pub name: String,
    pub version: String,
    // TODO: add in more items like homepage, repo, author etc
}

type DependencyGroup = BTreeMap<String, Option<VersionReq>>;
#[derive(Clone)]
pub struct Config {
    pub name: Option<String>,
    pub version: Option<Version>,
    pub groups: BTreeMap<String, DependencyGroup>,
}
#[derive(Debug, Clone)]
pub struct DepInfo {
    name: String,
    author: Option<Author>,
    homepage: Option<String>,
    repository: Option<String>, // package repo
    license: Option<String>,
    description: Option<String>,
    versions: Vec<Version>,
}
type Lockfile = HashMap<String, Version>;
type MetaData = HashMap<String, DepInfo>;
#[derive(Clone)]
pub struct Project {
    config: Config,
    lockfile: Lockfile,
    metadata: MetaData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Author {
    name: String,
    url: Option<String>,
    email: Option<String>,
}
impl ToString for Author {
    fn to_string(&self) -> String {
        let mut author_string = self.name.to_string();
        if let Some(v) = &self.email {
            author_string = format!("{} <{}>", author_string, &v.to_string());
        }
        if let Some(v) = &self.url {
            author_string = format!("{} [{}]", author_string, &v.to_string());
        }
        author_string
    }
}

pub fn stringify<T: ToString>(value: &Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    }
}

// Mostly for derived values
impl Project {
    pub fn determine_kind(folder: &str) -> Option<ParserKind> {
        if parsers::is_this_it(folder, &ParserKind::JavascriptNpm) {
            Some(ParserKind::JavascriptNpm)
        } else if parsers::is_this_it(folder, &ParserKind::RustCargo) {
            Some(ParserKind::RustCargo)
        } else {
            None
        }
    }
    pub async fn parse(folder: &str, kind: &ParserKind) -> Project {
        let config = parsers::parse_config(folder, kind);
        let lockfile = parsers::parse_lockfile(folder, kind);
        let dep_names: Vec<String> = config
            .groups
            .keys()
            .flat_map(|x| config.groups[x].keys())
            .map(|x| x.to_string())
            .collect();

        let fetchers = dep_names
            .clone()
            .into_iter()
            .map(|x| parsers::fetch_dep_info(x, kind))
            .collect::<Vec<_>>();

        let mut metadata = HashMap::new();
        let mut st = stream::iter(fetchers).buffer_unordered(5);
        while let Some(chunk) = st.next().await {
            if let Ok(mut item) = chunk {
                item.versions.sort();
                item.versions = item.versions.into_iter().rev().collect();
                metadata.insert(item.name.to_string(), item);
            }
        }

        Project {
            config,
            lockfile,
            metadata,
        }
    }

    pub async fn reparse(&self, folder: &str, kind: &ParserKind) -> Project {
        let config = parsers::parse_config(folder, kind);
        let lockfile = parsers::parse_lockfile(folder, kind);

        let dep_names: Vec<String> = config
            .groups
            .keys()
            .flat_map(|x| config.groups[x].keys())
            .map(|x| x.to_string())
            .filter(|x| !self.metadata.keys().any(|e| e == x))
            .collect();

        let fetchers = dep_names
            .clone()
            .into_iter()
            .map(|x| parsers::fetch_dep_info(x, kind));
        let results = try_join_all(fetchers).await.unwrap_or_default();
        let mut metadata = HashMap::new();
        if results.len() == dep_names.len() {
            for (count, item) in dep_names.into_iter().enumerate() {
                let mut api_data = results[count].clone();
                api_data.versions.sort();
                api_data.versions = api_data.versions.into_iter().rev().collect();
                metadata.insert(item, api_data);
            }
        }
        metadata.extend(self.metadata.clone());
        Project {
            config,
            lockfile,
            metadata,
        }
    }

    // pub async fn search_deps(kind: &ParserKind, query: &str) {}

    pub fn get_groups(&self) -> Vec<TabItem> {
        let mut groups = vec![];
        for key in self.config.groups.keys() {
            let item = TabItem {
                value: key.to_string(),
                label: format!(
                    "{}({})",
                    key.to_string(),
                    self.config.groups[key].keys().len()
                ),
            };
            groups.push(item)
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
        self.metadata.get(name).is_some()
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
                    let current_pos = vers.iter().position(|r| r == cv);
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
        let author = &self.metadata.get(name)?.author;
        match author {
            Some(a) => Some(a.to_string()),
            None => None,
        }
    }
    pub fn get_homepage(&self, name: &str) -> Option<String> {
        self.metadata.get(name)?.homepage.clone()
    }
    pub fn get_repository(&self, name: &str) -> Option<String> {
        self.metadata.get(name)?.repository.clone()
    }
    pub fn get_license(&self, name: &str) -> Option<String> {
        self.metadata.get(name)?.license.clone()
    }
    pub fn get_description(&self, name: &str) -> Option<String> {
        self.metadata.get(name)?.description.clone()
    }

    pub fn delete_dep(&self, kind: &ParserKind, folder: &str, group: &str, name: &str) -> bool {
        parsers::delete_dep(kind, folder, group, name)
    }

    pub fn install_dep(&self, kind: &ParserKind, folder: &str, dep: InstallCandidate) -> bool {
        parsers::install_dep(kind, folder, dep)
    }

    pub async fn search_dep(&self, kind: &ParserKind, term: &str) -> Option<Vec<SearchDep>> {
        parsers::search_dep(kind, term).await.ok()
    }
}
