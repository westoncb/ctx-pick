// Declare the modules we've created
mod config;
mod error;
mod file_resolver;
mod types;

use clap::Parser;
use std::collections::BTreeSet; // For unique canonical paths
use std::path::PathBuf; // Though not directly used here, often useful. Let's keep it for now.

// Bring our types into scope
use config::Config;
use error::AppError;
use types::{InputResolution, ResolvedFile};

// Import for clipboard functionality
use arboard::Clipboard;

#[derive(Parser, Debug)]
#[clap(
    author = "Weston C. Beecroft",
    version = "0.1.1", // Consider incrementing version for new feature
    about = "Builds context strings from code files for LLMs and copies to clipboard.",
    long_about = "Reads specified code files (or files found by partial names/in folders), \
                  concatenates their contents into a single Markdown formatted string, \
                  prints it to stdout, and attempts to copy it to the system clipboard."
)]
struct Cli {
    /// Files, partial names, or folders to include in the context.
    /// Provide space-separated values.
    #[arg(required = true, num_args = 1..)]
    inputs: Vec<String>,
    // Example of a future flag:
    // #[arg(short, long, help = "Enable verbose output for debugging.")]
    // verbose: bool,
}

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = Config::new()?;

    let mut all_resolutions: Vec<InputResolution<'_>> = Vec::new();
    for input_str in &cli.inputs {
        let resolution = file_resolver::resolve_input_string(input_str, &config);
        all_resolutions.push(resolution);
    }

    let mut final_ordered_files: Vec<ResolvedFile> = Vec::new();
    let mut seen_canonical_paths: BTreeSet<PathBuf> = BTreeSet::new();

    let mut path_does_not_exist_errors: Vec<&InputResolution<'_>> = Vec::new();
    let mut not_founds: Vec<&InputResolution<'_>> = Vec::new();
    let mut ambiguities_found: Vec<&InputResolution<'_>> = Vec::new();

    for resolution in &all_resolutions {
        match resolution {
            InputResolution::Success(resolved_files_for_input) => {
                for resolved_file in resolved_files_for_input {
                    if seen_canonical_paths.insert(resolved_file.canonical_path().to_path_buf()) {
                        final_ordered_files.push(resolved_file.clone());
                    }
                }
            }
            InputResolution::Ambiguous { .. } => {
                ambiguities_found.push(resolution);
            }
            InputResolution::NotFound { .. } => {
                not_founds.push(resolution);
            }
            InputResolution::PathDoesNotExist { .. } => {
                path_does_not_exist_errors.push(resolution);
            }
        }
    }

    if !path_does_not_exist_errors.is_empty()
        || !not_founds.is_empty()
        || !ambiguities_found.is_empty()
    {
        eprintln!("Could not proceed due to unresolved inputs:");
        eprintln!("-----------------------------------------");

        if !path_does_not_exist_errors.is_empty() {
            eprintln!("\nThe following specified paths do not exist:");
            for case in path_does_not_exist_errors {
                if let InputResolution::PathDoesNotExist {
                    input_string,
                    path_tried,
                } = case
                {
                    eprintln!("  - Input: '{}' (checked: {:?})", input_string, path_tried);
                }
            }
        }

        if !not_founds.is_empty() {
            eprintln!("\nThe following inputs could not be found:");
            for case in not_founds {
                if let InputResolution::NotFound { input_string } = case {
                    eprintln!("  - Input: '{}'", input_string);
                }
            }
        }

        if !ambiguities_found.is_empty() {
            eprintln!("\nThe following inputs are ambiguous:");
            for case in ambiguities_found {
                if let InputResolution::Ambiguous {
                    input_string,
                    conflicting_paths,
                } = case
                {
                    eprintln!("  - Input: '{}' matched:", input_string);
                    for path in conflicting_paths {
                        eprintln!("    - {:?}", path);
                    }
                }
            }
        }

        if !final_ordered_files.is_empty() {
            eprintln!("\nSuccessfully resolved files (would have been included):");
            for resolved_file in &final_ordered_files {
                eprintln!("  - {:?}", resolved_file.display_path());
            }
        } else {
            eprintln!("\nNo files were successfully resolved.");
        }

        eprintln!("\nPlease resolve the issues above and try again.");
        return Err(AppError::ResolutionError {
            input: "Multiple Inputs".to_string(),
            message: "One or more inputs could not be resolved.".to_string(),
        });
    }

    if final_ordered_files.is_empty() {
        if cli.inputs.is_empty() {
            eprintln!("No input files specified."); // Should be caught by clap
        } else {
            eprintln!("No files were found or resolved based on your input.");
        }
        return Ok(());
    }

    eprintln!("The following files will be included in the context (in order):");
    for resolved_file in &final_ordered_files {
        eprintln!("- {:?}", resolved_file.display_path());
    }

    let mut markdown_output = String::new();
    for resolved_file in &final_ordered_files {
        let file_content = match std::fs::read_to_string(resolved_file.canonical_path()) {
            Ok(content) => content,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to read file content for {:?} (from {}): {}. Including error in output.",
                    resolved_file.display_path(),
                    resolved_file.canonical_path().display(),
                    e
                );
                format!(
                    "Error: Could not read file content for {:?}.\nDetails: {}",
                    resolved_file.display_path(),
                    e
                )
            }
        };

        markdown_output.push_str(&format!(
            "{}\n```\n{}\n```\n\n",
            resolved_file.display_path().to_string_lossy(),
            file_content.trim_end()
        ));
    }

    // Attempt to copy to clipboard
    match Clipboard::new() {
        Ok(mut clipboard) => {
            // Added 'mut' here
            match clipboard.set_text(markdown_output.clone()) {
                // Clone so we can still print
                Ok(_) => {
                    eprintln!(
                        "\nContext successfully copied to clipboard ({} bytes, {} lines).",
                        markdown_output.len(),
                        markdown_output.lines().count()
                    );
                }
                Err(err) => {
                    eprintln!(
                        "\nWarning: Failed to copy context to clipboard: {}. Output will be printed to stdout below.",
                        err
                    );
                }
            }
        }
        Err(err) => {
            eprintln!(
                "\nWarning: Failed to initialize clipboard: {}. Output will be printed to stdout below.",
                err
            );
        }
    }

    // Always print the final markdown to stdout
    print!("{}", markdown_output);

    Ok(())
}
