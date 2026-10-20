use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

use flatten::{flatten_directory, plan_flatten, FlattenPlan};

#[derive(Parser, Debug)]
#[command(author, version, about = "Flattens a directory by moving all nested files to the root.", long_about = None)]
struct Args {
    directory_name: PathBuf,

    /// Show what would happen without moving any files.
    #[arg(short = 'n', long)]
    dryrun: bool,
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

    if args.dryrun {
        print_dry_run(&target_dir, &plan_flatten(&target_dir)?);
    } else {
        flatten_directory(&target_dir)?;
        println!("Flattening complete!");
    }

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

/// Renders `path` relative to `root` so the output stays readable.
fn relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}
