use crate::config::Config;
use crate::types::{InputResolution, ResolvedFile}; // Assuming types.rs is in crate::types
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

// Helper to check if a WalkDir entry is a file (and not a directory or symlink to a dir).
// We generally want to resolve symlinks to files, but WalkDir's default is to
// not follow symlinks for file paths unless configured. is_file() on DirEntry
// checks the metadata of the entry itself, which for a symlink refers to the symlink.
// To check the target, we might need to canonicalize or use entry.metadata()?.is_file().
// For now, let's keep it simple and assume WalkDir gives us usable file entries,
// or we handle symlinks primarily through canonicalization later.
// A simpler is_file for initial WalkDir filtering:
fn is_walkdir_file_entry(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
}

/// Attempts to create a ResolvedFile instance from a given path.
/// This involves canonicalizing the path and creating a display-friendly version.
///
/// Errors here are strings meant for internal consumption by `resolve_input_string`
/// to be converted into an appropriate `InputResolution` variant.
fn create_resolved_file(path_to_resolve: &Path, config: &Config) -> Result<ResolvedFile, String> {
    let canonical_path = fs::canonicalize(path_to_resolve)
        .map_err(|e| format!("Failed to canonicalize path {:?}: {}", path_to_resolve, e))?;

    // Create a display path relative to the working directory.
    // Using pathdiff as it can create ../ sibling paths if necessary.
    let display_path = pathdiff::diff_paths(&canonical_path, &config.working_dir)
        .unwrap_or_else(|| canonical_path.clone()); // Fallback to canonical if diffing fails

    Ok(ResolvedFile::new(display_path, canonical_path))
}

