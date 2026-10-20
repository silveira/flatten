use std::fs::{self, File};
use std::process::Command;

use flatten::plan_flatten;
use tempfile::tempdir;

#[test]
fn test_plan_does_not_touch_the_filesystem() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    // Create root/sub/test.txt
    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    let plan = plan_flatten(root)?;

    // The plan describes the move...
    assert_eq!(plan.moves.len(), 1);
    assert_eq!(plan.moves[0].from, sub_dir.join("test.txt"));
    assert_eq!(plan.moves[0].to, root.join("test.txt"));

    // ...but nothing actually moved.
    assert!(sub_dir.join("test.txt").exists());
    assert!(sub_dir.exists());
    assert!(!root.join("test.txt").exists());

    Ok(())
}

#[test]
fn test_plan_matches_what_flattening_actually_does() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub1 = root.join("sub1");
    let sub2 = root.join("sub2");
    fs::create_dir(&sub1)?;
    fs::create_dir(&sub2)?;
    File::create(sub1.join("data.txt"))?;
    File::create(sub2.join("data.txt"))?;

    let plan = plan_flatten(root)?;

    flatten::flatten_directory(root)?;

    // Every planned destination is where the file actually ended up.
    for planned in &plan.moves {
        assert!(planned.to.exists(), "planned destination missing: {:?}", planned.to);
        assert!(!planned.from.exists(), "planned source still present: {:?}", planned.from);
    }

    Ok(())
}

#[test]
fn test_plan_renames_on_collision() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub1 = root.join("sub1");
    let sub2 = root.join("sub2");
    fs::create_dir(&sub1)?;
    fs::create_dir(&sub2)?;
    File::create(sub1.join("data.txt"))?;
    File::create(sub2.join("data.txt"))?;

    let plan = plan_flatten(root)?;

    // Two files with the same name must not be planned onto the same target.
    let targets: std::collections::HashSet<_> = plan.moves.iter().map(|m| &m.to).collect();
    assert_eq!(targets.len(), plan.moves.len());

    assert!(root.join("data.txt").exists() || plan.moves.iter().any(|m| m.to == root.join("data.txt")));
    assert!(plan.moves.iter().any(|m| m.to == root.join("data_1.txt")));

    Ok(())
}

#[test]
fn test_dryrun_flag_moves_nothing_and_reports_paths() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg("--dryrun")
        .arg(root)
        .output()?;

    assert!(output.status.success(), "--dryrun should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // It reports where the file is and where it would end up.
    assert!(stdout.contains("sub/test.txt"), "missing source path: {stdout}");
    assert!(stdout.contains("test.txt"), "missing destination path: {stdout}");
    assert!(stdout.contains("->"), "missing before/after arrow: {stdout}");

    // And nothing was actually moved.
    assert!(sub_dir.join("test.txt").exists());
    assert!(sub_dir.exists());
    assert!(!root.join("test.txt").exists());

    Ok(())
}

#[test]
fn test_dryrun_short_flag_works() -> std::io::Result<()> {
    let dir = tempdir()?;
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir)?;
    File::create(sub_dir.join("test.txt"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_flatten"))
        .arg("-n")
        .arg(root)
        .output()?;

    assert!(output.status.success(), "-n should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry run"), "missing dry run notice: {stdout}");
    assert!(sub_dir.join("test.txt").exists());

    Ok(())
}
