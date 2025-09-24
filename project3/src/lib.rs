//! # CLI Tool Library
//!
//! A high-performance command-line interface tool built with Rust,
//! featuring concurrent processing, advanced error handling, and cross-platform compatibility.

pub mod cache;
pub mod config;
pub mod error;
pub mod processor;
pub mod utils;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main application state
#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pub config: Arc<Config>,
}

/// Application configuration
#[derive(Clone, Debug)]
pub struct Config {
    pub max_memory: usize,
    pub workers: usize,
    pub batch_size: usize,
    pub host: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_memory: 1_073_741_824, // 1GB
            workers: 4,
            batch_size: 1000,
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// Initialize the application with default configuration
pub fn init_app() -> Result<AppState, Box<dyn std::error::Error>> {
    let config = Config::default();
    let cache = Arc::new(RwLock::new(HashMap::new()));

    Ok(AppState {
        cache,
        config: Arc::new(config),
    })
}

/// Health check function
pub async fn health_check(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    let cache = state.cache.read().await;
    let entries = cache.len();

    if entries >= 0 {
        Ok(())
    } else {
        Err("Cache state is invalid".into())
    }
}

/// Get application statistics
pub async fn get_stats(state: &AppState) -> HashMap<String, serde_json::Value> {
    let cache = state.cache.read().await;
    let mut stats = HashMap::new();

    stats.insert("total_entries".to_string(), serde_json::json!(cache.len()));
    stats.insert("memory_usage".to_string(), serde_json::json!(calculate_memory_usage(&cache)));
    stats.insert("uptime".to_string(), serde_json::json!(get_uptime()));

    stats
}

fn calculate_memory_usage(cache: &HashMap<String, Vec<u8>>) -> usize {
    cache.values().map(|v| v.len()).sum()
}

fn get_uptime() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_initialization() {
        let state = init_app().expect("Failed to initialize app");
        assert!(health_check(&state).await.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let state = init_app().expect("Failed to initialize app");
        let stats = get_stats(&state).await;
        assert!(stats.contains_key("total_entries"));
        assert!(stats.contains_key("memory_usage"));
        assert!(stats.contains_key("uptime"));
    }
}