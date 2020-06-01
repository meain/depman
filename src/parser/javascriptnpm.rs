use crate::render::InstallCandidate;
use futures::future::try_join_all;
use humanesort::prelude::*;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{Config, DepGroup, Parser};
use crate::parser::{Author, Dep, SearchDep};
use serde::{Deserialize, Serialize};

use async_trait::async_trait;
use semver::{Version, VersionReq};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavascriptPackageJson {
    name: String,
    dependencies: Option<HashMap<String, String>>,
    #[serde(alias = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}
impl JavascriptPackageJson {
    fn from(folder: &str) -> Option<JavascriptPackageJson> {
        let path_string = format!("{}/package.json", folder);
        let path = Path::new(&path_string);
        let file = File::open(path).expect(&format!("Unable to read {}", &path_string));
        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(package_json) => Some(package_json),
            _ => None,
        }
    }
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
    fn from(folder: &str) -> Option<JavascriptPackageJsonLockfile> {
        let path_string = format!("{}/package-lock.json", folder);
        let path = Path::new(&path_string);
        let file = File::open(path).expect(&format!("Unable to read {}", &path_string));
        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(package_json) => Some(package_json),
            _ => None,
        }
    }
    pub fn get_lockfile_version(&self, name: &str) -> Option<String> {
        match &self.dependencies {
            Some(deps) => Some(deps[name].version.clone()),
            None => None,
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
    pub fn get_versions_list(&self) -> Vec<Version> {
        let mut versions = vec![];
        for (_, ver) in self.versions.iter() {
            if let Ok(v) = Version::parse(&ver.version) {
                versions.push(v)
            }
        }
        versions
    }

    pub fn inject_inportant_versions(&self, dep: &mut Dep) {
        let mut key_list: Vec<String> = self
            .versions
            .iter()
            .map(|(_, x)| x.version.clone())
            .collect();
        key_list.humane_sort();

        let mut parsed_versions: Vec<Version> = Vec::new();
        let mut latest_semantic_version: Option<Version> = None;
        let mut latest_version: Option<Version> = None;
        for key in key_list {
            if let Ok(valid_version) = Version::parse(&key) {
                parsed_versions.push(valid_version.clone());
                latest_version = Some(valid_version.clone());
                if let Some(spec) = &dep.specified_version {
                    if spec.matches(&valid_version) {
                        latest_semantic_version = Some(valid_version.clone());
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
struct ResponseWithMeta {
    data: NpmResponse,
    name: String,
    kind: String,
}
async fn fetch_resp(dep: &str, kind: String, name: String) -> Result<ResponseWithMeta, Box<dyn Error>> {
    let mut url = format!("https://registry.npmjs.org/{}", dep);
    match env::var("MEAIN_TEST_ENV") {
        Ok(_) => url = format!("http://localhost:8000/npm/{}.json", dep),
        _ => {}
    }
    let resp = reqwest::get(&url).await?.json().await?;
    Ok(ResponseWithMeta { data: resp, kind, name })
}

async fn fetch_dep_infos(config: &mut Config) -> Result<(), Box<dyn Error + 'static>> {
    let mut gets = vec![];
    for (kind, group) in config.dep_groups.iter() {
        for (name, dep) in group.iter() {
            // so that we do not refetch it on rerender
            if let Some(_) = dep.latest_version {
                gets.push(fetch_resp(name, kind.to_string(), name.to_string()));
            }
        }
    }
    let results = try_join_all(gets).await?;
    for result in &results {
        let mut dep = &mut config
            .dep_groups
            .get_mut(&result.kind)
            .unwrap()
            .get_mut(&result.name)
            .unwrap();
        dep.description = result.data.description.clone();
        dep.available_versions = Some(result.data.get_versions_list());
        dep.license = result.data.license.clone();
        dep.homepage = result.data.homepage.clone();
        dep.author = result.data.author.clone();
        result.data.inject_inportant_versions(dep);
    }
    Ok(())
}

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
        package_repo: format!("https://www.npmjs.com/package/{}", name),
        license: None,
        specified_version: VersionReq::parse(specified_version).ok(), // from config files
        current_version: current_version_parsed,                      // parsed from lockfiles
        available_versions: None,
        latest_version: None,
        latest_semver_version: None,
    }
}

fn build_deps_array(
    kind: &str,
    deps: HashMap<String, String>,
    lockfile: &JavascriptPackageJsonLockfile,
) -> DepGroup {
    let mut dep_group: DepGroup = HashMap::new();
    deps.keys().into_iter().for_each(|x| {
        dep_group.insert(
            x.to_string(),
            build_dep_entry(x, &kind, &deps[x], lockfile.get_lockfile_version(x)),
        );
    });
    dep_group
}

pub struct JavascriptNpm;

#[async_trait]
impl Parser for JavascriptNpm {
    async fn parse(folder: &str) -> Config {
        let configfile = JavascriptPackageJson::from(folder).expect("Unable to read package.json");
        let lockfile =
            JavascriptPackageJsonLockfile::from(folder).expect("Unable to read package-lock.json");
        let mut dep_groups = HashMap::new();
        if let Some(deps) = configfile.dependencies {
            dep_groups.insert(
                "dependencies".to_string(),
                build_deps_array("dependencies", deps, &lockfile),
            );
        }
        if let Some(deps) = configfile.dev_dependencies {
            dep_groups.insert(
                "devDependencies".to_string(),
                build_deps_array("devDependencies", deps, &lockfile),
            );
        }

        let mut config = Config { dep_groups };
        let _ = fetch_dep_infos(&mut config).await; // ignore error
        config
    }

    fn install_dep(dep: InstallCandidate, folder: &str) {
        let path_string = format!("{}/package.json", folder);
        let data = std::fs::read_to_string(&path_string).unwrap();
        let mut package_json: serde_json::Value = serde_json::from_str(&data).unwrap();
        package_json[dep.kind][dep.name] =
            serde_json::Value::String("^".to_string() + &dep.version);
        std::fs::write(
            &path_string,
            serde_json::to_string_pretty(&package_json).unwrap(),
        )
        .unwrap();
    }
    async fn search_deps(name: &str) -> Result<Vec<SearchDep>, Box<dyn Error>> {
        let url = format!(
            "http://registry.npmjs.com/-/v1/search?text={}&size=10",
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
