use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about = "Flattens a directory by moving all nested files to the root.", long_about = None)]
struct Args {
    directory_name: PathBuf,
}

/// Core logic extracted for testability
fn flatten_directory(target_dir: &Path) -> std::io::Result<()> {
    let mut files_to_move = Vec::new();
    let mut dirs_to_remove = Vec::new();

    for entry in WalkDir::new(target_dir).contents_first(true) {
        let entry = entry.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(parent) = path.parent() {
                if parent != target_dir {
                    files_to_move.push(path.to_path_buf());
                }
            }
        } else if path.is_dir() && path != target_dir {
            dirs_to_remove.push(path.to_path_buf());
        }
    }

    for file_path in files_to_move {
        let file_name = file_path.file_name().unwrap();
        let mut new_path = target_dir.join(file_name);

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
    }

    for dir in dirs_to_remove {
        let _ = fs::remove_dir(dir);
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
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
}
