use crate::types::{FileContext, InputResolution, ResolvedFile};
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

// --- Public API ---

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
    /// This function orchestrates the printing of different error sections.
    pub fn print_resolution_errors(
        &self,
        path_errors: &[&InputResolution],
        not_founds: &[&InputResolution],
        ambiguities: &[&InputResolution],
        invalid_globs: &[&InputResolution],
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
                self.report_path_does_not_exist_case(&mut stderr, case)?;
            }
        }

        if !invalid_globs.is_empty() {
            writeln!(
                stderr,
                "\n{}",
                self.error_style
                    .apply_to("The following glob patterns are invalid:")
            )?;
            for case in invalid_globs {
                self.report_invalid_glob_case(&mut stderr, case)?;
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
                self.report_not_found_case(&mut stderr, case)?;
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
                self.report_ambiguous_case(&mut stderr, case)?;
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
                self.report_successful_file_case(&mut stderr, resolved_file)?;
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
    pub fn print_operation_summary_and_preview(
        &self,
        contexts: &[FileContext], // <-- Receives the new struct
        clipboard_result: &Result<(), arboard::Error>,
        output_count: usize,
        unit_str: &str,
        depth: Option<usize>,
    ) -> io::Result<()> {
        let mut stderr = self.term.clone();
        let summary_verb = if depth.is_some() {
            "Context skeleton copied"
        } else {
            "Context copied"
        };
        let file_count = contexts.len();

        match clipboard_result {
            Ok(_) => {
                writeln!(
                    stderr,
                    "\n{} {} to clipboard ({} {}, {} {})",
                    self.success_style.apply_to("✅"),
                    summary_verb,
                    self.metadata_style.apply_to(file_count.to_string()),
                    self.metadata_style
                        .apply_to(if file_count == 1 { "file" } else { "files" }),
                    self.metadata_style.apply_to(output_count.to_string()),
                    self.metadata_style.apply_to(unit_str)
                )?;
            }
            Err(err) => {
                // ... (error case is unchanged) ...
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

        if contexts.is_empty() {
            writeln!(
                stderr,
                "  {}",
                self.metadata_style.apply_to("(No files to preview)")
            )?;
        } else {
            for (i, context) in contexts.iter().enumerate() {
                let (icon, label) = if let Some(d) = depth {
                    (
                        "🧬",
                        format!("{} (skeleton only; depth={})", context.display_path, d),
                    )
                } else {
                    ("📄", context.display_path.clone())
                };

                let (metric_value, metric_unit) = if depth.is_some() {
                    // Skeleton mode: count characters from the context's content.
                    (context.content.chars().count(), "characters")
                } else {
                    // Full file mode: count lines from the context's content.
                    (context.content.lines().count(), "lines")
                };

                writeln!(
                    stderr,
                    "\n{}. {}",
                    self.metadata_style.apply_to(format!("{}", i + 1)),
                    self.filename_style.apply_to(label)
                )?;

                writeln!(
                    stderr,
                    "    {} {} {}", // e.g., "📄 125 lines" or "🧬 850 characters"
                    self.metadata_style.apply_to(icon),
                    self.metadata_style.apply_to(metric_value.to_string()),
                    self.metadata_style.apply_to(metric_unit)
                )?;
            }
        }
        writeln!(stderr, "\n{}", self.metadata_style.apply_to("=".repeat(40)))?;
        Ok(())
    }

    // --- Private Error Reporters ---

    fn report_path_does_not_exist_case(
        &self,
        stderr: &mut Term,
        case: &InputResolution,
    ) -> io::Result<()> {
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
        Ok(())
    }

    fn report_invalid_glob_case(
        &self,
        stderr: &mut Term,
        case: &InputResolution,
    ) -> io::Result<()> {
        if let InputResolution::InvalidGlobPattern {
            input_string,
            error,
        } = case
        {
            writeln!(
                stderr,
                "  {} {} {}",
                self.metadata_style.apply_to("•"),
                self.error_style
                    .apply_to(format!("Input: '{}'", input_string)),
                self.metadata_style.apply_to(format!("(error: {})", error))
            )?;
        }
        Ok(())
    }

    fn report_not_found_case(&self, stderr: &mut Term, case: &InputResolution) -> io::Result<()> {
        if let InputResolution::NotFound { input_string } = case {
            writeln!(
                stderr,
                "  {} {}",
                self.metadata_style.apply_to("•"),
                self.warning_style
                    .apply_to(format!("Input: '{}'", input_string))
            )?;
        }
        Ok(())
    }

    fn report_ambiguous_case(&self, stderr: &mut Term, case: &InputResolution) -> io::Result<()> {
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

            const MAX_TO_SHOW: usize = 8;
            for (i, path) in conflicting_paths.iter().enumerate() {
                if i < MAX_TO_SHOW {
                    writeln!(
                        stderr,
                        "    {} {}",
                        self.metadata_style.apply_to("→"),
                        self.filename_style.apply_to(format!("{:?}", path))
                    )?;
                } else {
                    let remaining = conflicting_paths.len() - MAX_TO_SHOW;
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
        Ok(())
    }

    fn report_successful_file_case(
        &self,
        stderr: &mut Term,
        resolved_file: &ResolvedFile,
    ) -> io::Result<()> {
        writeln!(
            stderr,
            "  {} {}",
            self.metadata_style.apply_to("✓"),
            self.filename_style
                .apply_to(format!("{:?}", resolved_file.display_path()))
        )
    }
}
