# flatten
flatten a folder so it is just files without subfolders

## Before running flatten
```
folder/
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
folder/
├─ banana.png
├─ apple.png
├─ tobby.jpg
├─ lola.zip
└─ phone.txt
```

`flatten` is a cross-platform CLI tool that extracts all files from nested subfolders and moves them to the main directory's root. It safely processes files with spaces or special characters, automatically renames files to prevent overwriting during collisions, and deletes the empty subdirectories left behind.
