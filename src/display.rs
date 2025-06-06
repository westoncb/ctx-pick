use crate::types::{InputResolution, ResolvedFile};
use console::{Style, Term};
use std::io::{self, Write};

// For clipboard_result in the new function
use arboard; // Make sure arboard is accessible, or pass a simpler error string

pub struct DisplayManager {
    term: Term,
    pub error_style: Style,
    pub warning_style: Style, // Will also be used for highlighting input_string in ambiguous
    pub success_style: Style,
    pub filename_style: Style,
    pub metadata_style: Style,
    pub ambiguous_style: Style, // For "Input" and "matched:" parts
}

impl DisplayManager {
    pub fn new() -> Self {
        Self {
            term: Term::stderr(), // Use stderr for status messages
            error_style: Style::new().red().bold(),
            warning_style: Style::new().yellow(), // Used for warnings and input string highlights
            success_style: Style::new().green().bold(), // Bolder success
            filename_style: Style::new().cyan().bold(),
            metadata_style: Style::new().dim(),
            ambiguous_style: Style::new().magenta().bold(), // For "Input" and "matched:"
        }
    }

    /// Print resolution errors with proper styling
    pub fn print_resolution_errors(
        &self,
        path_errors: &[&InputResolution],
        not_founds: &[&InputResolution],
        ambiguities: &[&InputResolution],
        successful_files: &[ResolvedFile], // To inform user what *was* found, if anything
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
                    // New styling: "Input 'err' matched:"
                    write!(
                        stderr,
                        "  {} {} ", // Note the space after "Input "
                        self.metadata_style.apply_to("•"),
                        self.ambiguous_style.apply_to("Input")
                    )?;
                    write!(
                        stderr,
                        "{} ",
                        self.warning_style.apply_to(format!("'{}'", input_string)) // 'err' in warning_style (yellow)
                    )?;
                    writeln!(
                        stderr,
                        "{}",
                        self.ambiguous_style.apply_to("matched:") // "matched:" in ambiguous_style
                    )?;

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
                    .apply_to("However, these files were successfully resolved (and would have been included):")
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
        } else if path_errors.is_empty() && not_founds.is_empty() && ambiguities.is_empty() {
            // This case should ideally not be hit if main exits, but as a safeguard:
            writeln!(
                stderr,
                "\n{}",
                self.error_style.apply_to(
                    "No files were successfully resolved and no specific errors to report."
                )
            )?;
        }

        writeln!(
            stderr,
            "\n{}",
            self.metadata_style
                .apply_to("Please resolve the issues above and try again.")
        )?;
        Ok(())
    }

    /// Prints the operational summary (clipboard status) and then previews the files.
    pub fn print_operation_summary_and_preview(
        &self,
        files: &[ResolvedFile],
        clipboard_result: &Result<(), arboard::Error>,
        markdown_lines: usize,
    ) -> io::Result<()> {
        let mut stderr = self.term.clone();

        // Print top-level status header
        match clipboard_result {
            Ok(_) => {
                writeln!(
                    stderr,
                    "{} Context copied to clipboard ({} files, {} lines)",
                    self.success_style.apply_to("✅"),
                    self.metadata_style.apply_to(files.len().to_string()),
                    self.metadata_style.apply_to(markdown_lines.to_string()),
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
                    "   {}: {}",
                    self.warning_style.apply_to("Error"),
                    self.warning_style.apply_to(err.to_string())
                )?;
                writeln!(
                    stderr,
                    "   {}",
                    self.metadata_style
                        .apply_to("Full context string will be printed to stdout as a fallback.")
                )?;
            }
        }

        writeln!(stderr, "{}", self.metadata_style.apply_to("=".repeat(40)))?;
        writeln!(
            stderr,
            "{}",
            self.filename_style.apply_to("Included files:")
        )?;

        // Print file previews (logic from old print_file_preview)
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
                    "\n{}. {}", // Numbered list for files
                    self.metadata_style.apply_to(format!("{}", i + 1)),
                    self.filename_style
                        .apply_to(resolved_file.display_path().to_string_lossy())
                )?;

                match std::fs::read_to_string(resolved_file.canonical_path()) {
                    Ok(content) => {
                        let lines: Vec<&str> = content.lines().collect();
                        let total_lines = lines.len();
                        // Removed byte count from here as it's in the summary
                        writeln!(
                            stderr,
                            "    {} {} lines",
                            self.metadata_style.apply_to("📄"),
                            self.metadata_style.apply_to(total_lines.to_string())
                        )?;

                        // let preview_lines_count = std::cmp::min(1, total_lines);
                        // if preview_lines_count > 0 {
                        //     // Removed "Preview:" sub-header to make it cleaner
                        //     for (line_num, line) in
                        //         lines.iter().take(preview_lines_count).enumerate()
                        //     {
                        //         let truncated_line = if line.len() > 80 {
                        //             format!("{}...", &line[..77])
                        //         } else {
                        //             line.to_string()
                        //         };
                        //         writeln!(
                        //             stderr,
                        //             "      {} {}", // Indent preview lines further
                        //             self.metadata_style.apply_to(format!("{:2}│", line_num + 1)),
                        //             self.preview_style.apply_to(truncated_line)
                        //         )?;
                        //     }
                        //     if total_lines > preview_lines_count {
                        //         writeln!(
                        //             stderr,
                        //             "      {} {} more lines...",
                        //             self.metadata_style.apply_to("  ┆"),
                        //             self.metadata_style.apply_to(format!(
                        //                 "({})",
                        //                 total_lines - preview_lines_count
                        //             ))
                        //         )?;
                        //     }
                        // }
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

/// Generate the full markdown output for clipboard (unchanged from your original logic)
pub fn generate_full_markdown(files: &[ResolvedFile]) -> String {
    let mut markdown_output = String::new();

    for resolved_file in files {
        let file_content = match std::fs::read_to_string(resolved_file.canonical_path()) {
            Ok(content) => content,
            Err(e) => {
                // This error message will be part of the markdown if a file can't be read
                // at the point of markdown generation.
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

    markdown_output
}
