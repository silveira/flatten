use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

use flatten::{
    flatten_directory_recording, plan_flatten, plan_undo, undo_directory, FlattenPlan, UndoPlan,
};

#[derive(Parser, Debug)]
#[command(author, version, about = "Flattens a directory by moving all nested files to the root.", long_about = None)]
struct Args {
    directory_name: PathBuf,

    /// Show what would happen without moving any files.
    #[arg(short = 'n', long)]
    dryrun: bool,

    /// Undo the last flatten operation on this directory.
    #[arg(long)]
    undo: bool,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let target_dir = match fs::canonicalize(&args.directory_name) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Cannot resolve directory path: {}", e);
            std::process::exit(1);
        }
    };

    if !target_dir.is_dir() {
        eprintln!("Error: {:?} is not a directory.", target_dir);
        std::process::exit(1);
    }

    if args.undo {
        undo(&target_dir, args.dryrun)?;
    } else if args.dryrun {
        print_dry_run(&target_dir, &plan_flatten(&target_dir)?);
    } else {
        flatten_directory_recording(&target_dir)?;
        println!("Flattening complete!");
    }

    Ok(())
}

/// Undoes the last flatten operation, or previews it when `dryrun` is set.
fn undo(target_dir: &Path, dryrun: bool) -> std::io::Result<()> {
    let plan = match plan_undo(target_dir)? {
        Some(plan) => plan,
        None => {
            eprintln!("Error: No flatten operation to undo in this directory.");
            std::process::exit(1);
        }
    };

    if dryrun {
        print_undo_dry_run(target_dir, &plan);
        return Ok(());
    }

    undo_directory(target_dir)?;
    println!("Undo complete!");

    Ok(())
}

/// Prints the plan without touching the filesystem.
fn print_dry_run(target_dir: &Path, plan: &FlattenPlan) {
    println!("Dry run: no files were moved.\n");

    if plan.moves.is_empty() && plan.dirs_to_remove.is_empty() {
        println!("Nothing to flatten: the directory is already flat.");
        return;
    }

    if !plan.moves.is_empty() {
        println!("Moves:");
        for planned in &plan.moves {
            println!(
                "  {} -> {}",
                relative_to(target_dir, &planned.from).display(),
                relative_to(target_dir, &planned.to).display()
            );
        }
        println!();
    }

    if !plan.dirs_to_remove.is_empty() {
        println!("Directories to remove:");
        for dir in &plan.dirs_to_remove {
            println!("  {}", relative_to(target_dir, dir).display());
        }
        println!();
    }

    println!(
        "{} file(s) would be moved, {} directory(ies) would be removed.",
        plan.moves.len(),
        plan.dirs_to_remove.len()
    );
}

/// Prints the undo plan without touching the filesystem.
fn print_undo_dry_run(target_dir: &Path, plan: &UndoPlan) {
    println!("Dry run: no files were moved back.\n");

    if !plan.moves.is_empty() {
        println!("Moves back:");
        for planned in &plan.moves {
            println!(
                "  {} -> {}",
                relative_to(target_dir, &planned.from).display(),
                relative_to(target_dir, &planned.to).display()
            );
        }
        println!();
    }

    if !plan.dirs_to_create.is_empty() {
        println!("Directories to recreate:");
        for dir in &plan.dirs_to_create {
            println!("  {}", relative_to(target_dir, dir).display());
        }
        println!();
    }

    println!(
        "{} file(s) would be moved back, {} directory(ies) would be recreated.",
        plan.moves.len(),
        plan.dirs_to_create.len()
    );
}

/// Renders `path` relative to `root` so the output stays readable.
fn relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}
