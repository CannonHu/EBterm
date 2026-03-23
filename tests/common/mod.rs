//! Test utilities and helpers for integration tests

use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub mod mocks;

pub use mocks::*;

/// Create a temporary directory for tests
pub fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

/// Create a test file with content
pub fn create_test_file(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(filename);
    std::fs::write(&file_path, content).expect("Failed to write test file");
    file_path
}

/// Read file content
pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).expect("Failed to read test file")
}