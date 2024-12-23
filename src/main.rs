/*
 * Gustasum *
 Partial Checksumming Done Right!

 Copyright (C) 2024 Gustaf Haglund <contact@ghagl.se>

 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.

 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use clap::{Arg, ArgAction, Command};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// For progress bar + TTY detection
use atty::Stream;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

#[allow(non_snake_case)]
fn main() {
    let matches = Command::new("gustasum")
        .version("0.1.0")
        .about("Generate/check partial checksums")
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .help("Read checksums from the specified file and verify them")
                .value_name("FILE")
                .num_args(1)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("remap")
                .long("remap")
                .help("Remaps old base path to new base path during verification. \
                       E.g., --remap OLD_BASE NEW_BASE")
                .num_args(2)
                .value_names(["OLD_BASE", "NEW_BASE"])
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("skip_errors")
                .long("skip-errors")
                .help("Skip files that produce read/metadata errors instead of marking them as FAILED")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("partial_bytes")
                .long("partial-bytes")
                .help("Number of bytes to read from start, middle, and end")
                .value_name("N")
                .num_args(1)
                .default_value("100")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("include_modtime")
                .long("include-modtime")
                .help("By default, modtime is NOT hashed. Use this flag if you explicitly want to include modtime.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("paths")
                .help("Paths to process (directories/files)")
                .num_args(1..)
                .action(ArgAction::Append)
                .required_unless_present("check"),
        )
        .after_help(
            "EXAMPLES:\n\
             1) Generate partial sums (NO modtime):\n\
                gustasum some_directory > partialsums.txt\n\n\
             2) Verify partial sums:\n\
                gustasum --check partialsums.txt\n\n\
             3) Remap old base to new base:\n\
                gustasum --check partialsums.txt --remap /old/path /new/path\n\n\
             4) If you used cp -p / cp -a (preserving modtime), add:\n\
                gustasum --include-modtime some_directory > partialsums.txt\n\
                gustasum --check partialsums.txt --include-modtime\n\n\
             NOTE:\n\
             - We skip creation time (birth time). If modtime isn't preserved (vanilla cp), you can rely solely on Gustasum's default setting."
        )
        .get_matches();

    let skip_errors = matches.get_flag("skip_errors");
    let remap_args = matches.get_many::<String>("remap");
    let (old_base, new_base) = match remap_args {
        Some(vals) => {
            let vec: Vec<String> = vals.map(|s| s.to_string()).collect();
            if vec.len() == 2 {
                (Some(PathBuf::from(&vec[0])), Some(PathBuf::from(&vec[1])))
            } else {
                (None, None)
            }
        }
        None => (None, None),
    };

    let partial_bytes_str = matches.get_one::<String>("partial_bytes").unwrap();
    let partial_bytes = partial_bytes_str.parse::<usize>().unwrap_or(100);

    // By default, we do NOT include modtime. If --include-modtime is set, we include it.
    let include_modtime = matches.get_flag("include_modtime");

    // Show progress if stderr is a TTY
    let show_progress = atty::is(Stream::Stderr);

    if let Some(check_file) = matches.get_one::<String>("check") {
        verify_mode(
            check_file,
            skip_errors,
            old_base,
            new_base,
            show_progress,
            partial_bytes,
            include_modtime,
        );
    } else if let Some(paths) = matches.get_many::<String>("paths") {
        let path_vec: Vec<PathBuf> = paths.map(PathBuf::from).collect();
        generate_mode(
            &path_vec,
            skip_errors,
            show_progress,
            partial_bytes,
            include_modtime,
        );
    } else {
        eprintln!("No paths provided and no check file specified. Use --help for usage.");
        std::process::exit(1);
    }
}

/// Generate checksums for all files in the given paths, ignoring modtime by default.
/// Use `include_modtime = true` if the user provided --include-modtime.
fn generate_mode(
    paths: &[PathBuf],
    skip_errors: bool,
    show_progress: bool,
    partial_bytes: usize,
    include_modtime: bool,
) {
    let files: Vec<PathBuf> = paths
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        .flat_map(|p| {
            WalkDir::new(p)
                .follow_links(false)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
        })
        .collect();

    let total_files = files.len();
    eprintln!(
        "Found {} files. Computing partial checksums...",
        total_files
    );

    let pb = if show_progress {
        let bar = ProgressBar::new(total_files as u64);
        bar.set_draw_target(ProgressDrawTarget::stderr());
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} files ({eta} remaining)",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        Some(bar)
    } else {
        None
    };

    let mut results = Vec::with_capacity(total_files);

    results.par_extend(files.par_iter().map(|path| {
        let hash_result = compute_hash_for_file(path, partial_bytes, include_modtime);
        if let Some(ref bar) = pb {
            bar.inc(1);
        }
        (path.clone(), hash_result)
    }));

    if let Some(ref bar) = pb {
        bar.finish_and_clear();
    }

    let mut successes = 0usize;
    let mut failures = 0usize;

    for (path, result) in results {
        match result {
            Ok(hash) => {
                // output to stdout
                println!("{}  {}", hash, path.display());
                successes += 1;
            }
            Err(e) => {
                if skip_errors {
                    eprintln!("Warning: Skipping file '{}': {}", path.display(), e);
                } else {
                    eprintln!("Error: Could not process file '{}': {}", path.display(), e);
                }
                failures += 1;
            }
        }
    }

    eprintln!(
        "\nSummary: total files = {}, succeeded = {}, errors = {}",
        total_files, successes, failures
    );

    if failures > 0 && !skip_errors {
        std::process::exit(1);
    }
}

/// Verify checksums from `--check`, with optional path remapping & modtime usage.
#[allow(non_snake_case)]
fn verify_mode(
    check_file: &str,
    skip_errors: bool,
    old_base: Option<PathBuf>,
    new_base: Option<PathBuf>,
    show_progress: bool,
    partial_bytes: usize,
    include_modtime: bool,
) {
    let contents = match fs::read_to_string(check_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read check file '{}': {}", check_file, e);
            std::process::exit(1);
        }
    };

    let lines: Vec<&str> = contents
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let total_lines = lines.len();
    eprintln!("Found {} checks to perform. Verifying...", total_lines);

    let pb = if show_progress {
        let bar = ProgressBar::new(total_lines as u64);
        bar.set_draw_target(ProgressDrawTarget::stderr());
        bar.set_style(
            ProgressStyle::with_template(
                "{spinner} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} lines ({eta} remaining)",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        Some(bar)
    } else {
        None
    };

    let mut results = Vec::with_capacity(total_lines);
    results.par_extend(lines.par_iter().map(|line| {
        let (expected_hash, file_str) = match split_line(line) {
            Some(x) => x,
            None => {
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
                return (
                    "".to_string(),
                    line.to_string(),
                    Err("Malformed line".to_string()),
                );
            }
        };

        let original_path = PathBuf::from(&file_str);
        let remapped = match (&old_base, &new_base) {
            (Some(ob), Some(nb)) => remap_path(&original_path, ob, nb),
            _ => original_path.clone(),
        };

        let hash_result = compute_hash_for_file(&remapped, partial_bytes, include_modtime);

        if let Some(ref bar) = pb {
            bar.inc(1);
        }

        (expected_hash, file_str.to_string(), hash_result)
    }));

    if let Some(ref bar) = pb {
        bar.finish_and_clear();
    }

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;

    for (expected, original_path, actual_res) in results {
        match actual_res {
            Ok(actual_hash) => {
                if actual_hash == expected {
                    println!("{}: OK", original_path);
                    ok_count += 1;
                } else {
                    eprintln!("{}: FAILED (mismatch)", original_path);
                    fail_count += 1;
                }
            }
            Err(e) => {
                fail_count += 1;
                if skip_errors {
                    eprintln!("Warning: Skipping file '{}': {}", original_path, e);
                } else {
                    eprintln!("{}: FAILED to compute hash ({})", original_path, e);
                }
            }
        }
    }

    eprintln!(
        "\nSummary: total checks = {}, OK = {}, FAILED = {}",
        total_lines, ok_count, fail_count
    );

    if fail_count > 0 && !skip_errors {
        std::process::exit(1);
    }
}

/// Split a line "<hash>  <path>" into (hash, path).
fn split_line(line: &str) -> Option<(String, String)> {
    if let Some(idx) = line.find("  ") {
        let (hash, path) = line.split_at(idx);
        let path = &path[2..];
        Some((hash.to_string(), path.to_string()))
    } else {
        None
    }
}

/// Remap path if it starts with `old_base`.
fn remap_path(original: &Path, old_base: &Path, new_base: &Path) -> PathBuf {
    if original.starts_with(old_base) {
        if let Ok(stripped) = original.strip_prefix(old_base) {
            return new_base.join(stripped);
        }
    }
    original.to_path_buf()
}

/// The number of times to retry on a read error (e.g., flakey HDD).
const READ_RETRIES: usize = 2;

/// Compute partial file hash. By default, we skip modtime. If `include_modtime` is true, we add modtime.
fn compute_hash_for_file(
    path: &Path,
    partial_bytes: usize,
    include_modtime: bool,
) -> Result<String, String> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        let res = do_compute_hash_for_file(path, partial_bytes, include_modtime);
        match res {
            Ok(h) => return Ok(h),
            Err(e) => {
                if attempts <= READ_RETRIES && is_transient_read_error(&e) {
                    eprintln!("Retrying file '{}': {}", path.display(), e);
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn do_compute_hash_for_file(
    path: &Path,
    partial_bytes: usize,
    include_modtime: bool,
) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| format!("metadata error: {}", e))?;
    let size = meta.len();

    // We never include creation time on Linux, it's too unreliable.

    // If user wants to include modtime and it's available, hash it. Otherwise, set to 0.
    let mod_time_secs = if include_modtime {
        meta.modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    } else {
        0
    };

    // File reading
    let file = fs::File::open(path).map_err(|e| format!("file open error: {}", e))?;
    let mut reader = BufReader::new(file);

    let mut first_buf = vec![0u8; partial_bytes];
    let mut middle_buf = vec![0u8; partial_bytes];
    let mut last_buf = vec![0u8; partial_bytes];

    // First
    let first_len = reader
        .read(&mut first_buf)
        .map_err(|e| format!("read error (first bytes): {}", e))?;
    first_buf.truncate(first_len);

    // Middle
    if size > (partial_bytes as u64 * 2) {
        let mid_offset = size / 2;
        reader
            .seek(SeekFrom::Start(mid_offset))
            .map_err(|e| format!("seek error (middle): {}", e))?;
        let middle_len = reader
            .read(&mut middle_buf)
            .map_err(|e| format!("read error (middle bytes): {}", e))?;
        middle_buf.truncate(middle_len);
    } else {
        middle_buf.clear();
    }

    // Last
    if size > partial_bytes as u64 {
        let end_offset = size.saturating_sub(partial_bytes as u64);
        reader
            .seek(SeekFrom::Start(end_offset))
            .map_err(|e| format!("seek error (end): {}", e))?;
        let last_len = reader
            .read(&mut last_buf)
            .map_err(|e| format!("read error (last bytes): {}", e))?;
        last_buf.truncate(last_len);
    } else {
        last_buf.clear();
    }

    // Combine data
    let mut hasher = Sha256::new();

    // Possibly zero or actual mod time
    hasher.update(mod_time_secs.to_le_bytes());

    // file size
    hasher.update(size.to_le_bytes());

    // partial contents
    hasher.update(&first_buf);
    hasher.update(&middle_buf);
    hasher.update(&last_buf);

    let final_hash = hasher.finalize();
    Ok(format!("{:x}", final_hash))
}

/// Check if an error is possibly transient (e.g., read error from failing HDD).
fn is_transient_read_error(err: &str) -> bool {
    err.contains("read error") || err.contains("I/O error") || err.contains("EIO")
}
