use std::fs::{self, File};

use flatten::flatten_directory;
use tempfile::tempdir;

#[test]
fn test_basic_flattening() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    // Create root/sub/test.txt
    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    flatten_directory(root)?;

    // Assert file moved to root and sub_dir was deleted
    assert!(root.join("test.txt").exists());
    assert!(!sub_dir.exists());

    Ok(())
}
