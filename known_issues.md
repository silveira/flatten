# Bug report

Findings from a review of the codebase. Every item below was reproduced against
`target/release/flatten`; none of them are caught by the existing test suite.

## 1. `--undo` can operate on the wrong directory (data integrity)

**Severity:** high

`HistoryEntry.moves` records **absolute** paths (`src/lib.rs:160`), and
`read_history` (`src/lib.rs:187`) never checks the stored `root` against the
directory passed on the command line. `plan_from_entry` (`src/lib.rs:260`) then
feeds those absolute paths straight into `fs::rename`.

Consequences:

- Flatten `/tmp/a`, move it to `/tmp/b`, then run `--undo /tmp/b`:

  ```
  Error: Custom { kind: NotFound, error: "Cannot undo: /tmp/a/file.txt is missing." }
  ```

  even though the file is sitting in `/tmp/b`.

- Worse, if `/tmp/a` exists again (copied or recreated), `--undo /tmp/b` silently
  modifies `/tmp/a` and then deletes `/tmp/b`'s history file:

  ```
  $ flatten dirA
  $ cp -r dirA dirB
  $ flatten --undo dirB        # prints "Undo complete!"
  ```

  Result: `dirA/sub/a.txt` is restored, `dirB` is untouched, and
  `dirB/.flatten_history.json` is deleted.

A copied or stale history can therefore move files **outside** the directory the
user asked to undo.

**Suggested fix:** store `from`/`to` relative to `root`, resolve them against the
*current* target directory on load, and reject any resolved path that escapes it.
Validate `root` and `version` when reading the file.

## 2. A user file named `.flatten_history.json` is destroyed

**Severity:** high

`plan_flatten` moves `sub/.flatten_history.json` up to the root, then
`write_history` (`src/lib.rs:209`) overwrites that exact path with the new
history.

```
$ echo "USER DATA" > nametest/sub/.flatten_history.json
$ flatten nametest
$ grep "USER DATA" nametest/.flatten_history.json
# no match: the file was overwritten
```

Related: if a non-JSON file already exists at the root with that name, flatten
aborts before doing anything with a cryptic error:

```
Error: Custom { kind: InvalidData, error: Error("expected ident", line: 1, column: 2) }
```

**Suggested fix:** exclude the history file from the flatten plan, write it
before applying the moves, or use a name that cannot collide with a user file.

## 3. Partial flatten is never recorded

**Severity:** medium

`flatten_directory_recording` (`src/lib.rs:160`) applies the whole plan first and
writes the history afterwards. If any `fs::rename` fails partway through — or the
history write itself fails, for example because the directory is read-only — the
function returns `Err` and every move that already succeeded is left unrecorded
and therefore unundoable.

A read-only target produces this after partial work:

```
Error: Os { code: 13, kind: PermissionDenied, message: "Permission denied" }
```

**Suggested fix:** record the entry before applying, or record moves
incrementally, and roll back on failure.

## 4. Symlinks are mishandled

**Severity:** medium

Three separate problems:

1. **Relative symlinks break when moved**, because they are resolved from a new
   directory:

   ```
   sub/link.txt -> deep/real.txt     # resolves while inside sub/
   # after flatten: root/link.txt -> deep/real.txt   (dangling)
   ```

2. **Symlinks pointing inside the tree break too**, because the target is also
   moved elsewhere.

3. **Broken symlinks and symlinks to directories are neither moved nor
   removed.** `is_file()` is false for both, so they are skipped, and
   `remove_dir` on a symlink fails; that error is swallowed by
   `let _ = fs::remove_dir(dir)` (`src/lib.rs:150`). The now-"empty" directory is
   silently left behind:

   ```
   after flatten: sub/{broken,linkdir} remain, sub/ not removed
   ```

**Suggested fix:** inspect entries with `symlink_metadata`, handle symlinks
explicitly instead of following them in the `is_file()`/`is_dir()` checks, and
report directory-removal failures instead of ignoring them.

## 5. Case-insensitive filesystems can lose files (macOS/Windows)

**Severity:** medium

`taken` is a case-sensitive `HashSet<PathBuf>` (`src/lib.rs:101`). Two nested
files named `Data.txt` and `data.txt` are planned to distinct root names; on a
case-insensitive filesystem the second `rename` replaces the first, silently
destroying it. The release matrix ships macOS and Windows binaries, so this is
reachable.

**Suggested fix:** compare planned destinations case-insensitively when the
target filesystem is case-insensitive, or verify existence again immediately
before each rename.

## 6. Poor error output

**Severity:** low

`main` returns `io::Result<()>` (`src/main.rs:23`), so runtime failures are
printed with `Debug` formatting:

```
Error: Custom { kind: NotFound, error: "Cannot undo: /tmp/x/a.txt is missing." }
```

**Suggested fix:** print `Error: {message}` from a small wrapper and exit with a
stable non-zero code.

## 7. `--undo --dryrun` does not validate feasibility

**Severity:** low

With a file deleted after flattening, the dry run happily reports a plan:

```
$ flatten --undo --dryrun drtest
Dry run: no files were moved back.
Moves back:
  a.txt -> sub/a.txt
...
1 file(s) would be moved back, 1 directory(ies) would be recreated.
```

but the real run fails immediately. The dry run should apply the same
pre-checks as `apply_undo_plan`.

## 8. History metadata is unused

**Severity:** low

- `History.version` is never validated on read, so a future format change will
  be interpreted as if it were version 1.
- `History.root` is written but never used (root cause of item 1).
- `plan_undo` and `undo_directory` each read the history file, so there is a
  TOCTOU window between the check and the undo.

## 9. Dangling symlink at a destination is silently replaced

**Severity:** low

`Path::exists()` follows symlinks, so a broken symlink at a planned destination
reports `false`. The plan treats the name as free and `fs::rename` replaces the
symlink. Use `symlink_metadata().is_ok()` to test for existence without following
links.
