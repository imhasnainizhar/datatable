//! Traversal iterators.
//!
//! Both iterators own a handle to the table and re-borrow it for each item,
//! so the table can be read or modified between items.

use crate::hash::Entry;
use crate::key::{Key, KeyRef};
use crate::table::Table;
use crate::value::Value;
use std::iter::FusedIterator;

/// A position in a traversal: an index into the array part, then an index
/// into the hash part's entry vector.
#[derive(Clone, Copy, Default)]
pub(crate) struct Cursor {
    pub in_hash: bool,
    pub idx: usize,
}

/// Iterator over every field of a table. See [`Table::pairs`].
#[derive(Clone)]
pub struct Pairs {
    table: Table,
    cursor: Cursor,
    done: bool,
}

impl Pairs {
    pub(crate) fn new(table: Table) -> Self {
        Self {
            table,
            cursor: Cursor::default(),
            done: false,
        }
    }
}

impl Iterator for Pairs {
    type Item = (Key, Value);

    #[inline]
    fn next(&mut self) -> Option<(Key, Value)> {
        if self.done {
            return None;
        }
        let item = self.table.inner.borrow().step(&mut self.cursor);
        self.done = item.is_none();
        item
    }
}

impl FusedIterator for Pairs {}

/// Iterator over the sequence part of a table. See [`Table::ipairs`].
#[derive(Clone)]
pub struct IPairs {
    table: Table,
    /// Next 1-based index; `0` once exhausted.
    next: i64,
}

impl IPairs {
    pub(crate) fn new(table: Table) -> Self {
        Self { table, next: 1 }
    }
}

impl Iterator for IPairs {
    type Item = (i64, Value);

    #[inline]
    fn next(&mut self) -> Option<(i64, Value)> {
        if self.next == 0 {
            return None;
        }
        let i = self.next;
        let v = self.table.inner.borrow().get_int(i);
        if v.is_nil() {
            self.next = 0;
            None
        } else {
            self.next = i + 1;
            Some((i, v))
        }
    }
}

impl FusedIterator for IPairs {}

/// Borrowing iterator over every field. See
/// [`TableRef::iter`](crate::table::TableRef::iter).
#[derive(Clone)]
pub struct Iter<'a> {
    array: std::iter::Enumerate<std::slice::Iter<'a, Value>>,
    entries: std::slice::Iter<'a, Entry>,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(array: &'a [Value], entries: &'a [Entry]) -> Self {
        Self {
            array: array.iter().enumerate(),
            entries: entries.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (KeyRef<'a>, &'a Value);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        for (i, v) in self.array.by_ref() {
            if !v.is_nil() {
                return Some((KeyRef::Integer(i as i64 + 1), v));
            }
        }
        for e in self.entries.by_ref() {
            if !e.value.is_nil() {
                return Some((e.key.as_key_ref(), &e.value));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.array.len() + self.entries.len()))
    }
}

impl FusedIterator for Iter<'_> {}
