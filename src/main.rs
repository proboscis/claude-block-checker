use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use colored::*;
use num_format::{Locale, ToFormattedString};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::Path;

mod models;
mod parser;
mod blocks;

use crate::models::*;
use crate::parser::*;
use crate::blocks::*;

#[derive(Parser)]
#[command(name = "claude-block-checker")]
#[command(about = "Check Claude Code usage for current billing blocks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Profile name to check (if not specified, checks all profiles)
    #[arg(short, long)]
    profile: Option<String>,
    
    /// Show detailed breakdown
    #[arg(short = 'd', long)]
    detailed: bool,
    
    /// Output in JSON format
    #[arg(short, long)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available profiles
    List,
    
    /// Check current block for profile(s)
    Check {
        /// Specific profile to check
        profile: Option<String>,
    },
    
    /// Show current block for all profiles (default)
    All,
}

#[derive(Debug, Serialize)]
struct ProfileUsage {
    name: String,
    active_block: Option<SessionBlock>,
    total_tokens: u64,
    total_cost: f64,
    models_used: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let profiles_dir = home::home_dir()
        .context("Could not find home directory")?
        .join("claude-profiles");
    
    if !profiles_dir.exists() {
        eprintln!("{}", "Error: ~/claude-profiles directory not found".red());
        std::process::exit(1);
    }
    
    match cli.command {
        Some(Commands::List) => list_profiles(&profiles_dir),
        Some(Commands::Check { profile }) => {
            if let Some(profile_name) = profile.or(cli.profile) {
                check_single_profile(&profiles_dir, &profile_name, cli.detailed, cli.json)
            } else {
                check_all_profiles(&profiles_dir, cli.detailed, cli.json)
            }
        }
        Some(Commands::All) | None => {
            check_all_profiles(&profiles_dir, cli.detailed, cli.json)
        }
    }
}

fn list_profiles(profiles_dir: &Path) -> Result<()> {
    println!("{}", "Available Claude Profiles:".bold().green());
    
    let mut profiles = Vec::new();
    for entry in fs::read_dir(profiles_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') {
                    profiles.push(name.to_string());
                }
            }
        }
    }
    
    profiles.sort();
    for profile in profiles {
        println!("  • {}", profile.cyan());
    }
    
    Ok(())
}

fn check_single_profile(
    profiles_dir: &Path,
    profile_name: &str,
    detailed: bool,
    json: bool,
) -> Result<()> {
    let profile_path = profiles_dir.join(profile_name);
    
    if !profile_path.exists() {
        eprintln!("{}", format!("Profile '{}' not found", profile_name).red());
        std::process::exit(1);
    }
    
    let usage = check_profile(&profile_path, profile_name, detailed)?;
    
    if json {
        println!("{}", serde_json::to_string_pretty(&usage)?);
    } else {
        print_profile_usage(&usage, detailed);
    }
    
    Ok(())
}

