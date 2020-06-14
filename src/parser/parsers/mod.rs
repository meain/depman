// mod javascriptnpm;
mod rustcargo;

use super::{Config, Lockfile};
use super::{DepInfo, determinekind::ParserKind};

use rustcargo::RustCargo;
// use javascriptnpm::JavascriptNpm;

pub fn parse_config(folder: &str, kind: &ParserKind) -> Config {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_config(folder),
        _ => unreachable!()
    }
}


pub fn parse_lockfile(folder: &str, kind: &ParserKind) -> Lockfile {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_lockfile(folder),
        _ => unreachable!()
    }
}


pub async fn fetch_dep_info(name: String, kind: &ParserKind) ->Result<DepInfo, Box<dyn std::error::Error>> {
    match kind {
        ParserKind::RustCargo => RustCargo::fetch_dep_info(&name).await,
        _ => unreachable!()
    }
}
