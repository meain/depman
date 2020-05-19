use crate::render::InstallCandidate;
use std::env;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::error::Error;
use humanesort::prelude::*;
use futures::future::try_join_all;

use serde::{Deserialize, Serialize};
use crate::parser::{Author, Dep, DepList, DepVersion, DepVersionReq, DepListList};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavascriptPackageJson {
    name: String,
    dependencies: Option<HashMap<String, String>>,
    devDependencies: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DepWithVersion {
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavascriptPackageJsonLockfile {
    name: String,
    dependencies: Option<HashMap<String, DepWithVersion>>,
}

impl JavascriptPackageJsonLockfile {
    fn from(folder: &str) -> JavascriptPackageJsonLockfile {
        let path_string = format!("{}/package-lock.json", folder);
        let path = Path::new(&path_string);
        let file_maybe = File::open(path);
        match file_maybe {
            Ok(file) => {
                let reader = BufReader::new(file);
                let p = serde_json::from_reader(reader);
                match p {
                    Ok(package_json) => {
                        return package_json;
                    }
                    _ => panic!("Cannot parse package-lock.json"),
                }
            }
            _ => panic!("Cannot read package.json"),
        }
    }
    pub fn get_lockfile_version(&self, name: &str) -> Option<String> {
        match &self.dependencies {
            Some(deps) => Some(deps[name].version.clone()),
            None => None,
        }
    }
}

impl JavascriptPackageJson {
    fn from(folder: &str) -> JavascriptPackageJson {
        let path_string = format!("{}/package.json", folder);
        let path = Path::new(&path_string);
        let file_maybe = File::open(path);
        match file_maybe {
            Ok(file) => {
                let reader = BufReader::new(file);
                let p = serde_json::from_reader(reader);
                match p {
                    Ok(package_json) => {
                        return package_json;
                    }
                    _ => panic!("Cannot read package.json"),
                }
            }
            _ => panic!("Cannot read package.json"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MockVersionRight {
    version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmResponse {
    name: String,
    author: Option<Author>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    versions: HashMap<String, MockVersionRight>, // TODO: remove this Value from here
}

impl NpmResponse {
    pub fn get_versions_list(&self) -> Vec<DepVersion> {
        let mut versions = vec![];
        for key in self.versions.keys() {
            versions.push(DepVersion::from(Some(key.clone())))
        }
        versions
    }

    pub fn inject_inportant_versions(&self, dep: &mut Dep) {
        let mut key_list: Vec<String> = Vec::new();
        for key in self.versions.keys() {
            key_list.push(key.to_string());
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


async fn fetch_resp(dep: &str) -> Result<NpmResponse, Box<dyn Error>> {
    let mut url = format!("https://registry.npmjs.org/{}", dep);
    match env::var("MEAIN_TEST_ENV") {
        Ok(_) => url = format!("http://localhost:8000/npm/{}.json", dep),
        _ => {}
    }
    let resp = reqwest::get(&url).await?.json().await?;
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
                if &result.name == &dep.name {
                    dep.description = result.description.clone();
                    dep.available_versions = Some(result.get_versions_list());
                    dep.license = result.license.clone();
                    dep.homepage = result.homepage.clone();
                    dep.author = result.author.clone();
                    result.inject_inportant_versions(&mut dep);
                }
            }
        }
    }

    Ok(())
}

pub async fn into(folder: &str) -> DepListList {
    let config = JavascriptPackageJson::from(folder);
    let lockfile = JavascriptPackageJsonLockfile::from(folder);
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
                package_repo: format!("https://www.npmjs.com/package/{}", dep.to_string()),
                license: None,
                specified_version: DepVersionReq::from(&deps[dep]), // from config files
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
                package_repo: format!("https://www.npmjs.com/package/{}", dep.to_string()),
                license: None,
                specified_version: DepVersionReq::from(&deps[dep]), // from config files
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

    let mut dep_list_list = DepListList { lists: items };
    fetch_dep_infos(&mut dep_list_list).await;  // Error does not matter, there is nothing I can do
    dep_list_list
}

pub fn install_dep(dep: InstallCandidate){
    todo!()
}
