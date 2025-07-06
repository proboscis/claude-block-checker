use chrono::{DateTime, Duration, Timelike, Utc};
use std::collections::HashSet;

use crate::models::{SessionBlock, UsageEntry};

const SESSION_DURATION_HOURS: i64 = 5;

/// Identify session blocks from usage entries
pub fn identify_session_blocks(entries: Vec<UsageEntry>) -> Vec<SessionBlock> {
    if entries.is_empty() {
        return Vec::new();
    }
    
    let mut blocks = Vec::new();
    let mut current_block_start: Option<DateTime<Utc>> = None;
    let mut current_block_entries = Vec::new();
    let now = Utc::now();
    
    for entry in entries {
        if let Some(block_start) = current_block_start {
            let time_since_start = entry.timestamp - block_start;
            
            // Check if this entry belongs to current block
            if time_since_start < Duration::hours(SESSION_DURATION_HOURS) {
                current_block_entries.push(entry);
            } else {
                // Close current block and start new one
                if !current_block_entries.is_empty() {
                    blocks.push(create_block(block_start, current_block_entries, now));
                }
                
                // Start new block
                current_block_start = Some(floor_to_hour(entry.timestamp));
                current_block_entries = vec![entry];
            }
        } else {
            // First entry - start first block
            current_block_start = Some(floor_to_hour(entry.timestamp));
            current_block_entries.push(entry);
        }
    }
    
    // Close final block
    if let Some(block_start) = current_block_start {
        if !current_block_entries.is_empty() {
            blocks.push(create_block(block_start, current_block_entries, now));
        }
    }
    
    blocks
}

/// Floor timestamp to the beginning of the hour
fn floor_to_hour(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    timestamp
        .with_minute(0).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap()
}

/// Create a session block from entries
fn create_block(
    start_time: DateTime<Utc>,
    entries: Vec<UsageEntry>,
    now: DateTime<Utc>,
) -> SessionBlock {
    let end_time = start_time + Duration::hours(SESSION_DURATION_HOURS);
    let is_active = now >= start_time && now < end_time;
    
    // Aggregate tokens and costs
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cache_creation_tokens = 0u64;
    let mut cache_read_tokens = 0u64;
    let mut total_cost = 0.0;
    let mut models = HashSet::new();
    
    for entry in &entries {
        input_tokens += entry.input_tokens;
        output_tokens += entry.output_tokens;
        cache_creation_tokens += entry.cache_creation_tokens;
        cache_read_tokens += entry.cache_read_tokens;
        total_cost += entry.cost;
        models.insert(entry.model.clone());
    }
    
    let total_tokens = input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens;
    
    SessionBlock {
        start_time,
        end_time,
        is_active,
        input_tokens,
        output_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        total_tokens,
        total_cost,
        models: models.into_iter().collect(),
        entry_count: entries.len(),
        burn_rate: None, // Will be calculated in main.rs when detailed mode is on
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_floor_to_hour() {
        let timestamp = DateTime::parse_from_rfc3339("2024-01-01T10:35:45Z")
            .unwrap()
            .with_timezone(&Utc);
        
        let floored = floor_to_hour(timestamp);
        assert_eq!(floored.hour(), 10);
        assert_eq!(floored.minute(), 0);
        assert_eq!(floored.second(), 0);
    }
    
    #[test]
    fn test_identify_blocks_single() {
        let entries = vec![
            UsageEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z").unwrap().with_timezone(&Utc),
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 150,
                cost: 0.001,
                model: "claude-3-5-sonnet".to_string(),
            },
            UsageEntry {
                timestamp: DateTime::parse_from_rfc3339("2024-01-01T11:00:00Z").unwrap().with_timezone(&Utc),
                input_tokens: 200,
                output_tokens: 100,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                total_tokens: 300,
                cost: 0.002,
                model: "claude-3-5-sonnet".to_string(),
            },
        ];
        
        let blocks = identify_session_blocks(entries);
        assert_eq!(blocks.len(), 1);
        
        let block = &blocks[0];
        assert_eq!(block.total_tokens, 450);
        assert_eq!(block.input_tokens, 300);
        assert_eq!(block.output_tokens, 150);
        assert_eq!(block.entry_count, 2);
    }
}