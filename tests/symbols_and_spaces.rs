use std::fs::{self, File};

use flatten::flatten_directory;
use tempfile::tempdir;

#[test]
fn test_symbols_and_spaces() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    // Create root/messy #folder (1)/file #1.txt
    let messy_dir = root.join("messy #folder (1)");
    fs::create_dir(&messy_dir)?;
    File::create(messy_dir.join("file #1.txt"))?;

    flatten_directory(root)?;

    assert!(root.join("file #1.txt").exists());
    assert!(!messy_dir.exists());

    Ok(())
}
