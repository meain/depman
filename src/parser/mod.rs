mod jspackagejson;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum DepVersion {
    Version(semver::Version),
    None,
}

impl DepVersion {
    pub fn to_string(&self) -> String {
        match self {
            DepVersion::None => "<unknown>".to_string(),
            DepVersion::Version(v) => v.to_string(),
        }
    }

    pub fn from(string: Option<String>) -> Self {
        match string {
            Some(s) => {
                let dvv = semver::Version::parse(&s);
                match dvv {
                    Ok(dv) => DepVersion::Version(dv),
                    _ => DepVersion::None,
                }
            }
            None => DepVersion::None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DepVersionReq {
    // might have to add stuff like guthub repo or file here
    Version(semver::VersionReq),
    None,
}

impl DepVersionReq {
    pub fn to_string(&self) -> String {
        match self {
            DepVersionReq::None => "<unknown>".to_string(),
            DepVersionReq::Version(v) => v.to_string(),
        }
    }
    pub fn from(string: &str) -> Self {
        let dvv = semver::VersionReq::parse(string);
        match dvv {
            Ok(dv) => DepVersionReq::Version(dv),
            _ => DepVersionReq::None,
        }
    }
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
        match &self.email {
            Some(v) => author_string = format!("{} <{}>", author_string, &v.to_string()),
            None => {}
        }
        match &self.url {
            Some(v) => author_string = format!("{} [{}]", author_string, &v.to_string()),
            None => {}
        }
        author_string
    }
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub author: Option<Author>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub specified_version: DepVersionReq, // from config files
    pub current_version: DepVersion,      // parsed from lockfiles
    pub available_versions: Option<Vec<DepVersion>>,
    pub latest_version: Option<DepVersion>,
    pub latest_semver_version: Option<DepVersion>,
}

pub enum UpgradeType {
    None,
    Patch,
    Minor,
    Major,
    // maybe unknown
}

impl Dep {
    pub fn get_name(&self) -> String {
        self.name.to_string()
    }
    pub fn get_author(&self) -> String {
        match &self.author {
            Some(value) => value.to_string(),
            None => "<unknown>".to_string(),
        }
    }
    pub fn get_description(&self) -> String {
        match &self.description {
            Some(value) => value.to_string(),
            None => "<unknown>".to_string(),
        }
    }
    pub fn get_homepage(&self) -> String {
        match &self.homepage {
            Some(value) => value.to_string(),
            None => "<unknown>".to_string(),
        }
    }
    pub fn get_license(&self) -> String {
        match &self.license {
            Some(value) => value.to_string(),
            None => "<unknown>".to_string(),
        }
    }

    pub fn get_ugrade_type(&self) -> UpgradeType {
        match &self.current_version {
            DepVersion::Version(cv) => match &self.latest_semver_version {
                Some(svv) => match svv {
                    DepVersion::Version(sv) => {
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
                    _ => {}
                },
                _ => {}
            },
            _ => {}
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
        self.specified_version.to_string()
    }
    pub fn get_current_version(&self) -> String {
        self.current_version.to_string()
    }
    pub fn get_latest_version(&self) -> String {
        match &self.latest_version {
            Some(v) => v.to_string(),
            None => "<unknown>".to_string(),
        }
    }
    pub fn get_latest_semver_version(&self) -> String {
        match &self.latest_semver_version {
            Some(v) => v.to_string(),
            None => "<unknown>".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DepList {
    pub name: String,
    pub deps: Vec<Dep>, // Could be hashmap, but that might cause if someone lets multiple versions to exist
}

#[derive(Debug, Clone)]
pub struct DepListList {
    pub lists: Vec<DepList>,
}

impl DepListList {
    pub fn get_dep_kinds(&self) -> Vec<String> {
        let mut kinds = vec![];
        for dep_list in &self.lists {
            let kind = dep_list.name.clone();
            kinds.push(kind)
        }
        kinds
    }

    pub fn get_dep(&self, dep_name: &str) -> Option<Dep> {
        for dep_list in &self.lists {
            for dep in &dep_list.deps {
                if dep_name == dep.name {
                    return Some(dep.clone());
                }
            }
        }
        None
    }
    pub fn get_dep_names(&self) -> Vec<String> {
        let mut deps = vec![];
        for dep_list in &self.lists {
            for dep in &dep_list.deps {
                let name = dep.name.to_string();
                deps.push(name);
            }
        }
        deps
    }
    pub fn get_dep_names_of_kind(&self, kind: &str) -> Vec<String> {
        let mut deps = vec![];
        for dep_list in &self.lists {
            if kind != &dep_list.name {
                continue;
            }
            for dep in &dep_list.deps {
                let name = dep.name.to_string();
                deps.push(name);
            }
        }
        deps
    }
}

impl DepListList {
    pub async fn new(folder: &str, kind: &str) -> DepListList {
        match kind {
            "javascript-package-json" => jspackagejson::into(folder).await,
            _ => unreachable!(),
        }
    }
}
