//! The hash part of a table: a compact, insertion-ordered open-addressing map.
//!
//! Layout (the same idea as CPython's dict and `indexmap`):
//!
//! - `entries`: a dense `Vec` of `(hash, key, value)` in insertion order.
//! - `index`: a SwissTable of `u32` positions into `entries`.
//!
//! Why this shape instead of a plain `HashMap<Key, Value>`:
//!
//! - Iteration is a linear scan over a dense vector, and a position in that
//!   vector is a stable cursor, which is what makes `next` O(1).
//! - Removal leaves a tombstone (value = nil) instead of moving entries, so
//!   clearing fields during a traversal is safe.
//!   Tombstones are reclaimed only when a new key is inserted.
//! - The index holds 4-byte slots, so probing touches very little memory, and
//!   every entry caches its hash, so growth never re-hashes a string.

use crate::error::Result;
use crate::key::{AsKey, Key, KeyRef};
use crate::value::Value;
use hashbrown::{DefaultHashBuilder, HashTable};
use std::hash::BuildHasher;

#[derive(Clone)]
pub(crate) struct Entry {
    pub hash: u64,
    pub key: Key,
    /// `Nil` marks a removed (dead) entry.
    pub value: Value,
}

#[derive(Clone, Default)]
pub(crate) struct HashPart {
    index: HashTable<u32>,
    entries: Vec<Entry>,
    /// Entries whose value is not nil.
    live: usize,
    /// Live entries with an integer key; lets integer misses in the array
    /// part skip hashing entirely when the map holds no integers.
    int_keys: usize,
    hasher: DefaultHashBuilder,
}

/// Tombstones are tolerated until there are this many *and* they outnumber
/// live entries; compaction is then amortized O(1) per removal.
const MIN_DEAD_TO_COMPACT: usize = 8;

