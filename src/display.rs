use crate::types::{InputResolution, ResolvedFile};
use console::{Style, Term};
use std::io::{self, Write};

pub struct DisplayManager {
    term: Term,
    // Predefined styles for consistency
    pub error_style: Style,
    pub warning_style: Style,
    pub success_style: Style,
    pub filename_style: Style,
    pub metadata_style: Style,
    pub ambiguous_style: Style,
    pub preview_style: Style,
}

impl DisplayManager {
    pub fn new() -> Self {
        Self {
            term: Term::stderr(), // Use stderr for status messages, stdout for actual output
            error_style: Style::new().red().bold(),
            warning_style: Style::new().yellow(),
            success_style: Style::new().green(),
            filename_style: Style::new().cyan().bold(),
            metadata_style: Style::new().dim(),
            ambiguous_style: Style::new().magenta().bold(),
            preview_style: Style::new().white().dim(),
        }
    }

    /// Print resolution errors with proper styling
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
                    writeln!(
                        stderr,
                        "  {} {} {}",
                        self.metadata_style.apply_to("•"),
                        self.ambiguous_style
                            .apply_to(format!("Input: '{}'", input_string)),
                        self.metadata_style.apply_to("matched:")
                    )?;
                    for path in conflicting_paths {
                        writeln!(
                            stderr,
                            "    {} {}",
                            self.metadata_style.apply_to("→"),
                            self.filename_style.apply_to(format!("{:?}", path))
                        )?;
                    }
                }
            }
        }

        if !successful_files.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.success_style
                    .apply_to("Successfully resolved files (would have been included):")
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
        } else {
            writeln!(
                stderr,
                "\n{}",
                self.error_style
                    .apply_to("No files were successfully resolved.")
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

    /// Print the list of files that will be included, with previews
    pub fn print_file_preview(&self, files: &[ResolvedFile]) -> io::Result<()> {
        let mut stderr = self.term.clone();

        writeln!(
            stderr,
            "{}",
            self.success_style
                .apply_to("Files to be included in context:")
        )?;
        writeln!(stderr, "{}", self.metadata_style.apply_to("=".repeat(40)))?;

        for (i, resolved_file) in files.iter().enumerate() {
            // File header
            writeln!(
                stderr,
                "\n{} {}",
                self.metadata_style.apply_to(format!("{}.", i + 1)),
                self.filename_style
                    .apply_to(resolved_file.display_path().to_string_lossy())
            )?;

            // Try to read and show preview
            match std::fs::read_to_string(resolved_file.canonical_path()) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let total_lines = lines.len();
                    let preview_lines = std::cmp::min(5, total_lines);

                    writeln!(
                        stderr,
                        "   {} {} lines, {} bytes",
                        self.metadata_style.apply_to("📄"),
                        self.metadata_style.apply_to(total_lines.to_string()),
                        self.metadata_style.apply_to(content.len().to_string())
                    )?;

                    if preview_lines > 0 {
                        writeln!(stderr, "   {}", self.metadata_style.apply_to("Preview:"))?;
                        for (line_num, line) in lines.iter().take(preview_lines).enumerate() {
                            let truncated_line = if line.len() > 80 {
                                format!("{}...", &line[..77])
                            } else {
                                line.to_string()
                            };
                            writeln!(
                                stderr,
                                "   {} {}",
                                self.metadata_style.apply_to(format!("{:2}│", line_num + 1)),
                                self.preview_style.apply_to(truncated_line)
                            )?;
                        }
                        if total_lines > preview_lines {
                            writeln!(
                                stderr,
                                "   {} {} more lines...",
                                self.metadata_style.apply_to("  ┆"),
                                self.metadata_style
                                    .apply_to(format!("({}", total_lines - preview_lines))
                            )?;
                        }
                    }
                }
                Err(e) => {
                    writeln!(
                        stderr,
                        "   {} {}",
                        self.error_style.apply_to("⚠"),
                        self.error_style
                            .apply_to(format!("Error reading file: {}", e))
                    )?;
                }
            }
        }

        writeln!(stderr, "\n{}", self.metadata_style.apply_to("=".repeat(40)))?;
        Ok(())
    }

    /// Print clipboard status
    pub fn print_clipboard_status(
        &self,
        success: bool,
        size: usize,
        lines: usize,
        error: Option<&str>,
    ) -> io::Result<()> {
        let mut stderr = self.term.clone();

        if success {
            writeln!(
                stderr,
                "\n{} Context copied to clipboard ({} bytes, {} lines)",
                self.success_style.apply_to("✅"),
                self.metadata_style.apply_to(size.to_string()),
                self.metadata_style.apply_to(lines.to_string())
            )?;
        } else if let Some(err_msg) = error {
            writeln!(
                stderr,
                "\n{} Failed to copy to clipboard: {}",
                self.warning_style.apply_to("⚠"),
                self.warning_style.apply_to(err_msg)
            )?;
            writeln!(stderr, "   Output printed to stdout instead.")?;
        }
        Ok(())
    }
}

/// Generate the full markdown output for clipboard (unchanged from your current logic)
pub fn generate_full_markdown(files: &[ResolvedFile]) -> String {
    let mut markdown_output = String::new();

    for resolved_file in files {
        let file_content = match std::fs::read_to_string(resolved_file.canonical_path()) {
            Ok(content) => content,
            Err(e) => {
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