fn check_all_profiles(profiles_dir: &Path, detailed: bool, json: bool) -> Result<()> {
    let mut all_usage = Vec::new();
    let mut total_tokens = 0u64;
    let mut total_cost = 0.0f64;
    let mut active_count = 0;
    
    // Get all profiles
    let mut profiles = Vec::new();
    for entry in fs::read_dir(profiles_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !name.starts_with('.') {
                    profiles.push((name.to_string(), path));
                }
            }
        }
    }
    
    profiles.sort_by(|a, b| a.0.cmp(&b.0));
    
    if !json {
        println!("{}", "Claude Code Usage - Current Block Report".bold().green());
        println!("Time: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        println!("Found {} profiles\n", profiles.len());
    }
    
    // Check each profile in parallel
    let results: Vec<(String, Result<ProfileUsage>)> = profiles
        .par_iter()
        .map(|(name, path)| {
            (name.clone(), check_profile(path, name, detailed))
        })
        .collect();
    
    // Process results in order
    for (name, result) in results {
        match result {
            Ok(usage) => {
                if usage.active_block.is_some() {
                    active_count += 1;
                    total_tokens += usage.total_tokens;
                    total_cost += usage.total_cost;
                }
                
                if !json {
                    print_profile_usage(&usage, detailed);
                }
                
                all_usage.push(usage);
            }
            Err(e) => {
                if !json {
                    println!("{} {}", format!("Profile: {}", name).bold().blue(), 
                             format!("(Error: {})", e).red());
                }
            }
        }
    }
    
    if json {
        let output = serde_json::json!({
            "profiles": all_usage,
            "summary": {
                "total_profiles": all_usage.len(),
                "active_profiles": active_count,
                "total_tokens": total_tokens,
                "total_cost": total_cost,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Print summary
        println!("\n{}", "━━━ Summary ━━━".bold().green());
        println!("Active profiles: {}/{}", active_count, all_usage.len());
        if active_count > 0 {
            println!("\n{}", "Aggregate Totals:".bold());
            println!("  Total Tokens: {}", total_tokens.to_formatted_string(&Locale::en));
            println!("  Total Cost:   ${:.4}", total_cost);
        }
    }
    
    Ok(())
}

fn check_profile(profile_path: &Path, profile_name: &str, detailed: bool) -> Result<ProfileUsage> {
    let projects_dir = profile_path.join("projects");
    
    if !projects_dir.exists() {
        return Ok(ProfileUsage {
            name: profile_name.to_string(),
            active_block: None,
            total_tokens: 0,
            total_cost: 0.0,
            models_used: Vec::new(),
        });
    }
    
    // Load all usage entries
    let entries = load_usage_entries(&projects_dir)?;
    
    if entries.is_empty() {
        return Ok(ProfileUsage {
            name: profile_name.to_string(),
            active_block: None,
            total_tokens: 0,
            total_cost: 0.0,
            models_used: Vec::new(),
        });
    }
    
    // Identify session blocks
    let blocks = identify_session_blocks(entries);
    
    // Find active block
    let mut active_block = blocks.into_iter()
        .find(|block| block.is_active);
    
    // Calculate burn rate if detailed mode and active block exists
    if detailed {
        if let Some(ref mut block) = active_block {
            let now = Utc::now();
            let elapsed = (now - block.start_time).num_minutes() as f64;
            if elapsed > 1.0 {
                let tokens_per_minute = (block.total_tokens as f64 / elapsed) as u64;
                let cost_per_hour = (block.total_cost / elapsed) * 60.0;
                
                let remaining_minutes = (5.0 * 60.0) - elapsed;
                let projected_tokens = if remaining_minutes > 0.0 {
                    (block.total_tokens as f64 + (tokens_per_minute as f64 * remaining_minutes)) as u64
                } else {
                    block.total_tokens
                };
                let projected_cost = if remaining_minutes > 0.0 {
                    block.total_cost + (cost_per_hour * remaining_minutes / 60.0)
                } else {
                    block.total_cost
                };
                
                block.burn_rate = Some(models::BurnRate {
                    elapsed_minutes: elapsed as u64,
                    tokens_per_minute,
                    cost_per_hour,
                    projected_tokens,
                    projected_cost,
                });
            }
        }
    }
    
    if let Some(ref block) = active_block {
        Ok(ProfileUsage {
            name: profile_name.to_string(),
            total_tokens: block.total_tokens,
            total_cost: block.total_cost,
            models_used: block.models.clone(),
            active_block,
        })
    } else {
        Ok(ProfileUsage {
            name: profile_name.to_string(),
            active_block: None,
            total_tokens: 0,
            total_cost: 0.0,
            models_used: Vec::new(),
        })
    }
}

fn print_profile_usage(usage: &ProfileUsage, detailed: bool) {
    println!("{} {}", "━━━ Profile:".bold().blue(), usage.name.bold().blue());
    
    if let Some(ref block) = usage.active_block {
        println!("  {} Active Block", "●".green());
        println!("  Started: {}", block.start_time.format("%Y-%m-%d %H:%M:%S UTC"));
        
        // Time remaining
        let now = Utc::now();
        let end_time = block.start_time + Duration::hours(5);
        if now < end_time {
            let remaining = end_time - now;
            let hours = remaining.num_hours();
            let minutes = remaining.num_minutes() % 60;
            println!("  Remaining: {}h {}m", hours, minutes);
        } else {
            println!("  Status: {}", "Expired".yellow());
        }
        
        if !block.models.is_empty() {
            println!("  Models: {}", block.models.join(", "));
        }
        
        println!("\n  {}:", "Token Usage".bold());
        println!("    Input:  {}", block.input_tokens.to_formatted_string(&Locale::en));
        println!("    Output: {}", block.output_tokens.to_formatted_string(&Locale::en));
        if block.cache_creation_tokens > 0 {
            println!("    Cache+: {}", block.cache_creation_tokens.to_formatted_string(&Locale::en));
        }
        if block.cache_read_tokens > 0 {
            println!("    Cache-: {}", block.cache_read_tokens.to_formatted_string(&Locale::en));
        }
        println!("    {}: {}", "Total".bold(), block.total_tokens.to_formatted_string(&Locale::en));
        
        println!("\n  {}: ${:.6}", "Cost".bold(), block.total_cost);
        
        // Burn rate and projections
        if detailed {
            let elapsed = (now - block.start_time).num_minutes() as f64;
            if elapsed > 1.0 {
                let tokens_per_minute = block.total_tokens as f64 / elapsed;
                let cost_per_hour = (block.total_cost / elapsed) * 60.0;
                
                println!("\n  {}:", "Burn Rate".bold());
                println!("    {} tokens/min", (tokens_per_minute as u64).to_formatted_string(&Locale::en));
                println!("    ${:.4}/hour", cost_per_hour);
                
                let remaining_minutes = (5.0 * 60.0) - elapsed;
                if remaining_minutes > 0.0 {
                    let projected_tokens = block.total_tokens as f64 + (tokens_per_minute * remaining_minutes);
                    let projected_cost = block.total_cost + (cost_per_hour * remaining_minutes / 60.0);
                    
                    println!("\n  {}:", "Projected (5h)".bold());
                    println!("    Tokens: {}", (projected_tokens as u64).to_formatted_string(&Locale::en));
                    println!("    Cost:   ${:.4}", projected_cost);
                }
            }
        }
        
        println!();
    } else {
        println!("  {}", "No active block".yellow());
        println!();
    }
}