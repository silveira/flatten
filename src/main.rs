use clap::Parser;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about = "Flattens a directory by moving all nested files to the root.", long_about = None)]
struct Args {
    /// The directory to flatten
    directory_name: PathBuf,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Canonicalize ensures absolute paths, avoiding issues with "./dir" vs "dir"
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

    let mut files_to_move = Vec::new();
    let mut dirs_to_remove = Vec::new();

    // Step 1: Map the directory. 
    // `contents_first(true)` guarantees we walk depth-first (visiting children before parents)
    for entry in WalkDir::new(&target_dir).contents_first(true) {
        let entry = entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            // If the file's parent is NOT the target directory, it's nested and needs moving
            if let Some(parent) = path.parent() {
                if parent != target_dir.as_path() {
                    files_to_move.push(path.to_path_buf());
                }
            }
        } else if path.is_dir() && path != target_dir.as_path() {
            // Queue nested directories for cleanup
            dirs_to_remove.push(path.to_path_buf());
        }
    }

    // Step 2: Move the files to the root of the target directory
    for file_path in files_to_move {
        let file_name = file_path.file_name().unwrap();
        let mut new_path = target_dir.join(file_name);

        // Handle naming collisions by appending an underscore and a number
        let mut counter = 1;
        while new_path.exists() {
            let stem = file_path.file_stem().unwrap().to_string_lossy();
            let ext = file_path.extension().map(|e| e.to_string_lossy().to_string());
            
            let new_name = match ext {
                Some(extension) => format!("{}_{}.{}", stem, counter, extension),
                None => format!("{}_{}", stem, counter),
            };
            
            new_path = target_dir.join(new_name);
            counter += 1;
        }

        fs::rename(&file_path, &new_path)?;
        println!("Moved: {:?} -> {:?}", file_path.file_name().unwrap(), new_path.file_name().unwrap());
    }

    // Step 3: Remove empty subdirectories
    for dir in dirs_to_remove {
        // `remove_dir` only succeeds if the directory is empty.
        // We ignore errors here in case there were hidden system files (like .DS_Store) we didn't move.
        let _ = fs::remove_dir(dir);
    }

    println!("Flattening complete!");
    Ok(())
}
