use std::process::Command;
use anyhow::{Result, Context as AnyhowContext};

#[derive(Debug)]
pub enum VerificationResult {
    Success,
    Failure(String),
}

pub struct Verifier;

impl Verifier {
    pub fn new() -> Self {
        Self
    }

    /// Verifies the Rust code by running `rustc <file_path> --crate-type bin --emit=metadata -o <temp_path>`.
    /// Returns `Success` if compilation passes, or `Failure` with a cleaned error message.
    pub fn verify(&self, file_path: &std::path::Path) -> Result<VerificationResult> {
        let output = Command::new("rustc")
            .arg(file_path)
            .arg("--crate-type")
            .arg("bin")
            .arg("--emit=metadata")
            .arg("-o")
            .arg(file_path.with_extension("verify"))
            .output()
            .context("Failed to execute rustc check")?;

        if output.status.success() {
            Ok(VerificationResult::Success)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let cleaned_error = Self::clean_error_message(&stderr);
            Ok(VerificationResult::Failure(cleaned_error))
        }
    }

    /// Extract only the most relevant error information for LLM feedback.
    /// Keeps `error[EXXXX]:` lines and up to 3 lines of context per error.
    fn clean_error_message(stderr: &str) -> String {
        let lines: Vec<&str> = stderr.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Capture error lines and their immediate context
            if line.starts_with("error") {
                result.push(lines[i].to_string());
                // Capture up to 3 context lines after each error
                let context_end = (i + 4).min(lines.len());
                for j in (i + 1)..context_end {
                    let ctx = lines[j].trim();
                    if !ctx.is_empty() && !ctx.starts_with("error") {
                        result.push(lines[j].to_string());
                    }
                }
            }
            i += 1;
        }

        // Deduplicate and truncate
        result.dedup();
        let joined = result.join("\n");

        // Hard limit: 500 chars to prevent prompt bloat
        if joined.len() > 500 {
            format!("{}...", &joined[..500])
        } else {
            joined
        }
    }
}
