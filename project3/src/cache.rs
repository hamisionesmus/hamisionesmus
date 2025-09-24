//! Cache implementation with LRU eviction and persistence

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub value: Vec<u8>,
    pub expires_at: Option<Instant>,
    pub created_at: Instant,
    pub access_count: u64,
    pub last_accessed: Instant,
}

impl CacheEntry {
    pub fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            expires_at: ttl.map(|t| now + t),
            created_at: now,
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |expires| Instant::now() > expires)
    }

    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }

    pub fn size(&self) -> usize {
        std::mem::size_of::<Self>() + self.value.len()
    }
}

pub struct Cache {
    data: HashMap<String, CacheEntry>,
    max_memory: usize,
    current_memory: usize,
}

impl Cache {
    pub fn new(max_memory: usize) -> Self {
        Self {
            data: HashMap::new(),
            max_memory,
            current_memory: 0,
        }
    }

    pub async fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.data.get_mut(key) {
            if entry.is_expired() {
                self.remove_expired(key);
                return None;
            }
            entry.access();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub async fn set(&mut self, key: String, value: Vec<u8>, ttl: Option<Duration>) {
        let entry = CacheEntry::new(value, ttl);
        let entry_size = entry.size();

        // Remove expired entries first
        self.remove_expired_entries();

        // If adding this entry would exceed memory limit, evict some entries
        while self.current_memory + entry_size > self.max_memory && !self.data.is_empty() {
            self.evict_lru();
        }

        // Remove old entry if it exists
        if let Some(old_entry) = self.data.remove(&key) {
            self.current_memory -= old_entry.size();
        }

        self.current_memory += entry_size;
        self.data.insert(key, entry);
    }

    pub async fn delete(&mut self, key: &str) -> bool {
        if let Some(entry) = self.data.remove(key) {
            self.current_memory -= entry.size();
            true
        } else {
            false
        }
    }

    pub async fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key) && !self.data[key].is_expired()
    }

    pub async fn clear(&mut self) {
        self.data.clear();
        self.current_memory = 0;
    }

    pub async fn stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("total_keys".to_string(), self.data.len() as u64);
        stats.insert("memory_used".to_string(), self.current_memory as u64);
        stats.insert("max_memory".to_string(), self.max_memory as u64);

        let total_accesses: u64 = self.data.values().map(|e| e.access_count).sum();
        stats.insert("total_accesses".to_string(), total_accesses);

        stats
    }

    fn remove_expired_entries(&mut self) {
        let expired_keys: Vec<String> = self.data.iter()
            .filter(|(_, entry)| entry.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.remove_expired(&key);
        }
    }

    fn remove_expired(&mut self, key: &str) {
        if let Some(entry) = self.data.remove(key) {
            self.current_memory -= entry.size();
        }
    }

    fn evict_lru(&mut self) {
        if let Some((key, entry)) = self.data.iter()
            .min_by_key(|(_, entry)| entry.last_accessed) {
            let key = key.clone();
            let entry = self.data.remove(&key).unwrap();
            self.current_memory -= entry.size();
        }
    }
}

pub type SharedCache = Arc<RwLock<Cache>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_operations() {
        let mut cache = Cache::new(1024 * 1024); // 1MB

        // Test set and get
        cache.set("key1".to_string(), b"value1".to_vec(), None).await;
        assert_eq!(cache.get("key1").await, Some(b"value1".to_vec()));

        // Test delete
        assert!(cache.delete("key1").await);
        assert_eq!(cache.get("key1").await, None);

        // Test exists
        cache.set("key2".to_string(), b"value2".to_vec(), None).await;
        assert!(cache.exists("key2").await);
        assert!(!cache.exists("nonexistent").await);
    }

    #[tokio::test]
    async fn test_ttl() {
        let mut cache = Cache::new(1024 * 1024);

        cache.set("ttl_key".to_string(), b"ttl_value".to_vec(), Some(Duration::from_millis(10))).await;

        // Should exist immediately
        assert!(cache.exists("ttl_key").await);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be expired
        assert!(!cache.exists("ttl_key").await);
        assert_eq!(cache.get("ttl_key").await, None);
    }

    #[tokio::test]
    async fn test_memory_limits() {
        let mut cache = Cache::new(100); // Very small limit

        // Add entries until we hit the limit
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = vec![0u8; 20]; // 20 bytes each
            cache.set(key, value, None).await;
        }

        // Should have evicted some entries to stay within memory limit
        let stats = cache.stats().await;
        assert!(stats["memory_used"] <= stats["max_memory"]);
    }
}