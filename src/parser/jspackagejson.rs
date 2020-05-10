use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};
use crate::parser::{Dep, DepList, DepVersion, DepVersionReq, DepListList};

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
        println!("path_string: {:?}", path_string);
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

pub fn into(folder: &str) -> DepListList {
    let config = JavascriptPackageJson::from(folder);
    let lockfile = JavascriptPackageJsonLockfile::from(folder);
    let mut items = vec![];
    if let Some(deps) = config.dependencies {
        let mut dep_list = vec![];
        for dep in deps.keys() {
            let dep_item = Dep {
                name: dep.to_string(),
                author: None,
                description: None,
                homepage: None,
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
                author: None,
                description: None,
                homepage: None,
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
    DepListList { lists: items }
}
