// mod javascriptnpm;
mod rustcargo;

use super::{determinekind::ParserKind, DepInfo, SearchDep};
use super::{Config, Lockfile};

use crate::render::InstallCandidate;
use rustcargo::RustCargo;
// use javascriptnpm::JavascriptNpm;

pub fn parse_config(folder: &str, kind: &ParserKind) -> Config {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_config(folder),
        _ => unreachable!(),
    }
}

pub fn parse_lockfile(folder: &str, kind: &ParserKind) -> Lockfile {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_lockfile(folder),
        _ => unreachable!(),
    }
}

pub async fn fetch_dep_info(
    name: String,
    kind: &ParserKind,
) -> Result<DepInfo, Box<dyn std::error::Error>> {
    match kind {
        ParserKind::RustCargo => RustCargo::fetch_dep_info(&name).await,
        _ => unreachable!(),
    }
}

pub fn delete_dep(kind: &ParserKind, folder: &str, group: &str, name: &str) -> bool {
    match kind {
        ParserKind::RustCargo => match RustCargo::delete_dep(folder, group, name) {
            Ok(_) => true,
            Err(_) => false,
        },
        _ => unreachable!(),
    }
}

pub fn install_dep(kind: &ParserKind, folder: &str, dep: InstallCandidate) -> bool {
    match kind {
        ParserKind::RustCargo => match RustCargo::install_dep(dep, folder) {
            Ok(_) => true,
            Err(_) => false,
        },
        _ => unreachable!(),
    }
}

pub async fn search_dep(kind: &ParserKind, term: &str) -> Option<Vec<SearchDep>> {
    match kind {
        ParserKind::RustCargo => RustCargo::search_dep(term).await,
        _ => unreachable!(),
    }
}
