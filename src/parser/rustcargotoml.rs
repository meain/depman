use crate::render::InstallCandidate;
use std::env;
use std::error::Error;
use humanesort::prelude::*;
use futures::future::try_join_all;
use toml::Value;
use toml::value::Table;
use toml_edit::{Document, value};

use serde::{Deserialize, Serialize};
use crate::parser::{Dep, DepList, DepVersion, DepVersionReq, DepListList, SearchDep};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct RutVersionObjectContent{
    version: String
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum RustVersionObject {
    Simple(String),
    Object(RutVersionObjectContent),
}

impl RustVersionObject {
    pub fn _to_string(&self) -> String {
        match self {
            RustVersionObject::Simple(s) => s.to_string(),
            RustVersionObject::Object(o) => o.version.to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ConfigFilePackage {
    name: String
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ConfigFile {
    package: ConfigFilePackage,
    dependencies: Option<Table>,
    #[serde(alias = "dev-dependencies")]
    devDependencies: Option<Table>,
    #[serde(alias = "build-dependencies")]
    buildDependencies: Option<Table>,
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
    fn from(folder: &str) -> LockFile {
        let path_string = format!("{}/Cargo.lock", folder);
        let text_maybe = std::fs::read_to_string(path_string);
        match text_maybe {
            Ok(text) => {
                let p = toml::from_str(&text);
                match p {
                    Ok(package_json) => {
                        return package_json;
                    }
                    _ => panic!("Cannot parse Cargo.lock"),
                }
            }
            _ => panic!("Cannot read Cargo.lock"),
        }
    }
    pub fn get_lockfile_version(&self, name: &str) -> Option<String> {
        for package in &self.package {
            if name == package.name{
                return Some(package.version.clone());
            }
        }
        None
    }
}

impl ConfigFile {
    fn from(folder: &str) -> ConfigFile {
        let path_string = format!("{}/Cargo.toml", folder);
        let text_maybe = std::fs::read_to_string(path_string);
        match text_maybe {
            Ok(text) => {
                let p = toml::from_str(&text);
                match p {
                    Ok(package_json) => {
                        return package_json;
                    }
                    _ => panic!("Cannot parse Cargo.toml"),
                }
            }
            _ => panic!("Cannot read Cargo.toml"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CargoResponseCrate {
    name: String,
    description: Option<String>,
    license: Option<String>,  // TODO: license is version specific
    homepage: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CargoResponseVersion {
    num: String
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
    match env::var("MEAIN_TEST_ENV") {
        Ok(_) => url = format!("http://localhost:8000/cargo/{}.json", dep),
        _ => {}
    }
    let resp = reqwest::Client::new().get(&url)
        .header("User-Agent", "depman (github.com/meain/depman)").send().await?.json().await?;
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

    for dep_list in &mut dep_list_list.lists {
        for mut dep in &mut dep_list.deps {
            for result in &results {
                if &result.info.name == &dep.name {
                    dep.description = result.info.description.clone();
                    dep.available_versions = Some(result.get_versions_list());
                    dep.license = result.info.license.clone();
                    dep.homepage = result.info.homepage.clone();
                    // dep.author = result.info.author.clone(); // author does not exist
                    result.inject_inportant_versions(&mut dep);
                }
            }
        }
    }

    Ok(())
}

fn toml_tabl_to_string(item: &Value) -> String {
    // TODO: make this better
    match item.as_str(){
        Some(s) => s.to_string(),
        None => {
            match item.as_table() {
                Some(t) => {
                    let tv = &t["version"];
                    match tv.as_str(){
                        Some(s) => s.to_string(),
                        None => "<invalid>".to_string()
                    }
                },
                None => "<invalid>".to_string()
            }
        }
    }
}

pub async fn into(folder: &str) -> DepListList {
    let config = ConfigFile::from(folder);
    let lockfile = LockFile::from(folder);
    let mut items = vec![];
    if let Some(deps) = config.dependencies {
        let mut dep_list = vec![];
        for dep in deps.keys() {
            let dep_item = Dep {
                name: dep.to_string(),
                kind: "dependencies".to_string(),
                author: None,
                description: None,
                homepage: None,
                package_repo: format!("https://crates.io/crates/{}", dep.to_string()),
                license: None,
                specified_version: DepVersionReq::from(&toml_tabl_to_string(&deps[dep])), // from config files
                current_version: DepVersion::from(lockfile.get_lockfile_version(dep)), // parsed from lockfiles
                available_versions: None,
                latest_version: None,
                latest_semver_version: None,
            };
            dep_list.push(dep_item);
        }
        items.push(DepList {
            name: "dependencies".to_string(),
            deps: dep_list,
        })
    }
    if let Some(deps) = config.devDependencies {
        let mut dep_list = vec![];
        for dep in deps.keys() {
            let dep_item = Dep {
                name: dep.to_string(),
                kind: "devDependencies".to_string(),
                author: None,
                description: None,
                homepage: None,
                package_repo: format!("https://crates.io/crates/{}", dep.to_string()),
                license: None,
                specified_version: DepVersionReq::from(&deps[dep].to_string()), // from config files
                current_version: DepVersion::from(lockfile.get_lockfile_version(dep)), // parsed from lockfiles
                available_versions: None,
                latest_version: None,
                latest_semver_version: None,
            };
            dep_list.push(dep_item);
        }
        items.push(DepList {
            name: "devDependencies".to_string(),
            deps: dep_list,
        })
    }

    if let Some(deps) = config.buildDependencies {
        let mut dep_list = vec![];
        for dep in deps.keys() {
            let dep_item = Dep {
                name: dep.to_string(),
                kind: "buildDependencies".to_string(),
                author: None,
                description: None,
                homepage: None,
                package_repo: format!("https://crates.io/crates/{}", dep.to_string()),
                license: None,
                specified_version: DepVersionReq::from(&deps[dep].to_string()), // from config files
                current_version: DepVersion::from(lockfile.get_lockfile_version(dep)), // parsed from lockfiles
                available_versions: None,
                latest_version: None,
                latest_semver_version: None,
            };
            dep_list.push(dep_item);
        }
        items.push(DepList {
            name: "buildDependencies".to_string(),
            deps: dep_list,
        })
    }

    let mut dep_list_list = DepListList { lists: items };
    fetch_dep_infos(&mut dep_list_list).await;  // Error does not matter, there is nothing I can do
    dep_list_list
}

fn convert_dep_type_string(kind: &str) -> &str {
    match kind {
        "devDependencies" => "dev-dependencies",
        "buildDependencies" => "build-dependencies",
        _ => "dependencies"
    }
}

pub fn install_dep(dep: InstallCandidate, folder: &str){
    let path_string = format!("{}/Cargo.toml", folder);
    let file_contents = std::fs::read_to_string(&path_string).unwrap();
    let mut doc = file_contents.parse::<Document>().expect("invalid doc");
    if doc[convert_dep_type_string(&dep.kind)][&dep.name]["version"].is_none() {
        doc[convert_dep_type_string(&dep.kind)][&dep.name] = value(dep.version);
    } else {
        doc[convert_dep_type_string(&dep.kind)][&dep.name]["version"] = value(dep.version);
    }
    std::fs::write(&path_string, doc.to_string()).unwrap();
}

// TODO: probably add description and other stuff we have for normal deps
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CratesIOSearchCreate {
    name: String,
    newest_version: String
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CratesIOSearchResp {
    crates: Vec<CratesIOSearchCreate>
}

pub async fn search_deps(name: &str) -> Result<Vec<SearchDep>, Box<dyn Error>> {
    let url = format!("https://crates.io/api/v1/crates?page=1&per_page=10&q={}", name);
    let resp: CratesIOSearchResp = reqwest::Client::new().get(&url)
        .header("User-Agent", "depman (github.com/meain/depman)").send().await?.json().await?;
    let mut deps: Vec<SearchDep> = vec![];
    for dep in resp.crates {
        deps.push(SearchDep{name: dep.name, version: dep.newest_version});
    }
    Ok(deps)
}
