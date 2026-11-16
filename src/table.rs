//! The table: array + map in one structure.
//!
//! # Layout
//!
//! - `array`: values for the integer keys `1..=array.len()` (key `i` lives at
//!   `array[i - 1]`). Slots may be `nil` (holes), but the last slot never is.
//! - `map`: every other key, in a compact insertion-ordered hash part.
//!
//! # Invariants
//!
//! 1. `array` is empty or its last element is non-nil.
//! 2. `map` holds no live integer key in `1..=array.len() + 1`.
//!
//! Together these make `array.len()` always a valid *border*
//! (`t[n]` is set and `t[n + 1]` is not), so [`Table::len`] is O(1) with no
//! binary search, and appending is a plain `Vec::push`.

use crate::error::{Error, Result};
use crate::hash::HashPart;
use crate::iter::{Cursor, IPairs, Iter, Pairs};
use crate::key::{AsKey, Key, KeyRef};
use crate::value::{write_number, Value};
use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Shared table handle. Cloning is a cheap reference, not a deep copy.
///
/// Tables compare and hash by identity.
///
/// # Reentrancy
///
/// Methods that take a closure ([`sort_by`](Self::sort_by),
/// [`for_each`](Self::for_each), [`with_slice`](Self::with_slice)) keep the
/// table borrowed while the closure runs; mutating the same table from
/// inside the closure panics. The iterators ([`pairs`](Self::pairs),
/// [`ipairs`](Self::ipairs)) hold no borrow between items, so the table may be
/// freely modified while iterating.
///
/// # Memory
///
/// Tables are reference counted. A table that (directly or indirectly)
/// contains itself is never freed unless the cycle is broken, for example
/// with [`clear`](Self::clear).
#[derive(Clone, Default)]
pub struct Table {
    pub(crate) inner: Rc<RefCell<TableInner>>,
}

#[derive(Clone, Default)]
pub(crate) struct TableInner {
    pub array: Vec<Value>,
    /// Number of `nil` slots inside `array`.
    pub holes: usize,
    pub map: HashPart,
}

impl TableInner {
    /// Borrow the live value under `key`, if any.
    #[inline(always)]
    pub fn get_ref(&self, key: KeyRef<'_>) -> Option<&Value> {
        match key {
            KeyRef::Integer(i) => {
                let idx = (i as u64).wrapping_sub(1);
                if idx < self.array.len() as u64 {
                    Some(&self.array[idx as usize]).filter(|v| !v.is_nil())
                } else if self.map.int_keys() != 0 {
                    self.map.get(key)
                } else {
                    None
                }
            }
            _ => self.map.get(key),
        }
    }

    #[inline(always)]
    pub fn get(&self, key: KeyRef<'_>) -> Value {
        match key {
            KeyRef::Integer(i) => self.get_int(i),
            _ => self.map.get(key).cloned().unwrap_or_default(),
        }
    }

    #[inline]
    pub fn get_int(&self, i: i64) -> Value {
        // One unsigned compare covers both `i >= 1` and `i <= len`.
        let idx = (i as u64).wrapping_sub(1);
        if idx < self.array.len() as u64 {
            self.array[idx as usize].clone()
        } else if self.map.int_keys() != 0 {
            self.map
                .get(KeyRef::Integer(i))
                .cloned()
                .unwrap_or_default()
        } else {
            Value::Nil
        }
    }

    #[inline(always)]
    fn contains(&self, key: KeyRef<'_>) -> bool {
        match key {
            KeyRef::Integer(i) => {
                let idx = (i as u64).wrapping_sub(1);
                if idx < self.array.len() as u64 {
                    !self.array[idx as usize].is_nil()
                } else {
                    self.map.int_keys() != 0 && self.map.get(key).is_some()
                }
            }
            _ => self.map.get(key).is_some(),
        }
    }

    /// `t[i] = value`. The two hot cases, overwriting a sequence slot and
    /// appending, are forced inline so the value is written straight from
    /// registers; everything else goes out of line.
    #[inline(always)]
    pub fn set_int(&mut self, i: i64, value: Value) {
        let len = self.array.len() as u64;
        // One unsigned compare covers both `i >= 1` and `i <= len`.
        let idx = (i as u64).wrapping_sub(1);
        if value.is_nil() || idx > len {
            self.set_int_slow(i, value);
        } else if idx < len {
            let slot = &mut self.array[idx as usize];
            if slot.is_nil() {
                self.holes -= 1;
            }
            slot.store(value);
        } else {
            // `t[len + 1] = v`
            self.array.push(value);
            self.migrate();
        }
    }

