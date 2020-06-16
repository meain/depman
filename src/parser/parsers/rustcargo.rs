use std::fs;
use std::env;
use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;

use toml::Value;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use toml_edit::Document;

use crate::parser::{Config, DependencyGroup, Lockfile, DepInfo};


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

pub struct RustCargo;
impl RustCargo {
    pub fn parse_config(folder: &str) -> Config {
        let path_string = format!("{}/Cargo.toml", folder);
        let text =
            fs::read_to_string(&path_string).expect(&format!("Unable to read {}", &path_string));
        let parsed: Value =
            toml::from_str(&text).expect(&format!("Unable to parse {}", &path_string));

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
                if key == "dependencies" {
                    if let Value::Table(gr) = &conf["dependencies"] {
                        let mut group: BTreeMap<String, Option<VersionReq>> = BTreeMap::new();
                        for dep in gr.keys() {
                            let version_req = match &gr[dep] {
                                Value::String(v) => match VersionReq::parse(&v) {
                                    Ok(vr) => Some(vr),
                                    Err(_) => None,
                                },
                                _ => None,
                            };
                            group.insert(dep.to_string(), version_req);
                        }
                        groups.insert("dependencies".to_string(), group);
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
        let text =
            fs::read_to_string(&path_string).expect(&format!("Unable to read {}", &path_string));
        let parsed: Value =
            toml::from_str(&text).expect(&format!("Unable to parse {}", &path_string));

        let mut packages = HashMap::new();
        if let Value::Table(conf) = parsed {
            if let Some(pgs) = conf.get("package") {
                if let Value::Array(p) = pgs {
                    for item in p {
                        if let Value::Table(it) = item {
                            if let Value::String(name) = it.get("name").unwrap() {
                                if let Value::String(ver) = it.get("version").unwrap() {
                                    packages.insert(name.to_string(), Version::parse(ver).unwrap());
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("packages: {:?}", packages.keys().len());
        packages
    }

    pub async fn fetch_dep_info(name: &str) -> Result<DepInfo, Box<dyn std::error::Error>> {
        let mut url = format!("https://crates.io/api/v1/crates/{}", name);
        if let Ok(_) = env::var("MEAIN_TEST_ENV") {
            url = format!("http://localhost:8000/cargo/{}.json", name)
        }
        let resp: CargoResponse = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", "depman (github.com/meain/depman)")
            .send()
            .await?
            .json()
            .await?;

        let versions = resp.versions.into_iter().map(|x| Version::parse(&x.num).unwrap()).collect();

        Ok(DepInfo{
            author: None,
            homepage: resp.info.homepage,
            license: resp.info.license,
            description: resp.info.description,
            repository: Some(format!("https://crates.io/crates/{}", name)),
            versions
        })
    }

    pub fn delete_dep(folder: &str, group: &str, name: &str) {
        let path_string = format!("{}/Cargo.toml", folder);
        let file_contents = std::fs::read_to_string(&path_string).unwrap();
        let mut doc = file_contents.parse::<Document>().expect("Invalid config file");
        doc[group][name] = toml_edit::Item::None;
        std::fs::write(&path_string, doc.to_string()).unwrap();
    }
}
