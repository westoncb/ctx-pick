mod config;
mod display;
mod error;
mod file_resolver;
mod symbol_extractor;
mod types;

use crate::{
    config::Config,
    display::DisplayManager,
    error::AppError,
    types::{FileContext, InputResolution, ResolvedFile},
};
use arboard::Clipboard;
use clap::Parser;
use std::{collections::BTreeSet, path::Path, path::PathBuf};

/// A versatile CLI tool that finds files by name, path, or glob pattern,
/// extracts their content or a structural 'skeleton', formats it as
/// Markdown, and copies it to the clipboard. Ideal for providing
/// context to LLMs.
#[derive(Parser, Debug)]
#[clap(
    author = "Weston C. Beecroft",
    version = "0.3.1", // Version bump for new features!
    about = "Builds context strings from code files for LLMs and copies to clipboard.",
    long_about = None // The long help is now the main help text above.
)]
struct Cli {
    /// A space-separated list of files, partial names, folders, or glob patterns.
    /// e.g., 'main.rs', 'src/utils', 'src/**/*.ts'
    #[arg(required = true, num_args = 1..)]
    inputs: Vec<String>,

    /// Instead of full file content, extract a structural 'skeleton' of the code
    /// (e.g., function signatures, struct definitions) up to a certain depth.
    /// A depth of 3-5 is usually effective.
    #[arg(
        long,
        value_name = "LEVEL",
        help = "Extract a code skeleton at a specific depth."
    )]
    depth: Option<usize>,

    /// Print the final context to stdout instead of copying to the clipboard.
    /// This is useful for piping the output to other commands.
    #[arg(long, help = "Print to stdout instead of the clipboard")]
    to_stdout: bool,
}

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = Config::new()?;
    let display = DisplayManager::new();

    // Resolve all user inputs into a list of `InputResolution` enums.
    let mut all_resolutions: Vec<InputResolution<'_>> = Vec::new();
    for input_str in &cli.inputs {
        let resolution = file_resolver::resolve_input_string(input_str, &config);
        all_resolutions.push(resolution);
    }

    // Process all resolutions, bucketing them into successes and various error types.
    let mut final_ordered_files: Vec<ResolvedFile> = Vec::new();
    let mut seen_canonical_paths: BTreeSet<PathBuf> = BTreeSet::new();

    let mut path_does_not_exist_errors: Vec<&InputResolution<'_>> = Vec::new();
    let mut not_founds: Vec<&InputResolution<'_>> = Vec::new();
    let mut ambiguities_found: Vec<&InputResolution<'_>> = Vec::new();
    let mut invalid_glob_patterns: Vec<&InputResolution<'_>> = Vec::new(); // New error bucket

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
            // Add the new case for our glob pattern errors
            InputResolution::InvalidGlobPattern { .. } => {
                invalid_glob_patterns.push(resolution);
            }
        }
    }

    // If any unrecoverable errors occurred, print a detailed report and exit.
    let has_errors = !path_does_not_exist_errors.is_empty()
        || !not_founds.is_empty()
        || !ambiguities_found.is_empty()
        || !invalid_glob_patterns.is_empty();

    if has_errors {
        display
            .print_resolution_errors(
                &path_does_not_exist_errors,
                &not_founds,
                &ambiguities_found,
                &invalid_glob_patterns, // Pass the new bucket to the display manager
                &final_ordered_files,
            )
            .unwrap_or_else(|e| eprintln!("Critical display error: {}", e));

        std::process::exit(1);
    }

    // If no files were successfully resolved from the inputs, inform the user and exit.
    if final_ordered_files.is_empty() {
        eprintln!(
            "{}",
            display
                .warning_style
                .apply_to("No files were found or resolved based on your input.")
        );
        std::process::exit(1);
    }

    // 1. Process all resolved files into our FileContext struct.
    let file_contexts = generate_file_contexts(&final_ordered_files, cli.depth);

    // 2. Build the final Markdown string for the output.
    let mut markdown_output = String::new();
    for context in &file_contexts {
        let lang_hint = if cli.depth.is_some() {
            ""
        } else {
            Path::new(&context.display_path)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        };
        markdown_output.push_str(&format!(
            "{}\n```{}\n{}\n```\n\n",
            context.display_path,
            lang_hint,
            context.content.trim_end()
        ));
    }

    if cli.to_stdout {
        // --- Script-Friendly Path ---
        // Just print the final Markdown to standard output and exit.
        print!("{}", markdown_output);
    } else {
        // --- Interactive/Clipboard Path (existing logic) ---
        let (total_metric, unit_str) = if cli.depth.is_some() {
            (markdown_output.len(), "characters")
        } else {
            let total_lines = file_contexts
                .iter()
                .map(|ctx| ctx.content.lines().count())
                .sum();
            (total_lines, "lines")
        };

        let clipboard_result = match Clipboard::new() {
            Ok(mut clipboard) => clipboard.set_text(markdown_output.clone()),
            Err(err) => Err(err),
        };

        display
            .print_operation_summary_and_preview(
                &file_contexts,
                &clipboard_result,
                total_metric,
                unit_str,
                cli.depth,
            )
            .unwrap_or_else(|e| eprintln!("Display error during summary: {}", e));

        if clipboard_result.is_err() {
            println!("{}", markdown_output);
        }
    }

    Ok(())
}

/// Processes a list of resolved files, returning a vector containing the
/// context (full or skeleton) for each.
fn generate_file_contexts(files: &[ResolvedFile], depth: Option<usize>) -> Vec<FileContext> {
    let mut contexts = Vec::new();

    for resolved_file in files {
        let display_path = resolved_file.display_path().to_string_lossy().to_string();
        let file_content_result = std::fs::read_to_string(resolved_file.canonical_path());

        let final_content = match file_content_result {
            Err(e) => format!(
                "Error: Could not read file content for {:?}.\nDetails: {}",
                display_path, e
            ),
            Ok(content) => {
                if let Some(max_depth) = depth {
                    let extension = resolved_file
                        .display_path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    match symbol_extractor::create_skeleton_by_depth(&content, extension, max_depth)
                    {
                        Ok(symbols) => symbols,
                        Err(e) => format!(
                            "---\n-- ERROR: Could not extract symbols from {:?}: {}\n-- Falling back to full file content.\n---\n\n{}",
                            display_path, e, content
                        ),
                    }
                } else {
                    content
                }
            }
        };

        contexts.push(FileContext {
            display_path,
            content: final_content,
        });
    }
    contexts
}