    /// Deletions, and integer keys outside `1..=len + 1`.
    #[inline(never)]
    fn set_int_slow(&mut self, i: i64, value: Value) {
        let len = self.array.len() as u64;
        let idx = (i as u64).wrapping_sub(1);
        if !value.is_nil() {
            debug_assert!(idx > len);
            // Infallible: integer keys are always valid.
            let _ = self.map.insert(i, value);
        } else if idx >= len {
            if self.map.int_keys() != 0 {
                self.map.remove(KeyRef::Integer(i));
            }
        } else if idx + 1 == len {
            self.array.pop();
            self.trim();
        } else {
            let slot = &mut self.array[idx as usize];
            if !slot.is_nil() {
                slot.store(Value::Nil);
                self.holes += 1;
            }
        }
    }

    /// Restore invariant 1 after the last slot was cleared or moved.
    /// A sequence without holes cannot end in `nil`, so the common case is
    /// a single compare.
    #[inline(always)]
    fn trim(&mut self) {
        if self.holes != 0 {
            self.trim_slow();
        }
    }

    #[cold]
    #[inline(never)]
    fn trim_slow(&mut self) {
        while let Some(Value::Nil) = self.array.last() {
            self.array.pop();
            self.holes -= 1;
        }
    }

    /// Restore invariant 2 after the array grew: pull the keys that now
    /// continue the sequence out of the hash part.
    #[inline]
    fn migrate(&mut self) {
        if self.map.int_keys() != 0 {
            self.migrate_slow();
        }
    }

    #[cold]
    fn migrate_slow(&mut self) {
        while self.map.int_keys() != 0 {
            let next = self.array.len() as i64 + 1;
            let v = self.map.remove(KeyRef::Integer(next));
            if v.is_nil() {
                break;
            }
            self.array.push(v);
        }
        self.map.purge_if_dead();
    }

    /// Advance a traversal cursor to the next live field.
    #[inline]
    pub fn step(&self, cur: &mut Cursor) -> Option<(Key, Value)> {
        if !cur.in_hash {
            while let Some(v) = self.array.get(cur.idx) {
                cur.idx += 1;
                if !v.is_nil() {
                    return Some((Key::Integer(cur.idx as i64), v.clone()));
                }
            }
            cur.in_hash = true;
            cur.idx = 0;
        }
        let entries = self.map.entries();
        while let Some(e) = entries.get(cur.idx) {
            cur.idx += 1;
            if !e.value.is_nil() {
                return Some((e.key.clone(), e.value.clone()));
            }
        }
        None
    }

    /// Cursor positioned just past `key`.
    fn cursor_after(&self, key: KeyRef<'_>) -> Result<Cursor> {
        if let KeyRef::Integer(i) = key {
            if i >= 1 && i as u64 <= self.array.len() as u64 {
                return Ok(Cursor {
                    in_hash: false,
                    idx: i as usize,
                });
            }
        }
        match self.map.position(key) {
            Some(p) => Ok(Cursor {
                in_hash: true,
                idx: p + 1,
            }),
            // A positive integer that is nowhere to be found was an array
            // slot that has since been cleared and trimmed away; everything
            // after it in the array is gone too, so resume at the hash part.
            None if matches!(key, KeyRef::Integer(i) if i >= 1) => Ok(Cursor {
                in_hash: true,
                idx: 0,
            }),
            None => Err(Error::InvalidKey),
        }
    }
}

type Memo = HashMap<*const (), Table>;

impl Table {
    /// An empty table. Does not allocate until the first insertion.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Table with room for `narray` sequence slots and `nhash` map slots.
    pub fn with_capacity(narray: usize, nhash: usize) -> Self {
        Self::from_inner(TableInner {
            array: Vec::with_capacity(narray),
            holes: 0,
            map: HashPart::with_capacity(nhash),
        })
    }

    #[inline]
    fn from_inner(inner: TableInner) -> Self {
        Self {
            inner: Rc::new(RefCell::new(inner)),
        }
    }

    /// Build a sequence table from values (`{a, b, c}`).
    pub fn from_seq<I, V>(values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        let array: Vec<Value> = values.into_iter().map(Into::into).collect();
        let holes = array.iter().filter(|v| v.is_nil()).count();
        let mut inner = TableInner {
            array,
            holes,
            map: HashPart::default(),
        };
        inner.trim();
        Self::from_inner(inner)
    }

    /// Build a table from key/value pairs (`{k1 = v1, k2 = v2}`).
    ///
    /// # Panics
    /// If a key is `nil` or NaN. Use [`set`](Self::set) to handle that.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsKey,
        V: Into<Value>,
    {
        let iter = pairs.into_iter();
        let t = Self::with_capacity(0, iter.size_hint().0);
        for (k, v) in iter {
            t.set(k, v).expect("invalid table key");
        }
        t
    }

