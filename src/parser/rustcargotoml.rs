use crate::render::InstallCandidate;
use futures::future::try_join_all;
use humanesort::prelude::*;
use std::env;
use std::error::Error;
use toml::value::Table;
use toml::Value;
use toml_edit::{value, Document};

use async_trait::async_trait;

use super::Parser;
use crate::parser::{Dep, DepList, DepListList, DepVersion, DepVersionReq, SearchDep};
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigFile {
    package: ConfigFilePackage,
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
    pub fn get_versions_list(&self) -> Vec<DepVersion> {
        let mut versions = vec![];
        for ver in &self.versions {
            versions.push(DepVersion::from(Some(ver.num.clone())))
        }
        versions
    }
    pub fn inject_inportant_versions(&self, dep: &mut Dep) {
        let mut key_list: Vec<String> = Vec::new();
        for version in &self.versions {
            key_list.push(version.num.to_string());
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
                        latest_semantic_version = Some(DepVersion::Version(valid_version.clone()));
                    }
                }
            };
        }
        dep.available_versions = Some(parsed_versions);
        dep.latest_version = latest_version;
        dep.latest_semver_version = latest_semantic_version;
    }
}

async fn fetch_resp(dep: &str) -> Result<CargoResponse, Box<dyn Error>> {
    let mut url = format!("https://crates.io/api/v1/crates/{}", dep);
    if let Ok(_) = env::var("MEAIN_TEST_ENV") {
        url = format!("http://localhost:8000/cargo/{}.json", dep)
    }
    let resp = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "depman (github.com/meain/depman)")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

async fn fetch_dep_infos(dep_list_list: &mut DepListList) -> Result<(), Box<dyn Error + 'static>> {
    let mut gets = vec![];
    for dep_list in &dep_list_list.lists {
        gets.extend(dep_list.deps.iter().map(|x| fetch_resp(&x.name)));
    }
    let results = try_join_all(gets).await?;

    for dep_list in &mut dep_list_list.lists {
        for mut dep in &mut dep_list.deps {
            for result in &results {
                if &result.info.name == &dep.name {
                    dep.description = result.info.description.clone();
                    dep.available_versions = Some(result.get_versions_list());
                    dep.license = result.info.license.clone();
                    dep.homepage = result.info.homepage.clone();
                    result.inject_inportant_versions(&mut dep);
                }
            }
        }
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

fn convert_dep_type_string(kind: &str) -> &str {
    match kind {
        "devDependencies" => "dev-dependencies",
        "buildDependencies" => "build-dependencies",
        _ => "dependencies",
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
    Dep {
        name: name.to_string(),
        kind: kind.to_string(),
        author: None,
        description: None,
        homepage: None,
        package_repo: format!("https://crates.io/crates/{}", name),
        license: None,
        specified_version: DepVersionReq::from(specified_version), // from config files
        current_version: DepVersion::from(current_version),        // parsed from lockfiles
        available_versions: None,
        latest_version: None,
        latest_semver_version: None,
    }
}

fn build_deps_array(deps: Table, lockfile: &LockFile, kind: &str) -> DepList {
    let dep_list = deps
        .keys()
        .into_iter()
        .map(|x| {
            build_dep_entry(
                x,
                kind,
                &toml_tabl_to_string(&deps[x]),
                lockfile.get_lockfile_version(x),
            )
        })
        .collect();
    DepList {
        name: kind.to_string(),
        deps: dep_list,
    }
}

pub struct RustCargo;

#[async_trait]
impl Parser for RustCargo {
    async fn parse(root: &str) -> DepListList {
        let config = ConfigFile::from(root).expect(&format!("Unable to parse Cargo.toml"));
        let lockfile = LockFile::from(root).expect(&format!("Unable to parse Cargo.lock"));
        let mut items = vec![];
        if let Some(deps) = config.dependencies {
            items.push(build_deps_array(deps, &lockfile, "dependencies"))
        }
        if let Some(deps) = config.dev_dependencies {
            items.push(build_deps_array(deps, &lockfile, "devDependencies"))
        }
        if let Some(deps) = config.build_dependences {
            items.push(build_deps_array(deps, &lockfile, "buildDependencies"))
        }

        let mut dep_list_list = DepListList { lists: items };
        let _ = fetch_dep_infos(&mut dep_list_list).await; // ignore error
        dep_list_list
    }

    fn install_dep(dep: InstallCandidate, root: &str) {
        let path_string = format!("{}/Cargo.toml", root);
        let file_contents = std::fs::read_to_string(&path_string).unwrap();
        let mut doc = file_contents.parse::<Document>().expect("invalid doc");
        if doc[convert_dep_type_string(&dep.kind)][&dep.name]["version"].is_none() {
            doc[convert_dep_type_string(&dep.kind)][&dep.name] = value(dep.version);
        } else {
            doc[convert_dep_type_string(&dep.kind)][&dep.name]["version"] = value(dep.version);
        }
        std::fs::write(&path_string, doc.to_string()).unwrap();
    }

    async fn search_deps(name: &str) -> Result<Vec<SearchDep>, Box<dyn Error>> {
        let url = format!(
            "https://crates.io/api/v1/crates?page=1&per_page=10&q={}",
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
