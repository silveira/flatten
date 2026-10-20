use std::fs::{self, File};
use std::process::Command;

use flatten::{
    flatten_directory_recording, plan_undo, read_history, undo_directory, UNDO_FILE_NAME,
};
use tempfile::tempdir;

#[test]
fn test_flatten_records_history() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    flatten_directory_recording(root)?;

    // The history file is written into the target directory.
    let history_path = root.join(UNDO_FILE_NAME);
    assert!(history_path.exists(), "history file should exist");

    let history = read_history(root)?;
    assert_eq!(history.entries.len(), 1);

    let entry = &history.entries[0];
    assert_eq!(entry.moves.len(), 1);
    assert_eq!(entry.moves[0].from, sub_dir.join("test.txt"));
    assert_eq!(entry.moves[0].to, root.join("test.txt"));
    assert_eq!(entry.removed_dirs, vec![sub_dir.clone()]);

    Ok(())
}

#[test]
fn test_history_file_is_valid_json() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    flatten_directory_recording(root)?;

    let contents = fs::read_to_string(root.join(UNDO_FILE_NAME))?;
    let parsed: serde_json::Value = serde_json::from_str(&contents)?;

    assert!(parsed["version"].is_number());
    assert!(parsed["entries"].is_array());
    assert_eq!(parsed["entries"][0]["moves"][0]["from"], sub_dir.join("test.txt").to_str().unwrap());

    Ok(())
}

#[test]
fn test_undo_restores_original_structure() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub = root.join("sub");
    let nested = sub.join("nested");
    fs::create_dir_all(&nested)?;
    File::create(sub.join("a.txt"))?;
    File::create(nested.join("b.txt"))?;
    File::create(root.join("root.txt"))?;

    flatten_directory_recording(root)?;

    assert!(root.join("a.txt").exists());
    assert!(root.join("b.txt").exists());
    assert!(!sub.exists());

    undo_directory(root)?;

    // Everything is back where it started.
    assert!(sub.join("a.txt").exists());
    assert!(nested.join("b.txt").exists());
    assert!(root.join("root.txt").exists());
    assert!(!root.join("a.txt").exists());
    assert!(!root.join("b.txt").exists());

    Ok(())
}

#[test]
fn test_undo_removes_history_file_when_empty() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    flatten_directory_recording(root)?;
    assert!(root.join(UNDO_FILE_NAME).exists());

    undo_directory(root)?;

    // Nothing left to undo, so the history file is cleaned up.
    assert!(!root.join(UNDO_FILE_NAME).exists());

    Ok(())
}

#[test]
fn test_undo_restores_collisions_to_original_names() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub1 = root.join("sub1");
    let sub2 = root.join("sub2");
    fs::create_dir(&sub1)?;
    fs::create_dir(&sub2)?;
    File::create(sub1.join("data.txt"))?;
    File::create(sub2.join("data.txt"))?;

    flatten_directory_recording(root)?;

    assert!(root.join("data.txt").exists());
    assert!(root.join("data_1.txt").exists());

    undo_directory(root)?;

    // Each file went back to its own directory, not to a renamed copy.
    assert!(sub1.join("data.txt").exists());
    assert!(sub2.join("data.txt").exists());
    assert!(!root.join("data.txt").exists());
    assert!(!root.join("data_1.txt").exists());

    Ok(())
}

#[test]
fn test_undo_is_reversible_across_multiple_runs() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let one = root.join("one");
    fs::create_dir(&one)?;
    File::create(one.join("first.txt"))?;

    flatten_directory_recording(root)?;

    let two = root.join("two");
    fs::create_dir(&two)?;
    File::create(two.join("second.txt"))?;

    flatten_directory_recording(root)?;
    assert_eq!(read_history(root)?.entries.len(), 2);

    // Undo peels one layer at a time.
    undo_directory(root)?;
    assert!(two.join("second.txt").exists());
    assert!(!root.join("second.txt").exists());
    // The first run is still in effect, so its file stays at the root.
    assert!(root.join("first.txt").exists());

    undo_directory(root)?;
    assert!(one.join("first.txt").exists());
    assert!(!root.join("first.txt").exists());
    assert!(!root.join("second.txt").exists());

    Ok(())
}

#[test]
fn test_undo_without_history_reports_nothing_to_undo() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    assert!(plan_undo(root)?.is_none());
    assert!(undo_directory(root)?.is_none());

    Ok(())
}

#[test]
fn test_undo_refuses_when_a_file_is_missing() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    flatten_directory_recording(root)?;

    // Something removed the flattened file, so it cannot be moved back.
    fs::remove_file(root.join("test.txt"))?;

    let result = undo_directory(root);
    assert!(result.is_err(), "undo should fail when a file is missing");

    // The history is kept so the state can be inspected.
    assert_eq!(read_history(root)?.entries.len(), 1);

    Ok(())
}

#[test]
fn test_undo_flag_restores_files() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    // Flatten through the CLI so the history file is written.
    let flatten = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg(root)
        .output()?;
    assert!(flatten.status.success());
    assert!(root.join("test.txt").exists());

    let undo = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg("--undo")
        .arg(root)
        .output()?;
    assert!(undo.status.success());

    let stdout = String::from_utf8_lossy(&undo.stdout);
    assert!(stdout.contains("Undo complete"), "unexpected output: {stdout}");

    assert!(sub_dir.join("test.txt").exists());
    assert!(!root.join("test.txt").exists());

    Ok(())
}

#[test]
fn test_undo_dryrun_moves_nothing() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg(root)
        .output()?;

    let undo = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg("--undo")
        .arg("--dryrun")
        .arg(root)
        .output()?;
    assert!(undo.status.success());

    let stdout = String::from_utf8_lossy(&undo.stdout);
    assert!(stdout.contains("->"), "missing before/after arrow: {stdout}");

    // Nothing was moved back.
    assert!(root.join("test.txt").exists());
    assert!(!sub_dir.join("test.txt").exists());

    Ok(())
}

#[test]
fn test_undo_without_history_exits_with_error() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let undo = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg("--undo")
        .arg(root)
        .output()?;

    assert!(!undo.status.success(), "undo without history should fail");

    let stderr = String::from_utf8_lossy(&undo.stderr);
    assert!(
        stderr.contains("No flatten operation to undo"),
        "unexpected error: {stderr}"
    );

    Ok(())
}
