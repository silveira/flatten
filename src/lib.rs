//! Core logic for flattening a directory.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Name of the hidden file where the undo history is stored.
pub const UNDO_FILE_NAME: &str = ".flatten_history.json";

/// Format version of the history file.
const HISTORY_VERSION: u32 = 1;

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

/// Every change undoing the last flattening would make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoPlan {
    /// Files that would be moved back to where they came from.
    pub moves: Vec<PlannedMove>,
    /// Directories that would be recreated.
    pub dirs_to_create: Vec<PathBuf>,
}

/// One recorded flattening, with everything needed to reverse it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// When the flattening ran, as seconds since the Unix epoch.
    pub timestamp: u64,
    /// The files that were moved, in the order they were moved.
    pub moves: Vec<RecordedMove>,
    /// The directories that were removed once emptied.
    pub removed_dirs: Vec<PathBuf>,
}

/// A single recorded file move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMove {
    /// Where the file came from.
    pub from: PathBuf,
    /// Where the file was moved to.
    pub to: PathBuf,
}

/// The on-disk undo history for a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    /// Format version, so future changes can still read older files.
    pub version: u32,
    /// The directory the history belongs to.
    pub root: PathBuf,
    /// Every flattening that has been applied, oldest first.
    pub entries: Vec<HistoryEntry>,
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
    apply_flatten_plan(&plan)
}

/// Applies a previously computed flatten plan.
pub fn apply_flatten_plan(plan: &FlattenPlan) -> std::io::Result<()> {
    for planned in &plan.moves {
        fs::rename(&planned.from, &planned.to)?;
    }

    for dir in &plan.dirs_to_remove {
        let _ = fs::remove_dir(dir);
    }

    Ok(())
}

/// Flattens `target_dir` and records what happened so it can be undone.
///
/// The history is stored in a hidden file inside `target_dir`. Flattening a
/// directory that is already flat records nothing, so it stays undoable.
pub fn flatten_directory_recording(target_dir: &Path) -> std::io::Result<History> {
    let plan = plan_flatten(target_dir)?;
    let mut history = read_history(target_dir)?;

    apply_flatten_plan(&plan)?;

    if !plan.moves.is_empty() || !plan.dirs_to_remove.is_empty() {
        history.entries.push(HistoryEntry {
            timestamp: now_seconds(),
            moves: plan
                .moves
                .iter()
                .map(|m| RecordedMove {
                    from: m.from.clone(),
                    to: m.to.clone(),
                })
                .collect(),
            removed_dirs: plan.dirs_to_remove.clone(),
        });
    }

    write_history(target_dir, &history)?;

    Ok(history)
}

/// Reads the undo history stored in `target_dir`, if there is one.
pub fn read_history(target_dir: &Path) -> std::io::Result<History> {
    let path = target_dir.join(UNDO_FILE_NAME);

    if !path.exists() {
        return Ok(History {
            version: HISTORY_VERSION,
            root: target_dir.to_path_buf(),
            entries: Vec::new(),
        });
    }

    let contents = fs::read_to_string(&path)?;
    let history: History = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(history)
}

/// Writes the undo history for `target_dir`.
///
/// When there is nothing left to undo the history file is removed, so the
/// directory is left exactly as it was before flattening.
pub fn write_history(target_dir: &Path, history: &History) -> std::io::Result<()> {
    let path = target_dir.join(UNDO_FILE_NAME);

    if history.entries.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let json = serde_json::to_string_pretty(history)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    fs::write(path, json)
}

/// Works out what undoing the last flattening of `target_dir` would do.
///
/// Returns `None` when there is nothing to undo.
pub fn plan_undo(target_dir: &Path) -> std::io::Result<Option<UndoPlan>> {
    let history = read_history(target_dir)?;

    let entry = match history.entries.last() {
        Some(entry) => entry,
        None => return Ok(None),
    };

    Ok(Some(plan_from_entry(entry)?))
}

/// Reverses the last recorded flattening of `target_dir`.
///
/// Files are moved back to where they came from and the removed directories
/// are recreated. The entry is dropped from the history afterwards.
pub fn undo_directory(target_dir: &Path) -> std::io::Result<Option<HistoryEntry>> {
    let mut history = read_history(target_dir)?;

    let entry = match history.entries.pop() {
        Some(entry) => entry,
        None => return Ok(None),
    };

    let plan = plan_from_entry(&entry)?;

    apply_undo_plan(&plan)?;
    write_history(target_dir, &history)?;

    Ok(Some(entry))
}

/// Builds the undo plan for a recorded entry.
fn plan_from_entry(entry: &HistoryEntry) -> std::io::Result<UndoPlan> {
    let moves = entry
        .moves
        .iter()
        .rev()
        .map(|recorded| PlannedMove {
            from: recorded.to.clone(),
            to: recorded.from.clone(),
        })
        .collect();

    let dirs_to_create = entry.removed_dirs.iter().rev().cloned().collect();

    Ok(UndoPlan {
        moves,
        dirs_to_create,
    })
}

/// Applies an undo plan, moving files back and recreating directories.
fn apply_undo_plan(plan: &UndoPlan) -> std::io::Result<()> {
    // Check everything before touching the filesystem, so a failure part way
    // through does not leave the directory half restored.
    for planned in &plan.moves {
        if !planned.from.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot undo: {} is missing.", planned.from.display()),
            ));
        }

        if planned.to.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Cannot undo: {} already exists.", planned.to.display()),
            ));
        }
    }

    for dir in &plan.dirs_to_create {
        fs::create_dir_all(dir)?;
    }

    // Undo in reverse order so a file never overwrites one already restored.
    for planned in &plan.moves {
        fs::rename(&planned.from, &planned.to)?;
    }

    Ok(())
}

/// Current time as seconds since the Unix epoch.
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