    /// A sequence of all values plus a field `n` holding
    /// their count (so trailing nils are not lost).
    pub fn pack<I, V>(values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        let array: Vec<Value> = values.into_iter().map(Into::into).collect();
        let n = array.len() as i64;
        let t = Self::from_seq(array);
        let _ = t.inner.borrow_mut().map.insert("n", Value::Integer(n));
        t
    }

    /// Shallow copy: a new table with the same keys and values. Nested
    /// tables are shared, not copied.
    pub fn copy(&self) -> Self {
        Self::from_inner(self.inner.borrow().clone())
    }

    /// Deep copy. Every table reachable through keys or values is copied
    /// exactly once, so shared sub-tables stay shared in the copy and cyclic
    /// structures are reproduced rather than recursed into forever.
    pub fn deep_clone(&self) -> Self {
        self.deep_clone_memo(&mut Memo::new())
    }

    fn deep_clone_memo(&self, memo: &mut Memo) -> Self {
        if let Some(done) = memo.get(&self.as_ptr()) {
            return done.clone();
        }
        let out = Table::new();
        memo.insert(self.as_ptr(), out.clone());

        let src = self.inner.borrow();
        let deep = |v: &Value, memo: &mut Memo| match v {
            Value::Table(t) => Value::Table(t.deep_clone_memo(memo)),
            other => other.clone(),
        };
        let array: Vec<Value> = src.array.iter().map(|v| deep(v, memo)).collect();
        let mut map = HashPart::with_capacity(src.map.live());
        for e in src.map.entries().iter().filter(|e| !e.value.is_nil()) {
            let key = match &e.key {
                Key::Table(t) => Key::Table(t.deep_clone_memo(memo)),
                other => other.clone(),
            };
            let _ = map.insert(key, deep(&e.value, memo));
        }
        *out.inner.borrow_mut() = TableInner {
            array,
            holes: src.holes,
            map,
        };
        out
    }

    // ---------------------------------------------------------------------
    // Field access
    // ---------------------------------------------------------------------

    /// `t[key]`. Missing fields, and keys that can never be stored (`nil`,
    /// NaN), read as `nil`.
    #[inline]
    pub fn get<K: AsKey>(&self, key: K) -> Value {
        match key.key_ref() {
            Ok(k) => self.inner.borrow().get(k),
            Err(_) => Value::Nil,
        }
    }

    /// `t[key] = value`. A `nil` value deletes the field.
    ///
    /// Errors only when `key` is `nil` or NaN.
    #[inline]
    pub fn set<K: AsKey, V: Into<Value>>(&self, key: K, value: V) -> Result<()> {
        // Borrow before converting: nothing that can unwind then sits
        // between building the value and storing it, so it stays in
        // registers instead of being spilled and reloaded.
        let mut inner = self.inner.borrow_mut();
        let kr = key.key_ref()?;
        if let KeyRef::Integer(i) = kr {
            inner.set_int(i, value.into());
            return Ok(());
        }
        let value = value.into();
        if value.is_nil() {
            inner.map.remove(kr);
            Ok(())
        } else {
            inner.map.insert(key, value)
        }
    }

