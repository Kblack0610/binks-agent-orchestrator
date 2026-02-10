//! LRU caching for screen details

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};

use lru::LruCache;
use parking_lot::Mutex;

use crate::types::ScreenMetadata;

/// LRU cache for screen metadata with hit/miss tracking
pub struct ScreenCache {
    cache: Mutex<LruCache<u32, ScreenMetadata>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl ScreenCache {
    /// Create a new cache with the given capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get a cached screen, tracking hits/misses
    pub fn get(&self, screen_id: u32) -> Option<ScreenMetadata> {
        let mut cache = self.cache.lock();
        match cache.get(&screen_id) {
            Some(meta) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(meta.clone())
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert a screen into the cache
    pub fn insert(&self, screen_id: u32, metadata: ScreenMetadata) {
        let mut cache = self.cache.lock();
        cache.put(screen_id, metadata);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock();
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }
}

/// Cache statistics
#[derive(Clone, Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    #[allow(dead_code)]
    pub capacity: usize,
}

impl CacheStats {
    /// Calculate hit rate as a percentage
    #[allow(dead_code)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}
