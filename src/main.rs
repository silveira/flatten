use clap::Parser;
use std::fs;
use std::path::PathBuf;

use flatten::flatten_directory;

#[derive(Parser, Debug)]
#[command(author, version, about = "Flattens a directory by moving all nested files to the root.", long_about = None)]
struct Args {
    directory_name: PathBuf,
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

    flatten_directory(&target_dir)?;
    println!("Flattening complete!");

    Ok(())
}
