use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw usage data from JSONL files
#[derive(Debug, Clone, Deserialize)]
pub struct RawUsageEntry {
    pub timestamp: String,
    pub message: MessageData,
    #[serde(rename = "costUSD")]
    pub cost_usd: Option<f64>,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageData {
    pub usage: Usage,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// Processed usage entry
#[derive(Debug, Clone)]
pub struct UsageEntry {
    pub timestamp: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_tokens: u64,
    pub cost: f64,
    pub model: String,
}

/// Session block (5-hour billing period)
#[derive(Debug, Clone, Serialize)]
pub struct SessionBlock {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_active: bool,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub models: Vec<String>,
    pub entry_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burn_rate: Option<BurnRate>,
}

/// Burn rate and projections
#[derive(Debug, Clone, Serialize)]
pub struct BurnRate {
    pub elapsed_minutes: u64,
    pub tokens_per_minute: u64,
    pub cost_per_hour: f64,
    pub projected_tokens: u64,
    pub projected_cost: f64,
}

/// Model pricing information
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub cache_creation_cost_per_million: f64,
    pub cache_read_cost_per_million: f64,
}

impl Default for ModelPricing {
    fn default() -> Self {
        // Default Claude pricing (you can update these)
        Self {
            input_cost_per_million: 3.0,    // $3 per million tokens
            output_cost_per_million: 15.0,  // $15 per million tokens
            cache_creation_cost_per_million: 3.75,
            cache_read_cost_per_million: 0.30,
        }
    }
}

/// Get pricing for a specific model
pub fn get_model_pricing(model: &str) -> ModelPricing {
    let pricing_map: HashMap<&str, ModelPricing> = HashMap::from([
        ("claude-3-5-sonnet-20241022", ModelPricing {
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
            cache_creation_cost_per_million: 3.75,
            cache_read_cost_per_million: 0.30,
        }),
        ("claude-3-5-sonnet-20240620", ModelPricing {
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
            cache_creation_cost_per_million: 3.75,
            cache_read_cost_per_million: 0.30,
        }),
        ("claude-3-5-haiku-20241022", ModelPricing {
            input_cost_per_million: 1.0,
            output_cost_per_million: 5.0,
            cache_creation_cost_per_million: 1.25,
            cache_read_cost_per_million: 0.10,
        }),
        ("claude-3-opus-20240229", ModelPricing {
            input_cost_per_million: 15.0,
            output_cost_per_million: 75.0,
            cache_creation_cost_per_million: 18.75,
            cache_read_cost_per_million: 1.50,
        }),
        ("claude-3-sonnet-20240229", ModelPricing {
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
            cache_creation_cost_per_million: 3.75,
            cache_read_cost_per_million: 0.30,
        }),
        ("claude-3-haiku-20240307", ModelPricing {
            input_cost_per_million: 0.25,
            output_cost_per_million: 1.25,
            cache_creation_cost_per_million: 0.30,
            cache_read_cost_per_million: 0.03,
        }),
        // Claude 4 models
        ("claude-sonnet-4-20250514", ModelPricing {
            input_cost_per_million: 3.0,
            output_cost_per_million: 15.0,
            cache_creation_cost_per_million: 3.75,
            cache_read_cost_per_million: 0.30,
        }),
        ("claude-opus-4-20250514", ModelPricing {
            input_cost_per_million: 15.0,
            output_cost_per_million: 75.0,
            cache_creation_cost_per_million: 18.75,
            cache_read_cost_per_million: 1.50,
        }),
    ]);
    
    // Try exact match first
    if let Some(pricing) = pricing_map.get(model) {
        return pricing.clone();
    }
    
    // Try to match by model family
    if model.contains("opus") {
        ModelPricing {
            input_cost_per_million: 15.0,
            output_cost_per_million: 75.0,
            cache_creation_cost_per_million: 18.75,
            cache_read_cost_per_million: 1.50,
        }
    } else if model.contains("haiku") {
        ModelPricing {
            input_cost_per_million: 0.25,
            output_cost_per_million: 1.25,
            cache_creation_cost_per_million: 0.30,
            cache_read_cost_per_million: 0.03,
        }
    } else {
        // Default to Sonnet pricing
        ModelPricing::default()
    }
}

impl UsageEntry {
    /// Calculate cost for this entry
    pub fn calculate_cost(&self) -> f64 {
        let pricing = get_model_pricing(&self.model);
        
        let input_cost = (self.input_tokens as f64 / 1_000_000.0) * pricing.input_cost_per_million;
        let output_cost = (self.output_tokens as f64 / 1_000_000.0) * pricing.output_cost_per_million;
        let cache_creation_cost = (self.cache_creation_tokens as f64 / 1_000_000.0) * pricing.cache_creation_cost_per_million;
        let cache_read_cost = (self.cache_read_tokens as f64 / 1_000_000.0) * pricing.cache_read_cost_per_million;
        
        input_cost + output_cost + cache_creation_cost + cache_read_cost
    }
}