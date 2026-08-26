use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    map: HashMap<K, (V, u64)>,
    /// Eviction order queue — front is the oldest (least recently used) key.
    order: VecDeque<K>,
    max_entries: usize,
    clock: u64,
}

impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
    /// Create a new LRU cache. Eviction is O(1) via front-pop, but access
    /// reordering in `get()` is O(N) due to linear scan of the VecDeque.
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_entries),
            order: VecDeque::with_capacity(max_entries),
            max_entries,
            clock: 0,
        }
    }

    /// Get a value from the cache, updating its LRU position.
    /// Note: reordering is O(N) due to VecDeque linear scan and removal.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock += 1;
        if let Some((_, timestamp)) = self.map.get_mut(key) {
            *timestamp = self.clock;
            // Move key to back (most recently used)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
        }
        self.map.get(key).map(|(v, _)| v)
    }

    /// Insert a value into the cache, evicting the LRU item in O(1) if full.
    pub fn insert(&mut self, key: K, value: V) {
        if self.max_entries == 0 {
            return;
        }
        self.clock += 1;

        // If the key already exists, update it and move to back of order.
        if self.map.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.max_entries {
            // Evict the front (oldest / least recently used) — O(1).
            while let Some(lru_key) = self.order.pop_front() {
                // Guard: the key might have been removed or re-inserted;
                // only evict if it's still in the map and genuinely stale.
                if self.map.contains_key(&lru_key) {
                    self.map.remove(&lru_key);
                    break;
                }
            }
        }

        self.order.push_back(key.clone());
        self.map.insert(key, (value, self.clock));
    }

    /// Remove a specific key from the cache.
    pub fn remove(&mut self, key: &K) {
        self.map.remove(key);
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
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
        self.order.clear();
        self.clock = 0;
    }
}
