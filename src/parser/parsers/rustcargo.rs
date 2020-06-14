use std::fs;

use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use toml::Value;

use crate::parser::{Config, DependencyGroup};

pub struct RustCargo;
impl RustCargo {
    pub fn parse_config(folder: &str) -> Config {
        let path_string = format!("{}/Cargo.toml", folder);
        let text =
            fs::read_to_string(&path_string).expect(&format!("Unable to read {}", &path_string));
        let parsed: Value = toml::from_str(&text).expect(&format!("Unable to parse {}", &path_string));

        let name = match &parsed["name"] {
            Value::String(v) => Some(v.to_string()),
            _ => None,
        };
        let version = match &parsed["version"] {
            Value::String(v) => {
                if let Ok(v) = Version::parse(&v) {
                    Some(v)
                } else {
                    None
                }
            }
            _ => None,
        };

        // Get all dep groups
        let groups: BTreeMap<String, DependencyGroup> = BTreeMap::new();
        if let Value::Table(conf) = parsed {
            for key in conf.keys() {
                if key == "dependencies" {
                    if let Value::Table(gr) = &conf["dependencies"] {
                        let mut group: BTreeMap<String, Option<VersionReq>> = BTreeMap::new();
                        for dep in gr.keys() {
                            let version_req = match &gr[dep] {
                                Value::String(v) => match VersionReq::parse(&v) {
                                    Ok(vr) => Some(vr),
                                    Err(_) => None
                                },
                                _ => None
                            };
                            group.insert(dep.to_string(), version_req);
                        }
                    }
                }
            }
        }

        Config{
            name,
            version,
            groups
        }
    }
}
