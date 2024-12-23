
# Gustasum: Partial Checksumming Done Right! 🚀

Ever wanted a checksum tool that’s **smart**, **blazing fast**, and **designed with real-world usability in mind**? Look no further! Gustasum combines the **speed of partial checksumming** with a suite of features that make file integrity verification a breeze. Gustasum is your go-to utility for **partial checksumming**, a perfect balance between speed and reliability.

In an age of terabyte drives and colossal files, traditional full checksums can be *overkill*. Enter **partial checksumming**, the art of verifying **just enough** of a file to confidently ensure its integrity.

With Gustasum, you can:
- Verify large datasets **quickly** without reading entire files.
- Track file changes, validate backups, and confirm copy operations—all with speed and efficiency.
- Customize behavior to suit your workflows, from tweaking chunk sizes to remapping base paths during validation.

---

## Key Features 🔑

- **Smart Checksumming**: Reads the **first**, **middle**, and **last** `100` bytes of a file for rapid verification.
- **Flexible Validation**: Validate files using a checksum file and optional base path remapping.
- **Progress Feedback**: Track your operations with stylish progress bars (automatically hidden in scripts).
- **Error Handling**: Skip files with errors or halt the process, your choice!
- **Customizable**: Adjust chunk size, include modification time in hashes, or keep things lean with defaults.
- **Modern**: Built in **Rust** for performance and reliability.

---

## Real-World Applications 🌎

### 1. File Copy Validation
Copied a huge folder? Use Gustasum to confirm everything copied intact:
```bash
gustasum /source/directory > source_checksums.txt
gustasum --check source_checksums.txt --remap /source /destination
```

### 2. Backup Integrity
Backups are critical, but are they reliable? Use Gustasum to ensure data hasn’t changed over time:
```bash
gustasum /backup/directory > backup_checksums.txt
gustasum --check backup_checksums.txt
```

### 3. Deduplication
Find duplicate files efficiently by comparing partial checksums.

### 4. Quick Verifications
Need a sanity check but don’t want to wait hours for a full checksum? Gustasum’s partial checksumming delivers confidence in seconds.

---

## Installation

### From Source
1. Clone the repository:
   ```bash
   git clone https://github.com/ghagl/gustasum.git
   cd gustasum
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run Gustasum:
   ```bash
   ./target/release/gustasum
   ```

---

## Usage Examples

### 1. Generate Partial Checksums
Generate checksums for all files in a directory:
```bash
gustasum /path/to/directory > partial_checksums.txt
```

### 2. Validate Checksums
Validate a directory against previously generated checksums:
```bash
gustasum --check partial_checksums.txt
```

### 3. Handle Path Changes
Validate files where the directory structure has changed:
```bash
gustasum --check partial_checksums.txt --remap /old/base/path /new/base/path
```

### 4. Include Modification Time
If you’ve used tools like `cp -p` to preserve file modification times, include modtime in your hashes:
```bash
gustasum --include-modtime /path/to/directory > partial_checksums_with_modtime.txt
gustasum --check partial_checksums_with_modtime.txt --include-modtime
```

### 5. Customize Chunk Sizes
Increase or decrease the bytes read from the file’s start, middle, and end:
```bash
gustasum --partial-bytes 256 /path/to/directory > custom_checksums.txt
```

---

## Command Overview

### Basic Commands
- **Generate Checksums**: `gustasum /path/to/files > checksums.txt`
- **Validate Checksums**: `gustasum --check checksums.txt`

### Options
- `--partial-bytes <N>`: Number of bytes to read from start, middle, and end of files (default: 100).
- `--include-modtime`: Include modification time in hashes.
- `--remap <OLD_BASE> <NEW_BASE>`: Adjust file paths during validation.
- `--skip-errors`: Skip files that produce errors during reading or metadata access.
- `--check <FILE>`: Validate files against a checksum file.

### Examples
For more examples, run:
```bash
gustasum --help
```

---

## License

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**. See [LICENSE](LICENSE) for details.

---

## Contributing

Gustasum is a personal project, but contributions are always welcome! If you encounter bugs, have feature requests, or want to make Gustasum even better, feel free to:
1. Open an issue.
2. Submit a pull request.
3. Share your feedback.

---

## Why Gustasum?

Because life’s too short to wait for full checksums. Gustasum is **fast**, **reliable**, and designed for **you**, the pragmatic power user. Whether you’re managing backups, verifying file copies, or tackling large datasets, Gustasum makes it *fun* to care about file integrity.

So go ahead—try Gustasum today, and bring speed and confidence to your file operations! 🚀
