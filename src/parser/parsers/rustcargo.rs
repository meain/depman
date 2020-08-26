use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use toml::Value;
use toml_edit::{value, Document};

use crate::{
    parser::{Config, DepInfo, DependencyGroup, Lockfile, SearchDep},
    render::InstallCandidate,
};

/// For lockfile
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct DepWithVersion {
    name: String,
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LockFile {
    package: Vec<DepWithVersion>,
}

/// For pulling versions
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
struct CargoResponse {
    #[serde(alias = "crate")]
    info: CargoResponseCrate,
    versions: Vec<CargoResponseVersion>,
}

/// For search
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

pub struct RustCargo;
impl RustCargo {
    pub fn is_this_it(folder: &str) -> bool {
        Path::new(&format!("{}/Cargo.toml", folder)).exists()
    }

    pub fn parse_config(folder: &str) -> Config {
        let path_string = format!("{}/Cargo.toml", folder);
        let text = fs::read_to_string(&path_string)
            .unwrap_or_else(|_| panic!("Unable to read {}", &path_string));
        let parsed: Value =
            toml::from_str(&text).unwrap_or_else(|_| panic!("Unable to parse {}", &path_string));

        let mut name = None;
        let mut version = None;
        if let Some(v) = &parsed.get("package") {
            if let Value::Table(pkg) = v {
                if let Some(vv) = pkg.get("name") {
                    if let Value::String(n) = vv {
                        name = Some(n.to_string());
                    }
                }
                if let Some(vv) = pkg.get("version") {
                    if let Value::String(vvv) = vv {
                        if let Ok(vvvv) = Version::parse(&vvv) {
                            version = Some(vvvv);
                        }
                    }
                }
            }
        }

        // TODO: Get all dep groups
        let mut groups: BTreeMap<String, DependencyGroup> = BTreeMap::new();
        if let Value::Table(conf) = parsed {
            for key in conf.keys() {
                if [
                    "dependencies".to_string(),
                    "dev-dependencies".to_string(),
                    "build-dependencies".to_string(),
                ]
                .contains(key)
                {
                    if let Value::Table(gr) = &conf[key] {
                        let mut group: BTreeMap<String, Option<VersionReq>> = BTreeMap::new();
                        for dep in gr.keys() {
                            let version_req = match &gr[dep] {
                                Value::String(v) => match VersionReq::parse(&v) {
                                    Ok(vr) => Some(vr),
                                    Err(_) => None,
                                },
                                Value::Table(t) => {
                                    if let Some(vs) = t.get("version") {
                                        match vs {
                                            Value::String(v) => VersionReq::parse(&v).ok(),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };
                            group.insert(dep.to_string(), version_req);
                        }
                        groups.insert(key.to_string(), group);
                    }
                }
            }

            if let Some(t) = &conf.get("target") {
                if let Value::Table(target) = t {
                    for g in target.keys() {
                        if let Some(ggg) = target.get(g).unwrap().get("dependencies") {
                            if let Value::Table(gg) = ggg {
                                let mut group: BTreeMap<String, Option<VersionReq>> =
                                    BTreeMap::new();
                                for dep in gg.keys() {
                                    let version_req = match &gg[dep] {
                                        Value::String(v) => match VersionReq::parse(&v) {
                                            Ok(vr) => Some(vr),
                                            Err(_) => None,
                                        },
                                        Value::Table(t) => {
                                            if let Some(vs) = t.get("version") {
                                                match vs {
                                                    Value::String(v) => VersionReq::parse(&v).ok(),
                                                    _ => None,
                                                }
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    };
                                    group.insert(dep.to_string(), version_req);
                                }
                                groups.insert(
                                    format!("target.{}.dependencies", g.to_string()),
                                    group,
                                );
                            }
                        }
                    }
                }
            }
        }

        Config {
            name,
            version,
            groups,
        }
    }

    pub fn parse_lockfile(folder: &str) -> Lockfile {
        let path_string = format!("{}/Cargo.lock", folder);
        let text = fs::read_to_string(&path_string)
            .unwrap_or_else(|_| panic!("Unable to read {}", &path_string));
        let parsed: LockFile =
            toml::from_str(&text).unwrap_or_else(|_| panic!("Unable to parse {}", &path_string));

        let mut packages = HashMap::new();
        for package in parsed.package {
            packages.insert(
                package.name.to_string(),
                Version::parse(&package.version).ok().unwrap(),
            );
        }
        packages
    }

    pub async fn fetch_dep_info(name: &str) -> Result<DepInfo, Box<dyn std::error::Error>> {
        let mut url = format!("https://crates.io/api/v1/crates/{}", name);
        if env::var("MEAIN_TEST_ENV").is_ok() {
            url = format!("http://localhost:8000/cargo/{}.json", name)
        }
        let resp: CargoResponse = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", "depman (github.com/meain/depman)")
            .send()
            .await?
            .json()
            .await?;

        let versions = resp
            .versions
            .into_iter()
            .map(|x| Version::parse(&x.num).unwrap())
            .collect();

        Ok(DepInfo {
            name: name.to_string(),
            author: None,
            homepage: resp.info.homepage,
            license: resp.info.license,
            description: resp.info.description,
            repository: Some(format!("https://crates.io/crates/{}", name)),
            versions,
        })
    }

    pub fn delete_dep(
        folder: &str,
        group: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_string = format!("{}/Cargo.toml", folder);
        let file_contents = std::fs::read_to_string(&path_string)?;
        let mut doc = file_contents
            .parse::<Document>()
            .expect("Invalid config file");
        doc[group][name] = toml_edit::Item::None;
        std::fs::write(&path_string, doc.to_string())?;
        Ok(())
    }

    pub fn install_dep(
        dep: InstallCandidate,
        folder: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_string = format!("{}/Cargo.toml", folder);
        let file_contents = std::fs::read_to_string(&path_string)?;
        let mut doc = file_contents.parse::<Document>()?;
        if doc[&dep.kind][&dep.name]["version"].is_none() {
            doc[&dep.kind][&dep.name] = value(dep.version);
        } else {
            doc[&dep.kind][&dep.name]["version"] = value(dep.version);
        }
        std::fs::write(&path_string, doc.to_string())?;
        Ok(())
    }

    pub async fn search_dep(term: &str) -> Result<Vec<SearchDep>, Box<dyn std::error::Error>> {
        let url = format!(
            "https://crates.io/api/v1/crates?page=1&per_page=20&q={}",
            term
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
