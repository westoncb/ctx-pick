use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String), // If Config::new() or other config steps fail

    #[error("File resolution failed for input '{input}': {message}")]
    ResolutionError { input: String, message: String },
    // This variant can be used by main.rs to summarize if any inputs
    // resulted in Ambiguous, NotFound, or PathDoesNotExist after the initial processing.

    // It might also be useful to have more specific errors that
    // file_resolver.rs could return if it were fallible, which could
    // then be transformed into InputResolution variants or an AppError.
    // For now, with an infallible resolve_input_string, these might be less used internally.

    // Example of a more specific error if needed later:
    // #[error("Path '{path}' could not be canonicalized: {source}")]
    // CanonicalizationError { path: PathBuf, source: std::io::Error },
}

// If we want to easily convert std::io::Error into AppError::IoError:
// (Option 1: Generic conversion - careful not to overuse if more specific mapping is better)
// impl From<std::io::Error> for AppError {
//     fn from(err: std::io::Error) -> Self {
//         AppError::IoError(err.to_string())
//     }
// }
// (Option 2: thiserror's #[from] if the variant is structured for it,
// e.g., #[error("I/O error")] Io(#[from] std::io::Error) )
// For now, explicit mapping like in Config::new is clear.