    /// Borrow the table for reading without cloning anything: see
    /// [`TableRef`]. Any attempt to modify the table while the returned
    /// guard is alive panics.
    #[inline]
    pub fn borrow(&self) -> TableRef<'_> {
        TableRef {
            inner: self.inner.borrow(),
        }
    }

    /// `t[key]` as an integer, without cloning the stored value. `None` if
    /// the field is missing or not an exact integer.
    #[inline]
    pub fn get_integer<K: AsKey>(&self, key: K) -> Option<i64> {
        let k = key.key_ref().ok()?;
        self.inner.borrow().get_ref(k)?.as_integer()
    }

    /// `t[key]` as a float (integers convert), without cloning.
    #[inline]
    pub fn get_number<K: AsKey>(&self, key: K) -> Option<f64> {
        let k = key.key_ref().ok()?;
        self.inner.borrow().get_ref(k)?.as_number()
    }

    /// `t[key]` as a boolean, without cloning. `None` if the field is
    /// missing or not a boolean.
    #[inline]
    pub fn get_bool<K: AsKey>(&self, key: K) -> Option<bool> {
        let k = key.key_ref().ok()?;
        self.inner.borrow().get_ref(k)?.as_bool()
    }

    /// Whether `t[key]` is set (not nil).
    #[inline]
    pub fn contains_key<K: AsKey>(&self, key: K) -> bool {
        match key.key_ref() {
            Ok(k) => self.inner.borrow().contains(k),
            Err(_) => false,
        }
    }

    /// Record / object field access (`t.name`).
    #[inline]
    pub fn get_field(&self, name: &str) -> Value {
        self.get(name)
    }

    /// `t.name = value`.
    #[inline]
    pub fn set_field(&self, name: &str, value: impl Into<Value>) -> Result<()> {
        self.set(name, value)
    }

    /// Treat as a set: `t[member] = true`.
    #[inline]
    pub fn set_add<K: AsKey>(&self, member: K) -> Result<()> {
        self.set(member, true)
    }

    /// Set membership: `t[member]` is truthy.
    #[inline]
    pub fn set_contains<K: AsKey>(&self, member: K) -> bool {
        match member.key_ref() {
            Ok(KeyRef::Integer(i)) => self.inner.borrow().get_int(i).is_truthy(),
            Ok(k) => self.inner.borrow().map.get(k).is_some_and(Value::is_truthy),
            Err(_) => false,
        }
    }

    /// Remove a set member; returns whether it was present.
    pub fn set_remove<K: AsKey>(&self, member: K) -> bool {
        let Ok(k) = member.key_ref() else {
            return false;
        };
        let mut inner = self.inner.borrow_mut();
        let present = inner.contains(k);
        if present {
            match k {
                KeyRef::Integer(i) => inner.set_int(i, Value::Nil),
                _ => drop(inner.map.remove(k)),
            }
        }
        present
    }

    // ---------------------------------------------------------------------
    // Size
    // ---------------------------------------------------------------------

    /// The length of the sequence, in O(1).
    ///
    /// For a proper sequence (`1..=n` all non-nil) this is `n`. With holes it
    /// is *a* border: `t[len]` is set and `t[len + 1]` is not.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.borrow().array.len()
    }

    /// Whether the table has no fields at all (`next(t) == nil`).
    ///
    /// Note that this is about every field, not just the sequence:
    /// `{x = 1}` has `len() == 0` but is not empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.borrow();
        inner.array.is_empty() && inner.map.live() == 0
    }

    /// Number of non-nil fields in both parts, in O(1).
    #[inline]
    pub fn field_count(&self) -> usize {
        let inner = self.inner.borrow();
        inner.array.len() - inner.holes + inner.map.live()
    }

    /// Remove every field, keeping allocated capacity.
    pub fn clear(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.array.clear();
        inner.holes = 0;
        inner.map.clear();
    }

    /// Reserve room for `narray` more sequence slots and `nhash` more map
    /// fields.
    pub fn reserve(&self, narray: usize, nhash: usize) {
        let mut inner = self.inner.borrow_mut();
        inner.array.reserve(narray);
        inner.map.reserve(nhash);
    }

    /// Release unused capacity and reclaim removed hash slots.
    ///
    /// Must not be called in the middle of a [`next`](Self::next) traversal.
    pub fn shrink_to_fit(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.array.shrink_to_fit();
        inner.map.shrink_to_fit();
    }

    // ---------------------------------------------------------------------
    // Sequence operations
    // ---------------------------------------------------------------------

    /// Append to the sequence (`t[len + 1] = value`). Amortized O(1).
    /// Appending `nil` does nothing.
    #[inline]
    pub fn push(&self, value: impl Into<Value>) -> Result<()> {
        let value = value.into();
        if !value.is_nil() {
            let mut inner = self.inner.borrow_mut();
            inner.array.push(value);
            inner.migrate();
        }
        Ok(())
    }

    /// Remove and return the last sequence element (`table.remove(t)`).
    /// Returns `nil` when the sequence is empty.
    #[inline]
    pub fn pop(&self) -> Value {
        let mut inner = self.inner.borrow_mut();
        let v = inner.array.pop().unwrap_or_default();
        inner.trim();
        v
    }

    /// Insert at `pos`, shifting `t[pos], ..., t[len]` up by one. `pos`
    /// must be in `1..=len + 1`.
    pub fn insert(&self, pos: i64, value: impl Into<Value>) -> Result<()> {
        let value = value.into();
        let mut inner = self.inner.borrow_mut();
        let len = inner.array.len();
        let idx = (pos as u64).wrapping_sub(1);
        if idx > len as u64 {
            return Err(Error::PositionOutOfBounds { position: pos, len });
        }
        if idx == len as u64 {
            if value.is_nil() {
                return Ok(());
            }
            inner.array.push(value);
        } else {
            if value.is_nil() {
                inner.holes += 1;
            }
            inner.array.insert(idx as usize, value);
        }
        inner.migrate();
        Ok(())
    }

    /// Remove and return `t[pos]`, shifting the
    /// elements after it down by one. `pos` must be in `1..=len + 1` (or `0`
    /// when the sequence is empty).
    pub fn remove(&self, pos: i64) -> Result<Value> {
        let mut inner = self.inner.borrow_mut();
        let len = inner.array.len();
        let idx = (pos as u64).wrapping_sub(1);
        if idx < len as u64 {
            let v = inner.array.remove(idx as usize);
            if v.is_nil() {
                inner.holes -= 1;
            }
            inner.trim();
            Ok(v)
        } else if idx == len as u64 {
            // `t[len + 1]` is nil by definition of a border.
            Ok(Value::Nil)
        } else if pos == 0 && len == 0 {
            Ok(inner.map.remove(KeyRef::Integer(0)))
        } else {
            Err(Error::PositionOutOfBounds { position: pos, len })
        }
    }

    /// Join the whole sequence with `sep`. Every element
    /// must be a string or a number.
    pub fn concat(&self, sep: &str) -> Result<String> {
        let inner = self.inner.borrow();
        concat_values(inner.array.iter(), inner.array.len(), sep)
    }

    /// Join `t[i], ..., t[j]` with `sep`.
    pub fn concat_range(&self, sep: &str, i: i64, j: i64) -> Result<String> {
        let inner = self.inner.borrow();
        if i > j {
            return Ok(String::new());
        }
        let len = inner.array.len() as i64;
        if i >= 1 && j <= len {
            let slice = &inner.array[(i - 1) as usize..j as usize];
            return concat_values(slice.iter(), slice.len(), sep);
        }
        let mut out = String::new();
        let mut k = i;
        loop {
            push_concat(&mut out, &inner.get_int(k))?;
            if k == j {
                return Ok(out);
            }
            out.push_str(sep);
            k += 1;
        }
    }

    /// The sequence `t[1], ..., t[len]` as a vector.
    pub fn unpack(&self) -> Vec<Value> {
        self.inner.borrow().array.clone()
    }

    /// `t[i], ..., t[j]` as a vector, with `nil` for
    /// missing fields.
    pub fn unpack_range(&self, i: i64, j: i64) -> Vec<Value> {
        let inner = self.inner.borrow();
        if i > j {
            return Vec::new();
        }
        let len = inner.array.len() as i64;
        if i >= 1 && j <= len {
            return inner.array[(i - 1) as usize..j as usize].to_vec();
        }
        let mut out = Vec::new();
        let mut k = i;
        loop {
            out.push(inner.get_int(k));
            if k == j {
                return out;
            }
            k += 1;
        }
    }

    /// Copy `t[f..=e]` to `t[pos..]` within
    /// this table. Overlapping ranges are handled correctly.
    pub fn move_within(&self, f: i64, e: i64, pos: i64) -> Result<()> {
        let Some(n) = check_move(f, e, pos, self.len())? else {
            return Ok(());
        };
        let mut inner = self.inner.borrow_mut();
        if pos > e || pos <= f {
            for k in 0..n {
                let v = inner.get_int(f + k);
                inner.set_int(pos + k, v);
            }
        } else {
            // Destination overlaps the tail of the source: copy backwards.
            for k in (0..n).rev() {
                let v = inner.get_int(f + k);
                inner.set_int(pos + k, v);
            }
        }
        Ok(())
    }

    /// Copy `t[f..=e]` into
    /// `dest[pos..]`.
    pub fn move_to(&self, f: i64, e: i64, pos: i64, dest: &Table) -> Result<()> {
        if self.ptr_eq(dest) {
            return self.move_within(f, e, pos);
        }
        let Some(n) = check_move(f, e, pos, self.len())? else {
            return Ok(());
        };
        let src = self.inner.borrow();
        let mut dst = dest.inner.borrow_mut();
        // Appending a hole-free slice is one bulk clone (`extend_from_slice`).
        if src.holes == 0
            && f >= 1
            && e as u64 <= src.array.len() as u64
            && pos as u64 == dst.array.len() as u64 + 1
        {
            dst.array
                .extend_from_slice(&src.array[(f - 1) as usize..e as usize]);
            dst.migrate();
            return Ok(());
        }
        for k in 0..n {
            dst.set_int(pos + k, src.get_int(f + k));
        }
        Ok(())
    }

    /// Sort the sequence ascending, in place.
    ///
    /// All elements must be numbers, or all strings; anything else is an
    /// [`Error::Incomparable`] and leaves the table untouched. Integers and
    /// floats compare exactly; NaNs sort last. Not stable.
    pub fn sort(&self) -> Result<()> {
        #[derive(Clone, Copy, PartialEq)]
        enum Kind {
            Int,
            Num,
            Str,
        }
        let mut inner = self.inner.borrow_mut();
        let a = &mut inner.array;
        let Some(first) = a.first() else {
            return Ok(());
        };
        let mut kind = match first {
            Value::Integer(_) => Kind::Int,
            Value::Number(_) => Kind::Num,
            Value::String(_) => Kind::Str,
            other => {
                return Err(Error::Incomparable {
                    left: other.type_name(),
                    right: a.get(1).unwrap_or(other).type_name(),
                })
            }
        };
        for v in &a[1..] {
            match (kind, v) {
                (Kind::Int, Value::Integer(_)) | (Kind::Str, Value::String(_)) => {}
                (Kind::Int | Kind::Num, Value::Integer(_) | Value::Number(_)) => kind = Kind::Num,
                _ => {
                    return Err(Error::Incomparable {
                        left: a[0].type_name(),
                        right: v.type_name(),
                    })
                }
            }
        }
        match kind {
            // Sort the raw integers: 8-byte moves and a branch-free compare
            // instead of shuffling tagged values.
            Kind::Int => {
                let mut ints: Vec<i64> = a
                    .iter()
                    .map(|v| match v {
                        Value::Integer(i) => *i,
                        _ => unreachable!(),
                    })
                    .collect();
                ints.sort_unstable();
                for (slot, i) in a.iter_mut().zip(ints) {
                    *slot = Value::Integer(i);
                }
            }
            // All floats: sort the raw `f64`s. `total_cmp` agrees with the
            // numeric order on ordinary numbers and puts NaN last. Mixed
            // integer/float sequences need the exact comparison instead.
            Kind::Num
                if a.iter().all(
                    |v| matches!(v, Value::Number(n) if !n.is_sign_negative() || !n.is_nan()),
                ) =>
            {
                let mut nums: Vec<f64> = a
                    .iter()
                    .filter_map(|v| match v {
                        Value::Number(n) => Some(*n),
                        _ => None,
                    })
                    .collect();
                nums.sort_unstable_by(f64::total_cmp);
                for (slot, n) in a.iter_mut().zip(nums) {
                    *slot = Value::Number(n);
                }
            }
            Kind::Num => a.sort_unstable_by(number_total_cmp),
            // Strings: sort (first 8 bytes, value) pairs. Most comparisons
            // are settled by the inline big-endian prefix, without following
            // the pointer to the string data.
            Kind::Str => {
                let mut keyed: Vec<(u64, Value)> = a
                    .iter_mut()
                    .map(|v| (str_prefix(v.as_str().unwrap_or_default()), v.take()))
                    .collect();
                keyed.sort_unstable_by(|(pa, va), (pb, vb)| {
                    pa.cmp(pb).then_with(|| match (va, vb) {
                        (Value::String(x), Value::String(y)) => x.as_bytes().cmp(y.as_bytes()),
                        _ => unreachable!(),
                    })
                });
                for (slot, (_, v)) in a.iter_mut().zip(keyed) {
                    *slot = v;
                }
            }
        }
        Ok(())
    }

    /// Sort the sequence with a comparator (stable).
    ///
    /// The table is borrowed while sorting: the comparator must not access
    /// this table. It must define a total order, as for `slice::sort_by`.
    pub fn sort_by<F>(&self, compare: F)
    where
        F: FnMut(&Value, &Value) -> Ordering,
    {
        let mut inner = self.inner.borrow_mut();
        inner.array.sort_by(compare);
        inner.trim();
    }

    /// Sort the sequence with a comparator (not stable, no allocation).
    ///
    /// Same restrictions as [`sort_by`](Self::sort_by).
    pub fn sort_unstable_by<F>(&self, compare: F)
    where
        F: FnMut(&Value, &Value) -> Ordering,
    {
        let mut inner = self.inner.borrow_mut();
        inner.array.sort_unstable_by(compare);
        inner.trim();
    }

    /// Reverse the sequence in place.
    pub fn reverse(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.array.reverse();
        inner.trim();
    }

    /// Run `f` on the sequence as a borrowed slice: zero-copy bulk access.
    /// Element `i` of the slice is `t[i + 1]`.
    ///
    /// The table is borrowed while `f` runs; `f` must not modify it.
    pub fn with_slice<R>(&self, f: impl FnOnce(&[Value]) -> R) -> R {
        f(&self.inner.borrow().array)
    }

    // ---------------------------------------------------------------------
    // Traversal
    // ---------------------------------------------------------------------

    /// Every field. Sequence keys come first in ascending
    /// order, then the remaining keys in insertion order.
    ///
    /// Holds no borrow between items. Existing fields may be
    /// assigned or cleared during the traversal; if new keys are added, the
    /// traversal stays safe but may skip or repeat fields.
    #[inline]
    pub fn pairs(&self) -> Pairs {
        Pairs::new(self.clone())
    }

    /// `(1, t[1]), (2, t[2]), ...` up to the first `nil`.
    #[inline]
    pub fn ipairs(&self) -> IPairs {
        IPairs::new(self.clone())
    }

    /// All keys, in [`pairs`](Self::pairs) order.
    pub fn keys(&self) -> impl Iterator<Item = Key> {
        self.pairs().map(|(k, _)| k)
    }

    /// All values, in [`pairs`](Self::pairs) order.
    pub fn values(&self) -> impl Iterator<Item = Value> {
        self.pairs().map(|(_, v)| v)
    }

    /// The field after `key` in traversal order, or the
    /// first field when `key` is `nil`. `Ok(None)` marks the end.
    ///
    /// O(1) per call. Errors with [`Error::InvalidKey`] if `key` is not a
    /// key of the table.
    pub fn next(&self, key: &Value) -> Result<Option<(Key, Value)>> {
        let inner = self.inner.borrow();
        let mut cur = match key {
            Value::Nil => Cursor::default(),
            k => inner.cursor_after(k.key_ref().map_err(|_| Error::InvalidKey)?)?,
        };
        Ok(inner.step(&mut cur))
    }

    /// Visit every field by reference, without cloning keys or values: the
    /// fastest full traversal. Same order as [`pairs`](Self::pairs).
    ///
    /// The table is borrowed while `f` runs; `f` must not modify it.
    pub fn for_each(&self, mut f: impl FnMut(&Key, &Value)) {
        let inner = self.inner.borrow();
        for (i, v) in inner.array.iter().enumerate() {
            if !v.is_nil() {
                f(&Key::Integer(i as i64 + 1), v);
            }
        }
        for e in inner.map.entries() {
            if !e.value.is_nil() {
                f(&e.key, &e.value);
            }
        }
    }

    // ---------------------------------------------------------------------
    // Identity and display
    // ---------------------------------------------------------------------

    /// Pointer identity: two handles to the same table.
    #[inline]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// An address unique to this table while it is alive.
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        Rc::as_ptr(&self.inner).cast()
    }

    /// Render the contents as a table literal, for debugging:
    /// `{1, 2, name = "x", [true] = 1}`. Tables already being printed
    /// (cycles) show as `<cycle>`.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        self.dump_into(&mut out, &mut Vec::new());
        out
    }

    fn dump_into(&self, out: &mut String, stack: &mut Vec<*const ()>) {
        if stack.contains(&self.as_ptr()) {
            out.push_str("<cycle>");
            return;
        }
        stack.push(self.as_ptr());
        let dump_value = |v: &Value, out: &mut String, stack: &mut Vec<*const ()>| match v {
            Value::Table(t) => t.dump_into(out, stack),
            other => {
                let _ = write!(out, "{other:?}");
            }
        };
        let inner = self.inner.borrow();
        out.push('{');
        let mut first = true;
        let mut sep = |out: &mut String| {
            if !std::mem::take(&mut first) {
                out.push_str(", ");
            }
        };
        for v in &inner.array {
            sep(out);
            dump_value(v, out, stack);
        }
        for e in inner.map.entries().iter().filter(|e| !e.value.is_nil()) {
            sep(out);
            match &e.key {
                Key::String(s) if is_identifier(s) => out.push_str(s),
                Key::Table(t) => {
                    out.push('[');
                    t.dump_into(out, stack);
                    out.push(']');
                }
                Key::String(s) => {
                    let _ = write!(out, "[{s:?}]");
                }
                other => {
                    let _ = write!(out, "[{other}]");
                }
            }
            out.push_str(" = ");
            dump_value(&e.value, out, stack);
        }
        out.push('}');
        stack.pop();
    }
}

