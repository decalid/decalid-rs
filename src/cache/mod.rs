//! Cache module for caching logic
//!
//! This module contains the caching logic for the application.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// A simple cache implementation
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// The cached items
    items: HashMap<K, (V, Instant)>,

    /// The TTL for cached items
    ttl: Duration,

    /// The maximum number of items in the cache
    max_size: usize,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new cache
    pub fn new(ttl_seconds: u64, max_size: usize) -> Self {
        Self {
            items: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            max_size,
        }
    }

    /// Get an item from the cache
    pub fn get(&mut self, key: &K) -> Option<V> {
        self.cleanup();

        self.items.get(key).map(|(value, _)| value.clone())
    }

    /// Insert an item into the cache
    pub fn insert(&mut self, key: K, value: V) {
        self.cleanup();

        // If we're at capacity, remove the oldest item
        if self.items.len() >= self.max_size {
            if let Some((oldest_key, _)) = self
                .items
                .iter()
                .min_by_key(|(_, (_, instant))| instant)
                .map(|(k, _)| (k.clone(), ()))
            {
                self.items.remove(&oldest_key);
            }
        }

        self.items.insert(key, (value, Instant::now()));
    }

    /// Remove an item from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.items.remove(key).map(|(value, _)| value)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Cleanup expired items
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.items
            .retain(|_, (_, instant)| now.duration_since(*instant) < self.ttl);
    }
}
