use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use walkdir::WalkDir;

use crate::models::*;

/// Load all usage entries from a projects directory
pub fn load_usage_entries(projects_dir: &Path) -> Result<Vec<UsageEntry>> {
    // Collect all JSONL file paths first
    let jsonl_files: Vec<_> = WalkDir::new(projects_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl")
        })
        .map(|e| e.path().to_path_buf())
        .collect();
    
    // Process files in parallel
    let mut entries: Vec<UsageEntry> = jsonl_files
        .par_iter()
        .filter_map(|path| load_jsonl_file(path).ok())
        .flatten()
        .collect();
    
    // Sort by timestamp
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    
    Ok(entries)
}

/// Load entries from a single JSONL file
fn load_jsonl_file(path: &Path) -> Result<Vec<UsageEntry>> {
    let file = File::open(path).context("Failed to open JSONL file")?;
    let reader = BufReader::with_capacity(64 * 1024, file); // 64KB buffer
    let mut entries = Vec::with_capacity(1000); // Pre-allocate for typical file size
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = line.context("Failed to read line")?;
        if line.trim().is_empty() {
            continue;
        }
        
        match parse_jsonl_line(&line) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}, // Skipped entry
            Err(e) => {
                // Just skip invalid lines
                // Silently skip parse errors - they're expected for some entries
                #[allow(unused_variables)]
                if false {  // Set to true to debug parsing issues
                    eprintln!("Warning: Failed to parse line {} in {}: {}", 
                             line_num + 1, path.display(), e);
                }
            }
        }
    }
    
    Ok(entries)
}

/// Parse a single JSONL line into a UsageEntry
#[inline]
fn parse_jsonl_line(line: &str) -> Result<Option<UsageEntry>> {
    let raw: RawUsageEntry = serde_json::from_str(line)
        .context("Failed to parse JSON")?;
    
    // Parse timestamp
    let timestamp = DateTime::parse_from_rfc3339(&raw.timestamp)
        .context("Failed to parse timestamp")?
        .with_timezone(&Utc);
    
    // Get model name
    let model = raw.model
        .or(raw.message.model)
        .unwrap_or_else(|| "unknown".to_string());
    
    // Skip if it's not a real model
    if model == "unknown" || model.is_empty() {
        return Ok(None);
    }
    
    // Calculate total tokens
    let total_tokens = raw.message.usage.input_tokens
        + raw.message.usage.output_tokens
        + raw.message.usage.cache_creation_input_tokens
        + raw.message.usage.cache_read_input_tokens;
    
    // Create processed entry
    let mut entry = UsageEntry {
        timestamp,
        input_tokens: raw.message.usage.input_tokens,
        output_tokens: raw.message.usage.output_tokens,
        cache_creation_tokens: raw.message.usage.cache_creation_input_tokens,
        cache_read_tokens: raw.message.usage.cache_read_input_tokens,
        total_tokens,
        cost: 0.0, // Will calculate below
        model,
    };
    
    // Use provided cost or calculate
    entry.cost = raw.cost_usd.unwrap_or_else(|| entry.calculate_cost());
    
    Ok(Some(entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_jsonl_line() {
        let line = r#"{"timestamp":"2024-01-01T10:00:00Z","message":{"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"costUSD":0.001,"model":"claude-3-5-sonnet-20241022"}"#;
        
        let result = parse_jsonl_line(line).unwrap();
        assert!(result.is_some());
        
        let entry = result.unwrap();
        assert_eq!(entry.input_tokens, 100);
        assert_eq!(entry.output_tokens, 50);
        assert_eq!(entry.total_tokens, 150);
        assert_eq!(entry.model, "claude-3-5-sonnet-20241022");
    }
}