/// A read-only view of a table, from [`Table::borrow`].
///
/// Everything it hands out is a plain reference into the table, so reads
/// cost no reference-count updates and no copies. This is the fastest way
/// to read many fields, or to read strings and nested tables.
///
/// ```
/// use flextable::{table, Value};
///
/// let t = table! { "name" => "flextable", "stars" => 5 };
/// let view = t.borrow();
/// assert_eq!(view.get("name").and_then(Value::as_str), Some("flextable"));
/// assert_eq!(view.iter().count(), 2);
/// ```
pub struct TableRef<'a> {
    inner: Ref<'a, TableInner>,
}

impl TableRef<'_> {
    /// Borrow `t[key]`; `None` when the field is missing.
    #[inline]
    pub fn get<K: AsKey>(&self, key: K) -> Option<&Value> {
        let k = key.key_ref().ok()?;
        self.inner.get_ref(k)
    }

    /// The sequence part: element `i` is `t[i + 1]`. May contain `nil`
    /// holes, but never ends in one.
    #[inline]
    pub fn as_slice(&self) -> &[Value] {
        &self.inner.array
    }

    /// The length of the sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.array.len()
    }

    /// Whether the table has no fields at all.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.array.is_empty() && self.inner.map.live() == 0
    }

    /// Number of non-nil fields.
    #[inline]
    pub fn field_count(&self) -> usize {
        self.inner.array.len() - self.inner.holes + self.inner.map.live()
    }

    /// Every field by reference, in [`pairs`](Table::pairs) order.
    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(&self.inner.array, self.inner.map.entries())
    }
}

