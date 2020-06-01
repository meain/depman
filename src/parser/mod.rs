mod javascriptnpm;
mod rustcargo;

use crate::render::InstallCandidate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;

use async_trait::async_trait;
use javascriptnpm::JavascriptNpm;
use rustcargo::RustCargo;

#[async_trait]
pub trait Parser {
    async fn parse(root: &str) -> Config;
    fn install_dep(dep: InstallCandidate, root: &str);
    async fn search_deps(name: &str) -> Result<Vec<SearchDep>, Box<dyn Error>>;
}

#[derive(Copy, Clone)]
pub enum ParserKind {
    RustCargo,
    JavascriptNpm,
}

#[derive(Debug, Clone)]
pub struct SearchDep {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Author {
    name: String,
    url: Option<String>,
    email: Option<String>,
}
impl Author {
    pub fn to_string(&self) -> String {
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

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: String,
    pub author: Option<Author>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub package_repo: String,
    pub license: Option<String>,
    pub specified_version: Option<semver::VersionReq>, // from config files
    pub current_version: Option<semver::Version>,      // parsed from lockfiles
    pub available_versions: Option<Vec<semver::Version>>,
    pub latest_version: Option<semver::Version>,
    pub latest_semver_version: Option<semver::Version>,
}

pub enum UpgradeType {
    None,
    Patch,
    Minor,
    Major,
}

impl Dep {
    pub fn get_author(&self) -> String {
        self.author.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_description(&self) -> String {
        self.description.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_homepage(&self) -> String {
        self.homepage.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_package_repo(&self) -> String {
        self.package_repo.to_string()
    }
    pub fn get_license(&self) -> String {
        self.license.as_ref().map_or("-".to_string(), |x| x.to_string())
    }

    pub fn get_ugrade_type(&self) -> UpgradeType {
        if let Some(cv) = &self.current_version {
            if let Some(sv) = &self.latest_semver_version {
                if cv.major < sv.major {
                    return UpgradeType::Major;
                }
                if cv.minor < sv.minor {
                    return UpgradeType::Minor;
                }
                if cv.patch < sv.patch {
                    return UpgradeType::Patch;
                }
            }
        }
        UpgradeType::None
    }

    pub fn get_version_strings(&self) -> Vec<String> {
        let mut version_strings = vec![];
        if let Some(av) = &self.available_versions {
            for version in av.iter().rev() {
                version_strings.push(version.to_string())
            }
        }
        version_strings
    }
    pub fn get_specified_version(&self) -> String {
        self.specified_version.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_current_version(&self) -> String {
        self.current_version.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_latest_version(&self) -> String {
        self.latest_version.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
    pub fn get_latest_semver_version(&self) -> String {
        self.latest_semver_version.as_ref().map_or("-".to_string(), |x| x.to_string())
    }
}

type DepGroup = BTreeMap<String, Dep>;

#[derive(Debug, Clone)]
pub struct Config {
    pub dep_groups: BTreeMap<String, DepGroup>,
}

impl Config {
    pub fn get_dep_kinds(&self) -> Vec<String> {
        let mut groups = vec![];
        for (gn, _) in self.dep_groups.iter() {
            groups.push(gn.to_string())
        }
        groups
    }
    pub fn get_dep(&self, dep_name: &str) -> Option<Dep> {
        for (_, group) in self.dep_groups.iter() {
            for (_, dep) in group.iter() {
                if dep_name == dep.name {
                    return Some(dep.clone());
                }
            }
        }
        None
    }
    pub fn get_dep_names_of_kind(&self, kind: &str) -> Vec<String> {
        let group = self.dep_groups.get(kind).unwrap();
        let mut names = vec![];
        for dep in group.keys().into_iter() {
            names.push(dep.to_string());
        }
        names
    }
}

impl Config {
    pub async fn new(folder: &str, kind: ParserKind) -> Self {
        match kind {
            ParserKind::JavascriptNpm => JavascriptNpm::parse(folder).await,
            ParserKind::RustCargo => RustCargo::parse(folder).await,
        }
    }
}

impl Config {
    pub fn install_dep(kind: ParserKind, dep: Option<InstallCandidate>, root: &str) -> bool {
        match dep {
            None => false,
            Some(d) => {
                match kind {
                    ParserKind::JavascriptNpm => JavascriptNpm::install_dep(d, root),
                    ParserKind::RustCargo => RustCargo::install_dep(d, root),
                }
                true
            }
        }
    }

    pub async fn search_deps(
        kind: ParserKind,
        name: &str,
    ) -> Result<Vec<SearchDep>, Box<dyn Error>> {
        match kind {
            ParserKind::JavascriptNpm => Ok(JavascriptNpm::search_deps(name).await?),
            ParserKind::RustCargo => Ok(RustCargo::search_deps(name).await?),
        }
    }
}
