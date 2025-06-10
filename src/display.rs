// display.rs

use crate::{
    symbol_extractor,
    types::{InputResolution, ResolvedFile},
};
use arboard;
use console::{Style, Term};
use std::io::{self, Write};

/// Manages all terminal output to stderr, such as status messages,
/// progress, and error reports. It uses the `console` crate for styling.
pub struct DisplayManager {
    term: Term,
    pub error_style: Style,
    pub warning_style: Style,
    pub success_style: Style,
    pub filename_style: Style,
    pub metadata_style: Style,
    pub ambiguous_style: Style,
}

impl DisplayManager {
    /// Creates a new `DisplayManager` with a default set of styles.
    pub fn new() -> Self {
        Self {
            term: Term::stderr(),
            error_style: Style::new().red().bold(),
            warning_style: Style::new().yellow(),
            success_style: Style::new().green().bold(),
            filename_style: Style::new().cyan().bold(),
            metadata_style: Style::new().dim(),
            ambiguous_style: Style::new().magenta().bold(),
        }
    }

    /// Prints a detailed report of all file resolution errors.
    /// This is used when the program must exit early due to unresolvable inputs.
    pub fn print_resolution_errors(
        &self,
        path_errors: &[&InputResolution],
        not_founds: &[&InputResolution],
        ambiguities: &[&InputResolution],
        successful_files: &[ResolvedFile],
    ) -> io::Result<()> {
        let mut stderr = self.term.clone();

        writeln!(
            stderr,
            "{}",
            self.error_style
                .apply_to("Could not proceed due to unresolved inputs:")
        )?;
        writeln!(stderr, "{}", self.metadata_style.apply_to("-".repeat(50)))?;

        if !path_errors.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.error_style
                    .apply_to("The following specified paths do not exist:")
            )?;
            for case in path_errors {
                if let InputResolution::PathDoesNotExist {
                    input_string,
                    path_tried,
                } = case
                {
                    writeln!(
                        stderr,
                        "  {} {} {}",
                        self.metadata_style.apply_to("•"),
                        self.error_style
                            .apply_to(format!("Input: '{}'", input_string)),
                        self.metadata_style
                            .apply_to(format!("(checked: {:?})", path_tried))
                    )?;
                }
            }
        }

        if !not_founds.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.warning_style
                    .apply_to("The following inputs could not be found:")
            )?;
            for case in not_founds {
                if let InputResolution::NotFound { input_string } = case {
                    writeln!(
                        stderr,
                        "  {} {}",
                        self.metadata_style.apply_to("•"),
                        self.warning_style
                            .apply_to(format!("Input: '{}'", input_string))
                    )?;
                }
            }
        }

        if !ambiguities.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.ambiguous_style
                    .apply_to("The following inputs are ambiguous:")
            )?;
            for case in ambiguities {
                if let InputResolution::Ambiguous {
                    input_string,
                    conflicting_paths,
                } = case
                {
                    write!(
                        stderr,
                        "  {} {} ",
                        self.metadata_style.apply_to("•"),
                        self.ambiguous_style.apply_to("Input")
                    )?;
                    write!(
                        stderr,
                        "{} ",
                        self.warning_style.apply_to(format!("'{}'", input_string))
                    )?;
                    writeln!(stderr, "{}", self.ambiguous_style.apply_to("matched:"))?;

                    const MAX_AMBIGUOUS_PATHS_TO_SHOW: usize = 8;
                    for (i, path) in conflicting_paths.iter().enumerate() {
                        if i < MAX_AMBIGUOUS_PATHS_TO_SHOW {
                            writeln!(
                                stderr,
                                "    {} {}",
                                self.metadata_style.apply_to("→"),
                                self.filename_style.apply_to(format!("{:?}", path))
                            )?;
                        } else {
                            let remaining = conflicting_paths.len() - MAX_AMBIGUOUS_PATHS_TO_SHOW;
                            writeln!(
                                stderr,
                                "    {} ... and {} more match{}.",
                                self.metadata_style.apply_to("→"),
                                self.metadata_style.apply_to(remaining.to_string()),
                                if remaining == 1 { "" } else { "es" }
                            )?;
                            break;
                        }
                    }
                }
            }
        }

        if !successful_files.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.success_style
                    .apply_to("However, these files were successfully resolved:")
            )?;
            for resolved_file in successful_files {
                writeln!(
                    stderr,
                    "  {} {}",
                    self.metadata_style.apply_to("✓"),
                    self.filename_style
                        .apply_to(format!("{:?}", resolved_file.display_path()))
                )?;
            }
        }

        writeln!(
            stderr,
            "\n{}",
            self.metadata_style
                .apply_to("Please resolve the issues above and try again.")
        )?;
        Ok(())
    }

    /// Prints the final summary report after a successful operation.
    /// This includes the clipboard status and a preview of the included files.
    pub fn print_operation_summary_and_preview(
        &self,
        files: &[ResolvedFile],
        clipboard_result: &Result<(), arboard::Error>,
        output_count: usize,
        symbols_mode: bool,
    ) -> io::Result<()> {
        let mut stderr = self.term.clone();
        let unit = if symbols_mode { "symbols" } else { "lines" };

        match clipboard_result {
            Ok(_) => {
                writeln!(
                    stderr,
                    "{} Context copied to clipboard ({} files, {} {})",
                    self.success_style.apply_to("✅"),
                    self.metadata_style.apply_to(files.len().to_string()),
                    self.metadata_style.apply_to(output_count.to_string()),
                    self.metadata_style.apply_to(unit),
                )?;
            }
            Err(err) => {
                writeln!(
                    stderr,
                    "{} Failed to copy to clipboard.",
                    self.warning_style.apply_to("⚠️")
                )?;
                writeln!(
                    stderr,
                    "    {}: {}",
                    self.warning_style.apply_to("Error"),
                    self.warning_style.apply_to(err.to_string())
                )?;
                writeln!(
                    stderr,
                    "    {}",
                    self.metadata_style
                        .apply_to("Full context will be printed to stdout as a fallback.")
                )?;
            }
        }

        writeln!(stderr, "{}", self.metadata_style.apply_to("=".repeat(40)))?;
        writeln!(
            stderr,
            "{}",
            self.filename_style.apply_to("Included files:")
        )?;

        if files.is_empty() {
            writeln!(
                stderr,
                "  {}",
                self.metadata_style.apply_to("(No files to preview)")
            )?;
        } else {
            for (i, resolved_file) in files.iter().enumerate() {
                writeln!(
                    stderr,
                    "\n{}. {}",
                    self.metadata_style.apply_to(format!("{}", i + 1)),
                    self.filename_style
                        .apply_to(resolved_file.display_path().to_string_lossy())
                )?;

                // NOTE: The per-file preview currently always shows the total line count of the
                // source file, even in symbols mode. A future enhancement could be to show the
                // extracted symbol count here, but that would require re-processing the file.
                match std::fs::read_to_string(resolved_file.canonical_path()) {
                    Ok(content) => {
                        let total_lines = content.lines().count();
                        writeln!(
                            stderr,
                            "    {} {} lines",
                            self.metadata_style.apply_to("📄"),
                            self.metadata_style.apply_to(total_lines.to_string())
                        )?;
                    }
                    Err(e) => {
                        writeln!(
                            stderr,
                            "    {} {}",
                            self.error_style.apply_to("⚠"),
                            self.error_style
                                .apply_to(format!("Error reading file for preview: {}", e))
                        )?;
                    }
                }
            }
        }
        writeln!(stderr, "\n{}", self.metadata_style.apply_to("=".repeat(40)))?;
        Ok(())
    }
}