impl<'b> IntoIterator for &'b TableRef<'_> {
    type Item = (KeyRef<'b>, &'b Value);
    type IntoIter = Iter<'b>;

    fn into_iter(self) -> Iter<'b> {
        self.iter()
    }
}

fn is_identifier(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Validate `table.move` arguments. `Ok(None)` means nothing to move,
/// otherwise the element count.
fn check_move(f: i64, e: i64, pos: i64, len: usize) -> Result<Option<i64>> {
    if e < f {
        return Ok(None);
    }
    // "too many elements to move"
    let n = e
        .checked_sub(f)
        .and_then(|d| d.checked_add(1))
        .ok_or(Error::PositionOutOfBounds { position: f, len })?;
    // "destination wrap around"
    pos.checked_add(n - 1)
        .ok_or(Error::PositionOutOfBounds { position: pos, len })?;
    Ok(Some(n))
}

#[inline]
fn push_concat(out: &mut String, v: &Value) -> Result<()> {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Integer(i) => {
            let _ = write!(out, "{i}");
        }
        Value::Number(n) => {
            let _ = write_number(out, *n);
        }
        other => {
            return Err(Error::TypeMismatch {
                expected: "string or number",
                found: other.type_name(),
            })
        }
    }
    Ok(())
}

fn concat_values<'a>(
    values: impl Iterator<Item = &'a Value> + Clone,
    count: usize,
    sep: &str,
) -> Result<String> {
    // Size the buffer once: exact for strings, a small guess for numbers.
    let payload: usize = values.clone().map(|v| v.as_str().map_or(8, str::len)).sum();
    let mut out = String::with_capacity(payload + sep.len() * count.saturating_sub(1));
    for (i, v) in values.enumerate() {
        if i != 0 {
            out.push_str(sep);
        }
        push_concat(&mut out, v)?;
    }
    Ok(out)
}

