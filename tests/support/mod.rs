//! Shared helpers for integration tests.

use std::fs;
use std::path::Path;

use cidian::{Dictionary, Entry, Result};

/// Reads a real dictionary fixture from the repository at test runtime.
pub fn read_fixture(format: &str, file_name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format)
        .join(file_name);

    match fs::read(&path) {
        Ok(data) => data,
        Err(error) => panic!(
            "failed to read {format} fixture `{}`: {error}",
            path.display()
        ),
    }
}

/// Parses a fixture and turns parser errors into a test failure with context.
#[track_caller]
pub fn parse_for_test(
    file_name: &str,
    data: &[u8],
    parser: fn(&[u8]) -> Result<Dictionary>,
) -> Dictionary {
    match parser(data) {
        Ok(dictionary) => dictionary,
        Err(error) => panic!("failed to parse {file_name}: {error}"),
    }
}

/// Checks a dictionary entry's word and coding components.
#[track_caller]
pub fn assert_entry(file_name: &str, index: usize, entry: &Entry, word: &str, code: &[&str]) {
    assert_eq!(entry.word, word, "{file_name}: word at entry {index}");
    assert_eq!(
        entry.code.as_slice(),
        code,
        "{file_name}: code at entry {index}"
    );
}
