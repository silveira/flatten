//! Core logic for flattening a directory.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A single file relocation computed by [`plan_flatten`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMove {
    /// Where the file currently lives.
    pub from: PathBuf,
    /// Where the file would end up once the directory is flattened.
    pub to: PathBuf,
}

/// Every change flattening a directory would make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlattenPlan {
    /// Files that would be moved up into the target directory.
    pub moves: Vec<PlannedMove>,
    /// Directories that would be removed once emptied.
    pub dirs_to_remove: Vec<PathBuf>,
}

/// Works out what flattening `target_dir` would do, without changing anything.
///
/// Destination names are resolved against both the current contents of
/// `target_dir` and the other moves in the plan, so the result matches what
/// [`flatten_directory`] would really do.
pub fn plan_flatten(target_dir: &Path) -> std::io::Result<FlattenPlan> {
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

    let mut moves = Vec::with_capacity(files_to_move.len());
    // Destinations already claimed by the target directory or by an earlier move.
    let mut taken = HashSet::new();

    for file_path in files_to_move {
        let file_name = file_path.file_name().unwrap();
        let mut new_path = target_dir.join(file_name);

        let mut counter = 1;
        while new_path.exists() || taken.contains(&new_path) {
            let stem = file_path.file_stem().unwrap().to_string_lossy();
            let ext = file_path.extension().map(|e| e.to_string_lossy().to_string());

            let new_name = match ext {
                Some(extension) => format!("{}_{}.{}", stem, counter, extension),
                None => format!("{}_{}", stem, counter),
            };

            new_path = target_dir.join(new_name);
            counter += 1;
        }

        taken.insert(new_path.clone());
        moves.push(PlannedMove {
            from: file_path,
            to: new_path,
        });
    }

    Ok(FlattenPlan {
        moves,
        dirs_to_remove,
    })
}

/// Moves every file nested under `target_dir` up into `target_dir` itself.
///
/// Files that would overwrite an existing file are renamed with an incrementing
/// `_1`, `_2`, ... suffix. Subdirectories left empty afterwards are removed.
pub fn flatten_directory(target_dir: &Path) -> std::io::Result<()> {
    let plan = plan_flatten(target_dir)?;

    for planned in &plan.moves {
        fs::rename(&planned.from, &planned.to)?;
    }

    for dir in &plan.dirs_to_remove {
        let _ = fs::remove_dir(dir);
    }

    Ok(())
}
