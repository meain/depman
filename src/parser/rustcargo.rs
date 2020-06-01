use crate::render::InstallCandidate;
use futures::future::try_join_all;
use humanesort::prelude::*;
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use toml::value::Table;
use toml::Value;
use toml_edit::{value, Document};

use async_trait::async_trait;

use super::{DepGroup, Parser};
use crate::parser::{Config, Dep, SearchDep};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct RutVersionObjectContent {
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum RustVersionObject {
    Simple(String),
    Object(RutVersionObjectContent),
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ConfigFilePackage {
    name: String,
}

// TODO: make this dynamic so that cfg specific deps can be added
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigFile {
    dependencies: Option<Table>,
    #[serde(alias = "dev-dependencies")]
    dev_dependencies: Option<Table>,
    #[serde(alias = "build-dependencies")]
    build_dependences: Option<Table>,
}
impl ConfigFile {
    fn from(root: &str) -> Option<ConfigFile> {
        let path_string = format!("{}/Cargo.toml", root);
        let text = std::fs::read_to_string(&path_string)
            .expect(&format!("Unable to read {}", &path_string));
        let p = toml::from_str(&text);
        match p {
            Ok(package_json) => Some(package_json),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct DepWithVersion {
    name: String,
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LockFile {
    package: Vec<DepWithVersion>,
}
impl LockFile {
    fn from(root: &str) -> Option<LockFile> {
        let path_string = format!("{}/Cargo.lock", root);
        let text = std::fs::read_to_string(&path_string)
            .expect(&format!("Unable to read {}", &path_string));
        let parsed = toml::from_str(&text);
        match parsed {
            Ok(package_json) => Some(package_json),
            _ => None,
        }
    }
    pub fn get_lockfile_version(&self, name: &str) -> Option<String> {
        for package in &self.package {
            if name == package.name {
                return Some(package.version.clone());
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CargoResponseCrate {
    name: String,
    description: Option<String>,
    license: Option<String>, // TODO: license is version specific
    homepage: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CargoResponseVersion {
    num: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CargoResponse {
    #[serde(alias = "crate")]
    info: CargoResponseCrate,
    versions: Vec<CargoResponseVersion>,
}
impl CargoResponse {
    pub fn get_versions_list(&self) -> Vec<Version> {
        let mut versions = vec![];
        for ver in &self.versions {
            if let Ok(v) = Version::parse(&ver.num) {
                versions.push(v)
            }
        }
        versions
    }
    pub fn inject_inportant_versions(&self, dep: &mut Dep) {
        let mut key_list: Vec<String> = Vec::new();
        for version in &self.versions {
            key_list.push(version.num.to_string());
        }
        key_list.humane_sort();

        let mut parsed_versions: Vec<Version> = Vec::new();
        let mut latest_semantic_version: Option<Version> = None;
        let mut latest_version: Option<Version> = None;
        for key in key_list {
            if let Ok(valid_version) = semver::Version::parse(&key) {
                parsed_versions.push(valid_version.clone());
                latest_version = Some(valid_version.clone());
                if let Some(spec) = &dep.specified_version {
                    if spec.matches(&valid_version) {
                        latest_semantic_version = Some(valid_version);
                    }
                }
            };
        }
        dep.available_versions = Some(parsed_versions);
        dep.latest_version = latest_version;
        dep.latest_semver_version = latest_semantic_version;
    }
}

// hack to get the kind for easier manipulation
struct CargoResponseWithKind {
    data: CargoResponse,
    kind: String,
}
async fn fetch_resp(dep: String, kind: String) -> Result<CargoResponseWithKind, Box<dyn Error>> {
    let mut url = format!("https://crates.io/api/v1/crates/{}", dep);
    if let Ok(_) = env::var("MEAIN_TEST_ENV") {
        url = format!("http://localhost:8000/cargo/{}.json", dep)
    }
    let resp: CargoResponse = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "depman (github.com/meain/depman)")
        .send()
        .await?
        .json()
        .await?;
    Ok(CargoResponseWithKind { data: resp, kind })
}

async fn fetch_dep_infos(config: &mut Config) -> Result<(), Box<dyn Error + 'static>> {
    let mut gets = vec![];
    for (kind, group) in config.dep_groups.iter() {
        for (name, dep) in group.iter() {
            // so that we do not refetch it on rerender
            if let None = dep.latest_version {
                gets.push(fetch_resp(name.to_string(), kind.to_string()));
            }
        }
    }

    let results = try_join_all(gets).await?;
    for result in &results {
        let mut dep = &mut config
            .dep_groups
            .get_mut(&result.kind)
            .unwrap()
            .get_mut(&result.data.info.name)
            .unwrap();
        dep.description = result.data.info.description.clone();
        dep.available_versions = Some(result.data.get_versions_list());
        dep.license = result.data.info.license.clone();
        result.data.inject_inportant_versions(dep);
    }
    Ok(())
}

fn toml_tabl_to_string(item: &Value) -> String {
    // TODO: make this better
    match item.as_str() {
        Some(s) => s.to_string(),
        None => match item.as_table() {
            Some(t) => {
                let tv = &t["version"];
                match tv.as_str() {
                    Some(s) => s.to_string(),
                    None => "<invalid>".to_string(),
                }
            }
            None => "<invalid>".to_string(),
        },
    }
}

// TODO: probably add description and other stuff we have for normal deps
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CratesIOSearchCreate {
    name: String,
    newest_version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CratesIOSearchResp {
    crates: Vec<CratesIOSearchCreate>,
}

fn build_dep_entry(
    name: &str,
    kind: &str,
    specified_version: &str,
    current_version: Option<String>,
) -> Dep {
    let current_version_parsed = match current_version {
        Some(cv) => Version::parse(&cv).ok(),
        None => None,
    };
    Dep {
        name: name.to_string(),
        kind: kind.to_string(),
        author: None,
        description: None,
        homepage: None,
        package_repo: format!("https://crates.io/crates/{}", name),
        license: None,
        specified_version: VersionReq::parse(specified_version).ok(), // from config files
        current_version: current_version_parsed,                      // parsed from lockfiles
        available_versions: None,
        latest_version: None,
        latest_semver_version: None,
    }
}

fn build_deps_array(kind: &str, deps: Table, lockfile: &LockFile) -> DepGroup {
    let mut dep_group: DepGroup = BTreeMap::new();
    deps.keys().into_iter().for_each(|x| {
        dep_group.insert(
            x.to_string(),
            build_dep_entry(
                x,
                &kind,
                &toml_tabl_to_string(&deps[x]),
                lockfile.get_lockfile_version(x),
            ),
        );
    });
    dep_group
}

pub struct RustCargo;

#[async_trait]
impl Parser for RustCargo {
    async fn parse(root: &str) -> Config {
        let configfile = ConfigFile::from(root).expect(&format!("Unable to parse Cargo.toml"));
        let lockfile = LockFile::from(root).expect(&format!("Unable to parse Cargo.lock"));
        let mut dep_groups = BTreeMap::new();
        if let Some(deps) = configfile.dependencies {
            dep_groups.insert(
                "dependencies".to_string(),
                build_deps_array("dependencies", deps, &lockfile),
            );
        }
        if let Some(deps) = configfile.dev_dependencies {
            dep_groups.insert(
                "dev-dependencies".to_string(),
                build_deps_array("dev-dependencies", deps, &lockfile),
            );
        }
        if let Some(deps) = configfile.build_dependences {
            dep_groups.insert(
                "build-dependencies".to_string(),
                build_deps_array("build-dependencies", deps, &lockfile),
            );
        }

        let mut config = Config { dep_groups };
        let _ = fetch_dep_infos(&mut config).await; // ignore error
        config
    }

    fn delete_dep(dep: Dep, root: &str) {
        let path_string = format!("{}/Cargo.toml", root);
        let file_contents = std::fs::read_to_string(&path_string).unwrap();
        let mut doc = file_contents.parse::<Document>().expect("Invalid config file");
        doc[&dep.kind][&dep.name] = toml_edit::Item::None;
        std::fs::write(&path_string, doc.to_string()).unwrap();
    }

    fn install_dep(dep: InstallCandidate, root: &str) {
        let path_string = format!("{}/Cargo.toml", root);
        let file_contents = std::fs::read_to_string(&path_string).unwrap();
        let mut doc = file_contents.parse::<Document>().expect("Invalid config file");
        if doc[&dep.kind][&dep.name]["version"].is_none() {
            doc[&dep.kind][&dep.name] = value(dep.version);
        } else {
            doc[&dep.kind][&dep.name]["version"] = value(dep.version);
        }
        std::fs::write(&path_string, doc.to_string()).unwrap();
    }

    async fn search_deps(name: &str) -> Result<Vec<SearchDep>, Box<dyn Error>> {
        let url = format!(
            "https://crates.io/api/v1/crates?page=1&per_page=20&q={}",
            name
        );
        let resp: CratesIOSearchResp = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", "depman (github.com/meain/depman)")
            .send()
            .await?
            .json()
            .await?;
        Ok(resp
            .crates
            .into_iter()
            .map(|x| SearchDep {
                name: x.name,
                version: x.newest_version,
            })
            .collect())
    }
}