/// Generates the final Markdown output string for the clipboard or stdout.
///
/// This function will either read the full file content or use the `symbol_extractor`
/// module to get symbol definitions, based on the `symbols_mode` flag.
pub fn generate_markdown_output(files: &[ResolvedFile], symbols_mode: bool) -> String {
    let mut markdown_output = String::new();

    for resolved_file in files {
        let file_content_result = std::fs::read_to_string(resolved_file.canonical_path());

        let output_block = match file_content_result {
            Err(e) => format!(
                "Error: Could not read file content for {:?}.\nDetails: {}",
                resolved_file.display_path(),
                e
            ),
            Ok(content) => {
                if symbols_mode {
                    // In symbols mode, attempt to extract symbols.
                    let extension = resolved_file
                        .display_path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    match symbol_extractor::create_skeleton_by_depth(&content, extension, 4) {
                        Ok(symbols) => symbols,
                        Err(e) => {
                            // If symbol extraction fails, provide a helpful error and fall back
                            // to including the full file content so the user still gets output.
                            format!(
                                "---\n-- ERROR: Could not extract symbols from {:?}: {}\n-- Falling back to full file content.\n---\n\n{}",
                                resolved_file.display_path(),
                                e,
                                content
                            )
                        }
                    }
                } else {
                    // Default mode: use the full file content.
                    content
                }
            }
        };

        // For symbol output, we omit the language hint in the markdown code block
        // as it's not a complete, compilable file.
        let lang_hint = if symbols_mode {
            ""
        } else {
            resolved_file
                .display_path()
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        };

        markdown_output.push_str(&format!(
            "{}\n```{}\n{}\n```\n\n",
            resolved_file.display_path().to_string_lossy(),
            lang_hint,
            output_block.trim_end()
        ));
    }

    markdown_output
}
