use crate::error::AppError; // We'll define this in the next step
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub working_dir: PathBuf,
    // We can add other configuration options here later if needed
    // e.g., verbosity, ignored patterns, etc.
}

impl Config {
    /// Creates a new Config instance.
    ///
    /// Initializes the working directory based on the current environment.
    pub fn new() -> Result<Self, AppError> {
        let working_dir = env::current_dir().map_err(|io_err| {
            AppError::IoError(format!(
                "Failed to determine current working directory: {}",
                io_err
            ))
        })?;
        Ok(Config { working_dir })
    }
}
