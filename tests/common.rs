use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

pub fn create_test_file(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(filename);
    std::fs::write(&file_path, content).expect("Failed to write test file");
    file_path
}

pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).expect("Failed to read test file")
}