/// The first 8 bytes of `s` as a big-endian integer, zero padded. Ordering
/// by this prefix agrees with bytewise string order whenever two prefixes
/// differ.
#[inline]
fn str_prefix(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut buf = [0u8; 8];
    let n = bytes.len().min(8);
    buf[..n].copy_from_slice(&bytes[..n]);
    u64::from_be_bytes(buf)
}

/// Total order over numeric values, with NaN after everything.
fn number_total_cmp(a: &Value, b: &Value) -> Ordering {
    a.lua_cmp(b).unwrap_or_else(|| {
        let nan = |v: &Value| matches!(v, Value::Number(n) if n.is_nan());
        nan(a).cmp(&nan(b))
    })
}

impl PartialEq for Table {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl Eq for Table {}

impl Hash for Table {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.as_ptr() as usize);
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "table: {:p}", self.as_ptr())
    }
}

impl FromIterator<Value> for Table {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        Self::from_seq(iter)
    }
}

impl FromIterator<(Key, Value)> for Table {
    fn from_iter<I: IntoIterator<Item = (Key, Value)>>(iter: I) -> Self {
        Self::from_pairs(iter)
    }
}

impl Extend<Value> for Table {
    fn extend<I: IntoIterator<Item = Value>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        self.reserve(iter.size_hint().0, 0);
        for v in iter {
            let _ = self.push(v);
        }
    }
}

impl Extend<(Key, Value)> for Table {
    fn extend<I: IntoIterator<Item = (Key, Value)>>(&mut self, iter: I) {
        for (k, v) in iter {
            let _ = self.set(k, v);
        }
    }
}

impl IntoIterator for &Table {
    type Item = (Key, Value);
    type IntoIter = Pairs;

    fn into_iter(self) -> Pairs {
        self.pairs()
    }
}
