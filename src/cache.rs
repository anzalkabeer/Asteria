use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    map: HashMap<K, (V, u64)>,
    max_entries: usize,
    clock: u64,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    /// Create a new timestamp-based LRU cache.
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_entries),
            max_entries,
            clock: 0,
        }
    }

    /// Get a value from the cache, updating its LRU timestamp.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock += 1;
        if let Some((_, timestamp)) = self.map.get_mut(key) {
            *timestamp = self.clock;
        }
        self.map.get(key).map(|(v, _)| v)
    }

    /// Insert a value into the cache, evicting the least recently used item if full.
    #[allow(clippy::collapsible_if)]
    pub fn insert(&mut self, key: K, value: V) {
        self.clock += 1;

        if self.map.len() >= self.max_entries && !self.map.contains_key(&key) {
            if let Some(lru_key) = self
                .map
                .iter()
                .min_by_key(|(_, (_, ts))| *ts)
                .map(|(k, _)| k.clone())
            {
                self.map.remove(&lru_key);
            }
        }

        self.map.insert(key, (value, self.clock));
    }

    /// Remove a specific key from the cache.
    pub fn remove(&mut self, key: &K) {
        self.map.remove(key);
    }

    /// Current number of items in the cache.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Is the cache empty?
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Clear all items from the cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.clock = 0;
    }
}
