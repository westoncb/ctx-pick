// main.rs

// Declare all modules for the application.
mod config;
mod display;
mod error;
mod file_resolver;
mod symbol_extractor;
mod types;

use crate::{
    config::Config,
    display::{DisplayManager, generate_markdown_output},
    error::AppError,
    types::{InputResolution, ResolvedFile},
};
use arboard::Clipboard;
use clap::Parser;
use std::{collections::BTreeSet, path::PathBuf};

/// `ctx-pick` is a command-line utility that gathers file contents into a
/// Markdown-formatted string for easy copying to a clipboard, tailored for
/// providing context to LLMs.
#[derive(Parser, Debug)]
#[clap(
    author = "Weston C. Beecroft",
    version = "0.1.2",
    about = "Builds context strings from code files for LLMs and copies to clipboard.",
    long_about = "Reads specified code files (or files found by partial names/in folders), \
                  concatenates their contents into a single Markdown formatted string, \
                  and attempts to copy it to the system clipboard, printing to stdout as a fallback."
)]
struct Cli {
    /// A space-separated list of files, partial names, or folders to include.
    #[arg(required = true, num_args = 1..)]
    inputs: Vec<String>,

    /// If set, extracts only symbol definitions (e.g., function signatures, struct definitions)
    /// instead of the full file content.
    #[arg(long, help = "Extract symbols instead of full file content.")]
    symbols: bool,
}

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = Config::new()?;
    let display = DisplayManager::new();

    // Resolve all user inputs into a list of `InputResolution` enums.
    // Each input is resolved independently to gather all successes and failures.
    let mut all_resolutions: Vec<InputResolution<'_>> = Vec::new();
    for input_str in &cli.inputs {
        let resolution = file_resolver::resolve_input_string(input_str, &config);
        all_resolutions.push(resolution);
    }

    // Process all resolutions, bucketing them into successes and various error types.
    // A BTreeSet is used to ensure that each canonical file path is included only once.
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

    // If any unrecoverable errors occurred (e.g., ambiguous inputs or non-existent paths),
    // print a detailed report and exit with a non-zero status code.
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
            .unwrap_or_else(|e| eprintln!("Critical display error: {}", e));

        std::process::exit(1);
    }

    // If no files were successfully resolved from the inputs, inform the user and exit.
    if final_ordered_files.is_empty() {
        if cli.inputs.is_empty() {
            // This case should be prevented by clap's `required = true`, but is here as a safeguard.
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
        std::process::exit(1);
    }

    // Generate the final output, either full file content or extracted symbols, based on the --symbols flag.
    let markdown_output = generate_markdown_output(&final_ordered_files, cli.symbols);
    let output_count = markdown_output.lines().count();

    // Attempt to copy the generated string to the system clipboard.
    let clipboard_result = match Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(markdown_output.clone()),
        Err(err) => Err(err),
    };

    // Display the summary report, passing the mode to allow for tailored output (e.g., "lines" vs "symbols").
    display
        .print_operation_summary_and_preview(
            &final_ordered_files,
            &clipboard_result,
            output_count,
            cli.symbols,
        )
        .unwrap_or_else(|e| eprintln!("Display error during summary: {}", e));

    // If clipboard access failed, print the full markdown output to stdout as a fallback.
    if clipboard_result.is_err() {
        println!("{}", markdown_output);
    }

    Ok(())
}
