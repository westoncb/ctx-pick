// Declare the modules we've created
mod config;
mod error;
mod file_resolver;
mod types;

use clap::Parser;
use std::collections::BTreeSet; // For unique canonical paths
use std::path::PathBuf;

// Bring our types into scope
use config::Config;
use error::AppError;
use types::{InputResolution, ResolvedFile};

#[derive(Parser, Debug)]
#[clap(
    author = "Your Name", // Replace with your name/handle
    version = "0.1.0",
    about = "Builds context strings from code files for LLMs.",
    long_about = "Reads specified code files (or files found by partial names/in folders) \
                  and concatenates their contents into a single Markdown formatted string."
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
    let config = Config::new()?; // This can return AppError::ConfigError

    // Store all resolution attempts. The lifetime 'a of InputResolution<'a>
    // is tied to the lifetime of strings in `cli.inputs`.
    let mut all_resolutions: Vec<InputResolution<'_>> = Vec::new();
    for input_str in &cli.inputs {
        let resolution = file_resolver::resolve_input_string(input_str, &config);
        all_resolutions.push(resolution);
    }

    let mut final_ordered_files: Vec<ResolvedFile> = Vec::new();
    let mut seen_canonical_paths: BTreeSet<PathBuf> = BTreeSet::new(); // Use BTreeSet

    // For collecting issues to report
    let mut path_does_not_exist_errors: Vec<&InputResolution<'_>> = Vec::new();
    let mut not_founds: Vec<&InputResolution<'_>> = Vec::new();
    let mut ambiguities_found: Vec<&InputResolution<'_>> = Vec::new();

    for resolution in &all_resolutions {
        match resolution {
            InputResolution::Success(resolved_files_for_input) => {
                for resolved_file in resolved_files_for_input {
                    // Add to final list only if its canonical path hasn't been seen
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

    // If there were any problems that prevent output generation, report them.
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

        // Optionally, list successfully resolved files even in case of error
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
            input: "Multiple Inputs".to_string(), // Generic summary
            message: "One or more inputs could not be resolved.".to_string(),
        });
    }

    // If we reach here, all inputs were resolved successfully (or resulted in empty sets).
    if final_ordered_files.is_empty() {
        if cli.inputs.is_empty() {
            // Should be caught by clap's `required=true`
            println!("No input files specified.");
        } else {
            println!("No files were found or resolved based on your input.");
        }
        return Ok(()); // Successful exit, just nothing to do.
    }

    // Announce the files that will be included.
    println!("The following files will be included in the context (in order):");
    for resolved_file in &final_ordered_files {
        println!("- {:?}", resolved_file.display_path());
    }
    println!("\nGenerating context string...\n");

    let mut markdown_output = String::new();
    for resolved_file in &final_ordered_files {
        let file_content = match std::fs::read_to_string(resolved_file.canonical_path()) {
            Ok(content) => content,
            Err(e) => {
                // This can happen if file is deleted/permissions change between resolution and read.
                eprintln!(
                    "Warning: Failed to read file content for {:?} (resolved from {}): {}. Including error in output.",
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

        // Use the display_path for the filename header in the Markdown.
        markdown_output.push_str(&format!(
            "{}\n```\n{}\n```\n\n",
            resolved_file.display_path().to_string_lossy(),
            file_content.trim_end() // Trim trailing newline from file content
        ));
    }

    print!("{}", markdown_output);

    Ok(())
}
