use crate::RafxResult;
use fnv::FnvHashMap;
use std::collections::hash_map::Entry;

pub struct LruCacheEntry<T: Clone> {
    value: T,
    last_usage: u64,
}

//NOTE: This has O(n) cost to evict oldest entry. But this is intended to be used for things that
// are expensive to create and not a huge number of entries. It could be improved by adding a
// priority queue, but then there is overhead per access instead of per create.
pub struct LruCache<T: Clone> {
    entries: FnvHashMap<u64, LruCacheEntry<T>>,
    next_usage_index: u64,
    max_count: usize,
}

impl<T: Clone> LruCache<T> {
    pub fn new(max_count: usize) -> Self {
        LruCache {
            entries: Default::default(),
            next_usage_index: 0,
            max_count,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_usage_index = 0;
    }

    pub fn get_or_create<CreateFn: FnOnce() -> RafxResult<T>>(
        &mut self,
        hash: u64,
        create_fn: CreateFn,
    ) -> RafxResult<T> {
        //
        // Get or create it, then bump the last_usage and return the renderpass
        //
        let value = match self.entries.entry(hash) {
            Entry::Occupied(mut x) => {
                let mut entry = x.get_mut();
                entry.last_usage = self.next_usage_index;
                entry.value.clone()
            }
            Entry::Vacant(x) => {
                let entry = LruCacheEntry {
                    value: (create_fn)()?,
                    last_usage: self.next_usage_index,
                };
                x.insert(entry).value.clone()
            }
        };

        // Increment so that next call uses the next higher index
        self.next_usage_index += 1;

        //
        // If the cache is full, evict the least-recently used renderpass
        //
        if self.entries.len() > self.max_count {
            let mut min_usage = u64::MAX;
            let mut min_usage_hash = 0;
            for (&hash, entry) in &self.entries {
                if entry.last_usage < min_usage {
                    min_usage = entry.last_usage;
                    min_usage_hash = hash;
                }
            }

            self.entries.remove(&min_usage_hash);
        }

        Ok(value)
    }
}
