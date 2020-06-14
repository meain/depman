// mod javascriptnpm;
mod rustcargo;

use super::Config;
use super::determinekind::ParserKind;

use rustcargo::RustCargo;
// use javascriptnpm::JavascriptNpm;

pub fn parse_config(folder: &str, kind: &ParserKind) -> Config {
    match kind {
        ParserKind::RustCargo => RustCargo::parse_config(folder),
        _ => unreachable!()
    }
}