impl HashPart {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            index: HashTable::with_capacity(n),
            entries: Vec::with_capacity(n),
            ..Self::default()
        }
    }

    #[inline]
    pub fn live(&self) -> usize {
        self.live
    }

    #[inline]
    pub fn int_keys(&self) -> usize {
        self.int_keys
    }

    /// All entry slots, dead ones included, in insertion order.
    #[inline]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[inline(always)]
    fn hash_of(&self, key: KeyRef<'_>) -> u64 {
        self.hasher.hash_one(key)
    }

    // Lookups come in one out-of-line function per key shape. Each takes its
    // key in registers and returns `Ok(position)` or `Err(hash)` in
    // registers, and inside each the key type is known statically, so the
    // probe loop compares without re-dispatching on the key variant.
    // Positions may refer to dead entries.

    #[inline(never)]
    fn lookup_str(&self, s: &str) -> std::result::Result<usize, u64> {
        let hash = self.hash_of(KeyRef::String(s));
        let entries = &self.entries;
        self.index
            .find(hash, |&i| {
                let e = &entries[i as usize];
                e.hash == hash
                    && matches!(&e.key, Key::String(k)
                        if k.len() == s.len()
                            && (std::ptr::eq(k.as_ptr(), s.as_ptr()) || **k == *s))
            })
            .map(|&i| i as usize)
            .ok_or(hash)
    }

    #[inline(never)]
    fn lookup_int(&self, n: i64) -> std::result::Result<usize, u64> {
        let hash = self.hash_of(KeyRef::Integer(n));
        let entries = &self.entries;
        self.index
            .find(
                hash,
                |&i| matches!(entries[i as usize].key, Key::Integer(k) if k == n),
            )
            .map(|&i| i as usize)
            .ok_or(hash)
    }

    #[inline(never)]
    fn lookup_other(&self, key: KeyRef<'_>) -> std::result::Result<usize, u64> {
        let hash = self.hash_of(key);
        let entries = &self.entries;
        self.index
            .find(hash, |&i| {
                let e = &entries[i as usize];
                e.hash == hash && key.matches(&e.key)
            })
            .map(|&i| i as usize)
            .ok_or(hash)
    }

    #[inline(always)]
    fn lookup(&self, key: KeyRef<'_>) -> std::result::Result<usize, u64> {
        match key {
            KeyRef::String(s) => self.lookup_str(s),
            KeyRef::Integer(n) => self.lookup_int(n),
            other => self.lookup_other(other),
        }
    }

    /// Position of `key` (live or dead), for resuming a traversal.
    #[inline]
    pub fn position(&self, key: KeyRef<'_>) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        self.lookup(key).ok()
    }

    /// The live value stored under `key`.
    #[inline(always)]
    pub fn get(&self, key: KeyRef<'_>) -> Option<&Value> {
        if self.live == 0 {
            return None;
        }
        let i = self.lookup(key).ok()?;
        let v = &self.entries[i].value;
        if v.is_nil() {
            None
        } else {
            Some(v)
        }
    }

    /// Store a non-nil `value`. The owned key is only built when the field is
    /// new; overwriting never allocates.
    #[inline(always)]
    pub fn insert<K: AsKey>(&mut self, key: K, value: Value) -> Result<()> {
        debug_assert!(!value.is_nil());
        match self.lookup(key.key_ref()?) {
            Ok(i) => {
                let e = &mut self.entries[i];
                if e.value.is_nil() {
                    // Revive a tombstone in place.
                    self.live += 1;
                    if matches!(e.key, Key::Integer(_)) {
                        self.int_keys += 1;
                    }
                }
                e.value.store(value);
            }
            Err(hash) => self.push_new(hash, key.into_key()?, value),
        }
        Ok(())
    }

    #[inline(never)]
    fn push_new(&mut self, hash: u64, key: Key, value: Value) {
        let dead = self.entries.len() - self.live;
        if dead != 0 && (self.live == 0 || (dead >= MIN_DEAD_TO_COMPACT && dead >= self.live)) {
            self.compact();
        }
        let i = u32::try_from(self.entries.len()).expect("table hash part exceeds u32::MAX fields");
        if matches!(key, Key::Integer(_)) {
            self.int_keys += 1;
        }
        self.live += 1;
        self.entries.push(Entry { hash, key, value });
        let entries = &self.entries;
        self.index
            .insert_unique(hash, i, |&j| entries[j as usize].hash);
    }

    /// Remove `key`, returning its value (`Nil` if absent).
    #[inline(always)]
    pub fn remove(&mut self, key: KeyRef<'_>) -> Value {
        if self.live == 0 {
            return Value::Nil;
        }
        let Ok(i) = self.lookup(key) else {
            return Value::Nil;
        };
        let e = &mut self.entries[i];
        let old = e.value.take();
        if !old.is_nil() {
            self.live -= 1;
            if matches!(e.key, Key::Integer(_)) {
                self.int_keys -= 1;
            }
        }
        old
    }

    /// Drop every tombstone and rebuild the index from the cached hashes.
    #[cold]
    fn compact(&mut self) {
        self.index.clear();
        if self.live == 0 {
            self.entries.clear();
            return;
        }
        self.entries.retain(|e| !e.value.is_nil());
        let entries = &self.entries;
        for (i, e) in entries.iter().enumerate() {
            self.index
                .insert_unique(e.hash, i as u32, |&j| entries[j as usize].hash);
        }
    }

    /// Free all slots at once if nothing live remains. Only called from
    /// operations that insert, where reorganizing is permitted.
    #[inline]
    pub fn purge_if_dead(&mut self) {
        if self.live == 0 && !self.entries.is_empty() {
            self.compact();
        }
    }

    pub fn clear(&mut self) {
        self.index.clear();
        self.entries.clear();
        self.live = 0;
        self.int_keys = 0;
    }

    pub fn reserve(&mut self, additional: usize) {
        let entries = &self.entries;
        self.index
            .reserve(additional, |&j| entries[j as usize].hash);
        self.entries.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.compact();
        let entries = &self.entries;
        self.index.shrink_to_fit(|&j| entries[j as usize].hash);
        self.entries.shrink_to_fit();
    }
}
