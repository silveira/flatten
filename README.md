# flatten
flatten a folder so it is just files without subfolders

## Before running flatten
```
myfolder/
├── fruits/
│   ├── banana.png
│   └── apple.png
├── animals/
│   ├── cats/
│   │   └── tobby.jpg
│   └─── lola.zip
└── phone.txt
```

## After running flatten

```
myfolder/
├─ banana.png
├─ apple.png
├─ tobby.jpg
├─ lola.zip
└─ phone.txt
```

`flatten` is a cross-platform CLI tool that extracts all files from nested subfolders and moves them to the main directory's root. It safely processes files with spaces or special characters, automatically renames files to prevent overwriting during collisions, and deletes the empty subdirectories left behind. Every run can be previewed with `--dryrun` and reversed with `--undo`.

## Usage

```
cargo build --release
./target/release/flatten myfolder
```

### Dry run

Use `--dryrun` (or `-n`) to preview the changes without touching anything on
disk. It prints where each file currently is and where it would be moved to:

```
$ ./target/release/flatten --dryrun myfolder

Dry run: no files were moved.

Moves:
  animals/cats/tobby.jpg -> tobby.jpg
  animals/lola.zip -> lola.zip
  fruits/banana.png -> banana.png
  fruits/apple.png -> apple.png

Directories to remove:
  animals/cats
  animals
  fruits

4 file(s) would be moved, 3 directory(ies) would be removed.
```

### Undo

Every flatten records what it did in a hidden `.flatten_history.json` file
inside the target directory. Use `--undo` to reverse the most recent flatten,
putting every file back where it came from and recreating the directories that
were removed:

```
$ ./target/release/flatten --undo myfolder
Undo complete!
```

The history keeps a stack of operations, so `--undo` can be repeated to step
back through several flattens, one at a time. Once there is nothing left to
undo the history file is removed and the directory is back to its original
state.

`--undo` can be combined with `--dryrun` to preview what would be restored:

```
$ ./target/release/flatten --undo --dryrun myfolder

Dry run: no files were moved back.

Moves back:
  apple.png -> fruits/apple.png
  banana.png -> fruits/banana.png
  lola.zip -> animals/lola.zip
  tobby.jpg -> animals/cats/tobby.jpg

Directories to recreate:
  fruits
  animals
  animals/cats

4 file(s) would be moved back, 3 directory(ies) would be recreated.
```

Undo refuses to run if a file it needs to move back is missing, or if
something already occupies the original path, so nothing is ever overwritten.

## Testing

```
cargo test
```
