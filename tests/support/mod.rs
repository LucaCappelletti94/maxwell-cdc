//! Fixture discovery shared by the integration tests and the benches.
//!
//! Fixtures are read at run time from `tests/fixtures`, so the directory is the single
//! source of truth for which message types the corpus covers. Adding a file is enough to
//! pull it into every test and bench that iterates the corpus.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Absolute path of the fixture directory.
pub fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Every fixture as a `(name, raw bytes)` pair, sorted by name.
///
/// The bytes are exactly what Maxwell wrote, so a comparison against them is a comparison
/// against the wire format rather than against this crate's own output.
pub fn all() -> Vec<(String, String)> {
    let mut fixtures: Vec<(String, String)> = std::fs::read_dir(dir())
        .unwrap_or_else(|e| panic!("read {}: {e}", dir().display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let name = path
                .file_stem()
                .expect("fixture stem")
                .to_str()
                .expect("utf-8 fixture name")
                .to_owned();
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            (name, raw)
        })
        .collect();

    assert!(!fixtures.is_empty(), "fixture directory is empty");
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

/// One named fixture's raw bytes.
pub fn get(name: &str) -> String {
    let path = dir().join(format!("{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
