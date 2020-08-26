use std::collections::hash_map::HashMap;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    parser::{Author, Config, DepInfo, DependencyGroup, Lockfile, SearchDep},
    render::InstallCandidate,
};

/// For config file
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JavascriptPackageJson {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<BTreeMap<String, String>>,
    #[serde(alias = "devDependencies")]
    dev_dependencies: Option<BTreeMap<String, String>>,
}

/// For lockfile
#[derive(Serialize, Deserialize, Debug, Clone)]
struct DepWithVersion {
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JavascriptPackageJsonLockfile {
    dependencies: BTreeMap<String, DepWithVersion>,
}

/// For metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmResponse {
    name: String,
    author: Option<Author>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    versions: BTreeMap<String, DepWithVersion>,
}

/// For search
#[derive(Serialize, Deserialize, Debug, Clone)]
struct NpmSearchDep {
    name: String,
    version: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct NpmSearchPackage {
    package: NpmSearchDep,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct NpmSearchResponse {
    objects: Vec<NpmSearchPackage>,
}

pub struct JavascriptNpm;
impl JavascriptNpm {
    pub fn is_this_it(folder: &str) -> bool {
        Path::new(&format!("{}/package.json", folder)).exists()
    }

    pub fn parse_config(folder: &str) -> Config {
        let path_string = format!("{}/package.json", folder);
        let path = Path::new(&path_string);
        let file = File::open(path).unwrap_or_else(|_| panic!("Unable to read {}", &path_string));
        let reader = BufReader::new(file);
        let parsed: JavascriptPackageJson = serde_json::from_reader(reader)
            .unwrap_or_else(|_| panic!("Unable to parse {}", &path_string));

        let mut groups: BTreeMap<String, DependencyGroup> = BTreeMap::new();
        if let Some(grp) = parsed.dependencies {
            let mut group: BTreeMap<String, Option<VersionReq>> = BTreeMap::new();
            for dep in grp.keys() {
                group.insert(
                    dep.to_string(),
                    VersionReq::parse(grp.get(dep).unwrap()).ok(),
                );
            }
            groups.insert("dependencies".to_string(), group);
        }
        if let Some(grp) = parsed.dev_dependencies {
            let mut group: BTreeMap<String, Option<VersionReq>> = BTreeMap::new();
            for dep in grp.keys() {
                group.insert(
                    dep.to_string(),
                    VersionReq::parse(grp.get(dep).unwrap()).ok(),
                );
            }
            groups.insert("dev-dependencies".to_string(), group);
        }

        let mut version = None;
        if let Some(v) = parsed.version {
            version = Version::parse(&v).ok()
        }

        Config {
            name: parsed.name,
            version,
            groups,
        }
    }

    pub fn parse_lockfile(folder: &str) -> Lockfile {
        let path_string = format!("{}/package-lock.json", folder);
        let path = Path::new(&path_string);
        let file = File::open(path).unwrap_or_else(|_| panic!("Unable to read {}", &path_string));
        let reader = BufReader::new(file);
        let parsed: JavascriptPackageJsonLockfile = serde_json::from_reader(reader)
            .unwrap_or_else(|_| panic!("Unable to parse {}", &path_string));

        let mut packages: Lockfile = HashMap::new();
        for dep in parsed.dependencies.keys() {
            packages.insert(
                dep.to_string(),
                Version::parse(&parsed.dependencies.get(dep).unwrap().version)
                    .unwrap_or_else(|_| Version::parse("0.0.0").unwrap()),  // or_else to deal with file:... like stuff
            );
        }
        packages
    }

    #[allow(clippy::useless_let_if_seq)]
    pub async fn fetch_dep_info(name: &str) -> Result<DepInfo, Box<dyn std::error::Error>> {
        let mut url = format!("https://registry.npmjs.org/{}", name);
        if env::var("MEAIN_TEST_ENV").is_ok() {
            url = format!("http://localhost:8000/npm/{}.json", name);
        }
        let resp: NpmResponse = reqwest::get(&url).await?.json().await?;

        let versions = resp
            .versions
            .keys()
            .map(|x| Version::parse(&x).unwrap())
            .collect();

        Ok(DepInfo {
            name: name.to_string(),
            author: resp.author,
            homepage: resp.homepage,
            license: resp.license,
            description: resp.description,
            repository: Some(format!("https://www.npmjs.com/package/{}", name)),
            versions,
        })
    }

    pub fn delete_dep(
        folder: &str,
        group: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_string = format!("{}/package.json", folder);
        let data = std::fs::read_to_string(&path_string)?;
        let mut package_json: serde_json::Value = serde_json::from_str(&data)?;
        if let Value::Object(pj) = &mut package_json[group] {
            pj.remove(name);
        }
        std::fs::write(&path_string, serde_json::to_string_pretty(&package_json)?)?;
        Ok(())
    }

    pub fn install_dep(
        dep: InstallCandidate,
        folder: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path_string = format!("{}/package.json", folder);
        let data = std::fs::read_to_string(&path_string)?;
        let mut package_json: serde_json::Value = serde_json::from_str(&data)?;
        package_json[dep.kind][dep.name] =
            serde_json::Value::String("^".to_string() + &dep.version);
        std::fs::write(&path_string, serde_json::to_string_pretty(&package_json)?)?;
        Ok(())
    }

    pub async fn search_dep(name: &str) -> Result<Vec<SearchDep>, Box<dyn std::error::Error>> {
        let url = format!(
            "http://registry.npmjs.com/-/v1/search?text={}&size=20",
            name
        );
        let resp: NpmSearchResponse = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", "depman (github.com/meain/depman)")
            .send()
            .await?
            .json()
            .await?;
        let mut deps: Vec<SearchDep> = vec![];
        for dep in resp.objects {
            deps.push(SearchDep {
                name: dep.package.name,
                version: dep.package.version,
            });
        }
        Ok(deps)
    }
}
