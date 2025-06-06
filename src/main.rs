// Declare the modules we've created
mod config;
mod display;
mod error;
mod file_resolver;
mod types;

use clap::Parser;
use std::collections::BTreeSet; // For unique canonical paths
use std::path::PathBuf;

// Bring our types into scope
use config::Config;
use display::{DisplayManager, generate_full_markdown}; // generate_full_markdown is still needed
use error::AppError; // AppError is still used by Config::new()
use types::{InputResolution, ResolvedFile};

// Import for clipboard functionality
use arboard::Clipboard; // Ensure this is in your Cargo.toml

#[derive(Parser, Debug)]
#[clap(
    author = "Weston C. Beecroft",
    version = "0.1.2", // Version bump for changes
    about = "Builds context strings from code files for LLMs and copies to clipboard.",
    long_about = "Reads specified code files (or files found by partial names/in folders), \
                  concatenates their contents into a single Markdown formatted string, \
                  and attempts to copy it to the system clipboard, printing to stdout as a fallback."
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
    let config = Config::new()?; // This can still return AppError::ConfigError or IoError via AppError
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
        // DisplayManager now handles detailed error printing to stderr
        display
            .print_resolution_errors(
                &path_does_not_exist_errors,
                &not_founds,
                &ambiguities_found,
                &final_ordered_files, // Pass currently resolved files for context
            )
            .unwrap_or_else(|e| eprintln!("Critical display error: {}", e)); // Fallback if display itself errors

        // Exit with a non-zero status code; detailed errors already printed
        std::process::exit(1);
    }

    if final_ordered_files.is_empty() {
        // This case implies inputs were given, but nothing resolved successfully,
        // and no specific errors like NotFound or Ambiguous were severe enough to exit above
        // (or they were handled gracefully by print_resolution_errors if it didn't exit).
        // However, with the std::process::exit(1) above, this block might only be reached
        // if inputs somehow led to no files and no *errors* that trigger the exit.
        // For example, if all inputs were empty directories.
        if cli.inputs.is_empty() {
            // Should be caught by clap's `required = true`
            eprintln!(
                "{}",
                display.error_style.apply_to("No input files specified.")
            );
        } else {
            eprintln!(
                "{}",
                display
                    .warning_style
                    .apply_to("No files were found or resolved based on your input.")
            );
        }
        // It's debatable whether this should be a clean exit (0) or an error exit (1).
        // If inputs were provided but no files resulted, it could be considered an error.
        std::process::exit(1); // Let's treat it as an issue if inputs led to no files.
    }

    // Generate the full markdown output for clipboard/stdout
    let markdown_output = generate_full_markdown(&final_ordered_files);
    let markdown_lines = markdown_output.lines().count();

    // Attempt to copy to clipboard
    let clipboard_result = match Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(markdown_output.clone()), // Clone because markdown_output might be printed to stdout
        Err(err) => Err(err), // Propagate arboard::Error
    };

    // Display the consolidated operation summary and file previews
    // This new function in DisplayManager will handle the conditional header
    // based on clipboard_result and then print the file previews.
    display
        .print_operation_summary_and_preview(
            &final_ordered_files,
            &clipboard_result, // Pass the Result itself
            markdown_lines,
        )
        .unwrap_or_else(|e| eprintln!("Display error during summary: {}", e));

    // If clipboard copy failed, print the markdown output to stdout as a fallback
    if clipboard_result.is_err() {
        // The failure message to stderr should have been handled by print_operation_summary_and_preview
        println!("{}", markdown_output);
    }

    Ok(())
}
