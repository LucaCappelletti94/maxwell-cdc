//! Fixture discovery shared by the tests and the benches. Read at run time, so the
//! directory is the only list of what the corpus covers.

// Each test binary that includes this module calls only a subset, so the unused ones would
// warn. `redundant_pub_crate` wants `pub` here while the compiler's `unreachable_pub` wants
// `pub(crate)`; the compiler lint wins.
#![allow(dead_code, clippy::redundant_pub_crate)]

use std::path::{Path, PathBuf};

/// Absolute path of the fixture directory.
pub(crate) fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Every fixture as a `(name, raw bytes)` pair, sorted by name. The bytes are what Maxwell
/// wrote, so comparing against them compares against the wire format.
pub(crate) fn all() -> Vec<(String, String)> {
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
pub(crate) fn get(name: &str) -> String {
    let path = dir().join(format!("{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