/// Resolves a single input string (which could be a path, a partial filename, or a directory name)
/// into an `InputResolution` outcome.
///
/// This function is designed to be "infallible" in that it always returns an `InputResolution`
/// variant, transforming internal errors (like IO errors during resolution of a *specific* item)
/// into an appropriate problem-describing variant of `InputResolution`.
pub fn resolve_input_string<'a>(input_str: &'a str, config: &Config) -> InputResolution<'a> {
    let input_path = PathBuf::from(input_str);

    // Step 1: Check if input_str is an existing path (absolute or relative to PWD)
    let path_to_check = if input_path.is_absolute() {
        input_path.clone()
    } else {
        config.working_dir.join(&input_path)
    };

    if path_to_check.exists() {
        if path_to_check.is_file() {
            return match create_resolved_file(&path_to_check, config) {
                Ok(resolved) => InputResolution::Success(vec![resolved]),
                Err(err_msg) => {
                    // If we found an explicit path but couldn't process it (e.g., canonicalize failed)
                    // Treat as NotFound for simplicity, or introduce a new InputResolution variant.
                    eprintln!(
                        "Warning: Could not process explicitly provided file path '{}': {}",
                        input_str, err_msg
                    );
                    InputResolution::NotFound {
                        input_string: input_str,
                    }
                }
            };
        } else if path_to_check.is_dir() {
            let mut files_in_dir: Vec<ResolvedFile> = Vec::new();
            // WalkDir setup for directory input
            for entry_result in WalkDir::new(&path_to_check)
                .min_depth(1) // Skip the directory itself, only contents
                .follow_links(true) // Follow links when resolving a directory
                .sort_by_file_name()
            // Ensure consistent order
            {
                match entry_result {
                    Ok(entry) => {
                        if entry.file_type().is_file() {
                            // Check if the resolved entry is a file
                            match create_resolved_file(entry.path(), config) {
                                Ok(resolved) => files_in_dir.push(resolved),
                                Err(err_msg) => {
                                    eprintln!(
                                        "Warning: Could not process file {:?} in directory '{}': {}",
                                        entry.path(),
                                        input_str,
                                        err_msg
                                    );
                                }
                            }
                        }
                    }
                    Err(walk_err) => {
                        eprintln!(
                            "Warning: Error walking directory '{}' for input '{}': {}",
                            path_to_check.display(),
                            input_str,
                            walk_err
                        );
                        // Optionally, could return a more specific error for the directory input itself
                    }
                }
            }
            // Even if some files failed, return success with the ones that worked.
            // If files_in_dir is empty, it's an empty directory or all files failed processing.
            return InputResolution::Success(files_in_dir);
        }
    } else if input_str.contains(std::path::MAIN_SEPARATOR) || input_path.components().count() > 1 {
        // Step 2: If it looked like a path (e.g., had separators) but didn't exist.
        // `components().count() > 1` handles cases like "foo/bar" even if MAIN_SEPARATOR isn't used.
        return InputResolution::PathDoesNotExist {
            input_string: input_str,
            path_tried: path_to_check, // The fully resolved path we attempted
        };
    }

    // Step 3: If not an existing path and didn't look like a non-existent specific path,
    // perform a recursive search from config.working_dir for a partial/exact filename.
    let mut exact_filename_matches: Vec<PathBuf> = Vec::new();
    let mut partial_filename_matches: Vec<PathBuf> = Vec::new();

    for entry_result in WalkDir::new(&config.working_dir)
        .follow_links(true) // Follow links during general search too
        .into_iter()
    {
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                // As per secondary analysis, handle WalkDir errors more gracefully.
                // Log non-critical errors and continue.
                // PermissionDenied is common, could be skipped silently or with a debug log.
                if e.io_error().map(|io_err| io_err.kind())
                    == Some(std::io::ErrorKind::PermissionDenied)
                {
                    // Optionally, log at a trace/debug level if verbose logging is added later
                    // eprintln!("Debug: Permission denied accessing {:?}, skipping.", e.path());
                } else {
                    // walkdir::Error implements Display, which provides a good default message.
                    // You can also access e.path() for the path, and e.io_error() for the underlying IO error if needed.
                    eprintln!(
                        "Warning: Error during file search at {:?}: {}",
                        e.path().unwrap_or_else(|| Path::new("<unknown path>")),
                        e
                    );
                }
                continue;
            }
        };

        if !is_walkdir_file_entry(&entry) {
            // Use our helper to filter for files
            continue;
        }

        let file_path = entry.path();
        if let Some(file_name_osstr) = file_path.file_name() {
            let file_name_str = file_name_osstr.to_string_lossy();

            if file_name_str == input_str {
                exact_filename_matches.push(file_path.to_path_buf());
            }
            // Only consider partial if it's not an exact match to avoid double-adding.
            // And ensure input_str is not empty for contains, though clap should prevent empty inputs.
            else if !input_str.is_empty() && file_name_str.contains(input_str) {
                partial_filename_matches.push(file_path.to_path_buf());
            }
        }
    }

    // Consolidate matches: prioritize exact_filename_matches
    let mut final_candidate_paths = if !exact_filename_matches.is_empty() {
        exact_filename_matches
    } else {
        partial_filename_matches
    };

    // Deduplicate paths (e.g., if different symlinks point to the same canonical file
    // and WalkDir followed them, though canonicalize in create_resolved_file helps later)
    // For now, simple sort and dedup on the PathBufs themselves.
    final_candidate_paths.sort();
    final_candidate_paths.dedup();

    match final_candidate_paths.len() {
        0 => InputResolution::NotFound {
            input_string: input_str,
        },
        1 => {
            // Only one candidate, try to resolve it.
            match create_resolved_file(&final_candidate_paths[0], config) {
                Ok(resolved) => InputResolution::Success(vec![resolved]),
                Err(err_msg) => {
                    eprintln!(
                        "Warning: Found unique match for '{}' but failed to process it ({:?}): {}",
                        input_str, &final_candidate_paths[0], err_msg
                    );
                    InputResolution::NotFound {
                        input_string: input_str,
                    } // Or a more specific error variant
                }
            }
        }
        _ => {
            // Multiple candidates, this is an ambiguity.
            // Convert candidate paths to display paths for the error message.
            let conflicting_display_paths: Vec<PathBuf> = final_candidate_paths
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
