mod javascriptnpm;
mod rustcargo;

use super::{determinekind::ParserKind, DepInfo, SearchDep};
use super::{Config, Lockfile};

use crate::render::InstallCandidate;
use javascriptnpm::JavascriptNpm;
use rustcargo::RustCargo;

pub fn parse_config(folder: &str, kind: &ParserKind) -> Config {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_config(folder),
        ParserKind::JavascriptNpm => JavascriptNpm::parse_config(folder),
    }
}

pub fn parse_lockfile(folder: &str, kind: &ParserKind) -> Lockfile {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_lockfile(folder),
        ParserKind::JavascriptNpm => JavascriptNpm::parse_lockfile(folder),
    }
}

pub async fn fetch_dep_info(
    name: String,
    kind: &ParserKind,
) -> Result<DepInfo, Box<dyn std::error::Error>> {
    match kind {
        ParserKind::RustCargo => RustCargo::fetch_dep_info(&name).await,
        ParserKind::JavascriptNpm => JavascriptNpm::fetch_dep_info(&name).await,
    }
}

pub fn delete_dep(kind: &ParserKind, folder: &str, group: &str, name: &str) -> bool {
    match kind {
        ParserKind::RustCargo => RustCargo::delete_dep(folder, group, name).is_ok(),
        ParserKind::JavascriptNpm => JavascriptNpm::delete_dep(folder, group, name).is_ok()
    }
}

pub fn install_dep(kind: &ParserKind, folder: &str, dep: InstallCandidate) -> bool {
    match kind {
        ParserKind::RustCargo => RustCargo::install_dep(dep, folder).is_ok(),
        ParserKind::JavascriptNpm => JavascriptNpm::install_dep(dep, folder).is_ok()
    }
}

pub async fn search_dep(
    kind: &ParserKind,
    term: &str,
) -> Result<Vec<SearchDep>, Box<dyn std::error::Error>> {
    match kind {
        ParserKind::RustCargo => RustCargo::search_dep(term).await,
        ParserKind::JavascriptNpm => JavascriptNpm::search_dep(term).await,
    }
}
