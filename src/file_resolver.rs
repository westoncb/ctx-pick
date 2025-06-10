// src/file_resolver.rs

use crate::config::Config;
use crate::types::{InputResolution, ResolvedFile};
use glob::glob; // Import the glob function
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

// Helper to check if a WalkDir entry is a file. (Unchanged)
fn is_walkdir_file_entry(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
}

/// Attempts to create a ResolvedFile instance from a given path. (Unchanged)
fn create_resolved_file(path_to_resolve: &Path, config: &Config) -> Result<ResolvedFile, String> {
    let canonical_path = fs::canonicalize(path_to_resolve)
        .map_err(|e| format!("Failed to canonicalize path {:?}: {}", path_to_resolve, e))?;

    let display_path = pathdiff::diff_paths(&canonical_path, &config.working_dir)
        .unwrap_or_else(|| canonical_path.clone());

    Ok(ResolvedFile::new(display_path, canonical_path))
}

/// Resolves a single input string into an `InputResolution` outcome.
///
/// This function now uses a three-phase resolution strategy:
/// 1. Direct Match: Checks if the input is a literal, existing file or directory.
/// 2. Glob Match: If not a direct match, checks if the input is a valid glob pattern.
/// 3. Fuzzy Search: If neither of the above, falls back to a recursive fuzzy search.
pub fn resolve_input_string<'a>(input_str: &'a str, config: &Config) -> InputResolution<'a> {
    // --- Phase 1: Direct Match ---
    // First, check if the input string is a literal path to an existing file or directory.
    // This ensures that filenames containing glob characters (e.g., "file[1].txt") are
    // found correctly if they exist.
    let path_to_check = config.working_dir.join(input_str);
    if path_to_check.exists() {
        if path_to_check.is_file() {
            return match create_resolved_file(&path_to_check, config) {
                Ok(resolved) => InputResolution::Success(vec![resolved]),
                Err(err_msg) => {
                    eprintln!(
                        "Warning: Found explicit file '{}' but could not process it: {}",
                        input_str, err_msg
                    );
                    // Treat processing failure as if it wasn't found.
                    InputResolution::NotFound {
                        input_string: input_str,
                    }
                }
            };
        } else if path_to_check.is_dir() {
            // Expand the directory and collect all files within it.
            let files_in_dir: Vec<ResolvedFile> = WalkDir::new(&path_to_check)
                .min_depth(1)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok()) // Ignore walk errors (e.g., permissions)
                .filter(|e| e.file_type().is_file())
                .filter_map(|entry| match create_resolved_file(entry.path(), config) {
                    Ok(resolved) => Some(resolved),
                    Err(err_msg) => {
                        eprintln!(
                            "Warning: Could not process file {:?} in directory '{}': {}",
                            entry.path(),
                            input_str,
                            err_msg
                        );
                        None
                    }
                })
                .collect();
            return InputResolution::Success(files_in_dir);
        }
    }

    // --- Phase 2: Glob Pattern Match ---
    // If it's not a direct path, check if it looks like a glob pattern.
    let is_glob_pattern = input_str.contains(&['*', '?', '[', '{'][..]);
    if is_glob_pattern {
        return match glob(input_str) {
            Err(pattern_error) => {
                // The glob pattern itself is invalid.
                InputResolution::InvalidGlobPattern {
                    input_string: input_str,
                    error: pattern_error.to_string(),
                }
            }
            Ok(paths) => {
                // The glob pattern is valid; now resolve the matched paths.
                let mut resolved_files: Vec<ResolvedFile> = Vec::new();
                for entry in paths {
                    match entry {
                        Ok(path) => {
                            if path.is_file() {
                                match create_resolved_file(&path, config) {
                                    Ok(resolved) => resolved_files.push(resolved),
                                    Err(err_msg) => {
                                        eprintln!(
                                            "Warning: Glob matched file {:?} but could not process it: {}",
                                            path, err_msg
                                        );
                                    }
                                }
                            }
                        }
                        Err(glob_error) => {
                            eprintln!(
                                "Warning: Error while processing glob match for '{}': {}",
                                input_str, glob_error
                            );
                        }
                    }
                }

                if resolved_files.is_empty() {
                    // Valid glob, but it matched no files.
                    InputResolution::NotFound {
                        input_string: input_str,
                    }
                } else {
                    // Glob successfully matched one or more files. This is not an ambiguity.
                    InputResolution::Success(resolved_files)
                }
            }
        };
    }

    // --- Phase 3: Fuzzy Search (Fallback) ---
    // If it's not a direct path or a glob, perform a recursive search for a partial match.
    let mut candidate_paths: Vec<PathBuf> = Vec::new();
    let walker = WalkDir::new(&config.working_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_walkdir_file_entry(e));

    for entry in walker {
        let entry_path = entry.path();
        let relative_path = pathdiff::diff_paths(entry_path, &config.working_dir)
            .unwrap_or_else(|| entry_path.to_path_buf());

        // Match if the relative path contains the input string.
        if relative_path.to_string_lossy().contains(input_str) {
            candidate_paths.push(entry.into_path());
        }
    }

    candidate_paths.sort();
    candidate_paths.dedup();

    match candidate_paths.len() {
        0 => {
            // No fuzzy matches found. Distinguish between a bad path and a simple not-found.
            if input_str.contains(std::path::MAIN_SEPARATOR) {
                InputResolution::PathDoesNotExist {
                    input_string: input_str,
                    path_tried: config.working_dir.join(input_str),
                }
            } else {
                InputResolution::NotFound {
                    input_string: input_str,
                }
            }
        }
        1 => {
            // Exactly one fuzzy match found.
            match create_resolved_file(&candidate_paths[0], config) {
                Ok(resolved) => InputResolution::Success(vec![resolved]),
                Err(err_msg) => {
                    eprintln!(
                        "Warning: Found unique match for '{}' but failed to process it: {}",
                        input_str, err_msg
                    );
                    InputResolution::NotFound {
                        input_string: input_str,
                    }
                }
            }
        }
        _ => {
            // Multiple fuzzy matches found, which is an ambiguity.
            let conflicting_display_paths: Vec<PathBuf> = candidate_paths
                .iter()
                .map(|p| pathdiff::diff_paths(p, &config.working_dir).unwrap_or_else(|| p.clone()))
                .collect();

            InputResolution::Ambiguous {
                input_string: input_str,
                conflicting_paths: conflicting_display_paths,
            }
        }
    }
}
