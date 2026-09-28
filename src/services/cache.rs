//! Tiny in-process TTL cache for public read-mostly data.
//!
//! The previous application cached public queries for six hours through
//! Next.js `unstable_cache`. This is the equivalent for a long-running
//! server: one cached value per data set, refreshed after the TTL expires
//! and invalidated explicitly whenever an admin mutation changes the
//! underlying collection.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub struct TtlCache<T> {
    slot: RwLock<Option<(Instant, Arc<T>)>>,
    ttl: Duration,
}

impl<T> TtlCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            slot: RwLock::new(None),
            ttl,
        }
    }

    /// Returns the cached value when it is still fresh.
    pub fn get(&self) -> Option<Arc<T>> {
        let guard = self.slot.read().expect("cache lock poisoned");
        match guard.as_ref() {
            Some((stored_at, value)) if stored_at.elapsed() < self.ttl => Some(Arc::clone(value)),
            _ => None,
        }
    }

    /// Stores a freshly loaded value and returns it wrapped in `Arc`.
    pub fn put(&self, value: T) -> Arc<T> {
        let value = Arc::new(value);
        let mut guard = self.slot.write().expect("cache lock poisoned");
        *guard = Some((Instant::now(), Arc::clone(&value)));
        value
    }

    /// Drops the cached value so the next read reloads from the database.
    pub fn invalidate(&self) {
        let mut guard = self.slot.write().expect("cache lock poisoned");
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_fresh_value_and_expires() {
        let cache = TtlCache::new(Duration::from_millis(50));
        assert!(cache.get().is_none());
        cache.put(42);
        assert_eq!(*cache.get().expect("fresh"), 42);
        std::thread::sleep(Duration::from_millis(60));
        assert!(cache.get().is_none(), "value should expire after the TTL");
    }

    #[test]
    fn invalidate_clears_value() {
        let cache = TtlCache::new(Duration::from_secs(3600));
        cache.put("hello");
        cache.invalidate();
        assert!(cache.get().is_none());
    }
}
