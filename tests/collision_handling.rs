use std::fs::{self, File};

use flatten::flatten_directory;
use tempfile::tempdir;

#[test]
fn test_collision_handling() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    // Create root/sub1/data.txt and root/sub2/data.txt
    let sub1 = root.join("sub1");
    let sub2 = root.join("sub2");
    fs::create_dir(&sub1)?;
    fs::create_dir(&sub2)?;

    File::create(sub1.join("data.txt"))?;
    File::create(sub2.join("data.txt"))?;

    flatten_directory(root)?;

    // Both files should exist at root, one renamed
    assert!(root.join("data.txt").exists());
    assert!(root.join("data_1.txt").exists());
    assert!(!sub1.exists());
    assert!(!sub2.exists());

    Ok(())
}
