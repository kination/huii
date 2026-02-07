use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct LockData {
    pub source_hash: String,
    pub generated_code: String,
    pub verified_at: String,
    pub passed_tests: Vec<String>,
}

pub struct LockManager;

impl LockManager {
    pub fn compute_hash(content: &str) -> String {
        let hash = blake3::hash(content.as_bytes());
        hash.to_hex().to_string()
    }

    pub fn load(path: &Path) -> Result<Option<LockData>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let data: LockData = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    pub fn save(path: &Path, data: &LockData) -> Result<()> {
        let content = serde_json::to_string_pretty(data)?;
        fs::write(path, content)?;
        Ok(())
    }
}