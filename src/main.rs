// Declare the modules we've created
mod config;
mod display;
mod error;
mod file_resolver;
mod types;

use clap::Parser;
use std::collections::BTreeSet; // For unique canonical paths
use std::path::PathBuf; // Though not directly used here, often useful. Let's keep it for now.

// Bring our types into scope
use config::Config;
use display::{DisplayManager, generate_full_markdown};
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
    let display = DisplayManager::new();

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
        display
            .print_resolution_errors(
                &path_does_not_exist_errors,
                &not_founds,
                &ambiguities_found,
                &final_ordered_files,
            )
            .unwrap_or_else(|e| eprintln!("Display error: {}", e));

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

    // Show styled preview of files that will be included
    display
        .print_file_preview(&final_ordered_files)
        .unwrap_or_else(|e| eprintln!("Display error: {}", e));

    // Generate the full markdown output for clipboard
    let markdown_output = generate_full_markdown(&final_ordered_files);

    // Attempt to copy to clipboard with styled status reporting
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(markdown_output.clone()) {
            Ok(_) => {
                display
                    .print_clipboard_status(
                        true,
                        markdown_output.len(),
                        markdown_output.lines().count(),
                        None,
                    )
                    .unwrap_or_else(|e| eprintln!("Display error: {}", e));
            }
            Err(err) => {
                display
                    .print_clipboard_status(false, 0, 0, Some(&err.to_string()))
                    .unwrap_or_else(|e| eprintln!("Display error: {}", e));
            }
        },
        Err(err) => {
            display
                .print_clipboard_status(false, 0, 0, Some(&err.to_string()))
                .unwrap_or_else(|e| eprintln!("Display error: {}", e));
        }
    }

    // Note: Full markdown is copied to clipboard, no need to spam terminal
    // print!("{}", markdown_output);

    Ok(())
}
