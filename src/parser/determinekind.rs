use std::path::Path;

#[derive(Clone)]
pub enum ParserKind {
    JavascriptNpm,
    RustCargo,
}

impl ParserKind {
    pub fn determine_kind(folder: &str) -> Option<ParserKind> {
        if Path::new(&format!("{}/package-lock.json", folder)).exists() {
            Some(ParserKind::JavascriptNpm)
        } else if Path::new(&format!("{}/Cargo.toml", folder)).exists() {
            Some(ParserKind::RustCargo)
        } else {
            None
        }
    }
}
