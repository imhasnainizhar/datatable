//! Hashable keys for tables.
//!
//! Lua forbids `nil` and NaN keys. Numeric keys are canonical: a float with
//! an exact integer value (`1.0`) is the same key as the integer (`1`).
//!
//! Three pieces live here:
//!
//! - [`Key`]: an owned, always-valid key (what a table stores).
//! - [`KeyRef`]: a borrowed key used for lookups, so `t.get("name")` hashes
//!   the `&str` directly and never allocates.
//! - [`AsKey`]: the conversion trait accepted by every keyed `Table` method.

use crate::error::{Error, Result};
use crate::table::Table;
use crate::value::{f64_to_i64_exact, write_number, Value};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A value that may index a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Boolean(bool),
    /// Canonical integer key (stored in the array part when in `1..=n`).
    Integer(i64),
    /// Bits of a float that is neither NaN nor an exact integer.
    Number(u64),
    String(Rc<str>),
    /// Another table, compared and hashed by identity. The key keeps that
    /// table alive.
    Table(Table),
}

/// A borrowed, validated key. `Copy`, allocation-free, and hashes exactly
/// like the [`Key`] it corresponds to.
#[derive(Debug, Clone, Copy)]
pub enum KeyRef<'a> {
    Boolean(bool),
    Integer(i64),
    Number(u64),
    String(&'a str),
    Table(&'a Table),
}

impl Key {
    /// Convert a [`Value`] into a key, rejecting `nil` and NaN.
    pub fn try_from_value(value: &Value) -> Result<Self> {
        value.key_ref().map(KeyRef::to_key)
    }

    /// Canonical key for a float: integer when exact, error when NaN.
    #[inline]
    pub fn from_f64(n: f64) -> Result<Self> {
        number_key(n).map(KeyRef::to_key)
    }

    #[inline]
    pub fn as_key_ref(&self) -> KeyRef<'_> {
        match self {
            Self::Boolean(b) => KeyRef::Boolean(*b),
            Self::Integer(i) => KeyRef::Integer(*i),
            Self::Number(bits) => KeyRef::Number(*bits),
            Self::String(s) => KeyRef::String(s),
            Self::Table(t) => KeyRef::Table(t),
        }
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    #[inline]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// The key as a value (cheap: clones a reference at most).
    #[inline]
    pub fn to_value(&self) -> Value {
        self.clone().into()
    }
}

impl<'a> KeyRef<'a> {
    /// Allocate the owned key. Only string keys allocate.
    #[inline]
    pub fn to_key(self) -> Key {
        match self {
            Self::Boolean(b) => Key::Boolean(b),
            Self::Integer(i) => Key::Integer(i),
            Self::Number(bits) => Key::Number(bits),
            Self::String(s) => Key::String(Rc::from(s)),
            Self::Table(t) => Key::Table(t.clone()),
        }
    }

    /// Whether this borrowed key denotes the same slot as `key`.
    #[inline]
    pub(crate) fn matches(self, key: &Key) -> bool {
        match (self, key) {
            // Same allocation (a key read back from this table) needs no
            // byte comparison at all.
            (Self::String(a), Key::String(b)) => {
                a.len() == b.len() && (std::ptr::eq(a.as_ptr(), b.as_ptr()) || a == &**b)
            }
            (Self::Integer(a), Key::Integer(b)) => a == *b,
            (Self::Table(a), Key::Table(b)) => a.ptr_eq(b),
            (Self::Boolean(a), Key::Boolean(b)) => a == *b,
            (Self::Number(a), Key::Number(b)) => a == *b,
            _ => false,
        }
    }
}

/// One feed per key keeps hashing as cheap as the hasher allows. Different
/// key types may collide with each other; equality sorts that out.
impl Hash for KeyRef<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            Self::String(s) => state.write(s.as_bytes()),
            Self::Integer(i) => state.write_u64(i as u64),
            Self::Number(bits) => state.write_u64(bits),
            Self::Table(t) => state.write_usize(t.as_ptr() as usize),
            Self::Boolean(b) => state.write_u8(b as u8),
        }
    }
}

impl Hash for Key {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_key_ref().hash(state);
    }
}

#[inline]
fn number_key<'a>(n: f64) -> Result<KeyRef<'a>> {
    if let Some(i) = f64_to_i64_exact(n) {
        Ok(KeyRef::Integer(i))
    } else if n.is_nan() {
        Err(Error::InvalidKey)
    } else {
        Ok(KeyRef::Number(n.to_bits()))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Number(bits) => write_number(f, f64::from_bits(*bits)),
            Self::String(s) => f.write_str(s),
            Self::Table(t) => write!(f, "table: {:p}", t.as_ptr()),
        }
    }
}

/// Anything usable as a table key: integers, floats, booleans, strings,
/// tables, and dynamic [`Value`]s / [`Key`]s (owned or borrowed).
///
/// Lookups go through [`key_ref`](Self::key_ref) and never allocate; an owned
/// [`Key`] is only built (via [`into_key`](Self::into_key)) when a brand-new
/// field is inserted.
pub trait AsKey {
    /// Borrowed, canonical form. Errors for `nil` and NaN.
    fn key_ref(&self) -> Result<KeyRef<'_>>;

    /// Owned form, reusing existing allocations where possible.
    fn into_key(self) -> Result<Key>
    where
        Self: Sized,
    {
        self.key_ref().map(KeyRef::to_key)
    }
}

macro_rules! int_as_key {
    ($($t:ty),*) => {$(
        impl AsKey for $t {
            #[inline]
            fn key_ref(&self) -> Result<KeyRef<'_>> {
                Ok(KeyRef::Integer(i64::from(*self)))
            }
        }
        impl From<$t> for Key {
            #[inline]
            fn from(i: $t) -> Self {
                Self::Integer(i64::from(i))
            }
        }
    )*};
}
int_as_key!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! wide_int_as_key {
    ($($t:ty),*) => {$(
        impl AsKey for $t {
            #[inline]
            fn key_ref(&self) -> Result<KeyRef<'_>> {
                match i64::try_from(*self) {
                    Ok(i) => Ok(KeyRef::Integer(i)),
                    // Beyond i64: the nearest float, as Lua would store it.
                    Err(_) => number_key(*self as f64),
                }
            }
        }
    )*};
}
wide_int_as_key!(isize, usize, u64);

impl AsKey for f64 {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        number_key(*self)
    }
}

impl AsKey for bool {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::Boolean(*self))
    }
}

impl AsKey for &str {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::String(self))
    }
}

impl AsKey for String {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::String(self))
    }
}

impl AsKey for &String {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::String(self))
    }
}

impl AsKey for Rc<str> {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::String(self))
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        Ok(Key::String(self))
    }
}

impl AsKey for Table {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::Table(self))
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        Ok(Key::Table(self))
    }
}

impl AsKey for &Table {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(KeyRef::Table(self))
    }
}

impl AsKey for Key {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(self.as_key_ref())
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        Ok(self)
    }
}

impl AsKey for &Key {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(self.as_key_ref())
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        Ok(self.clone())
    }
}

impl AsKey for KeyRef<'_> {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        Ok(*self)
    }
}

impl AsKey for Value {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        match self {
            Value::Nil => Err(Error::NilKey),
            Value::Boolean(b) => Ok(KeyRef::Boolean(*b)),
            Value::Integer(i) => Ok(KeyRef::Integer(*i)),
            Value::Number(n) => number_key(*n),
            Value::String(s) => Ok(KeyRef::String(s)),
            Value::Table(t) => Ok(KeyRef::Table(t)),
        }
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        match self {
            // Reuse the reference-counted allocations.
            Value::String(s) => Ok(Key::String(s)),
            Value::Table(t) => Ok(Key::Table(t)),
            other => other.key_ref().map(KeyRef::to_key),
        }
    }
}

impl AsKey for &Value {
    #[inline]
    fn key_ref(&self) -> Result<KeyRef<'_>> {
        (**self).key_ref()
    }
    #[inline]
    fn into_key(self) -> Result<Key> {
        match self {
            Value::String(s) => Ok(Key::String(Rc::clone(s))),
            other => other.key_ref().map(KeyRef::to_key),
        }
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Self::String(Rc::from(s))
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Self::String(Rc::from(s))
    }
}

impl From<Rc<str>> for Key {
    fn from(s: Rc<str>) -> Self {
        Self::String(s)
    }
}

impl From<bool> for Key {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<Table> for Key {
    fn from(t: Table) -> Self {
        Self::Table(t)
    }
}

impl TryFrom<Value> for Key {
    type Error = Error;

    fn try_from(value: Value) -> Result<Self> {
        value.into_key()
    }
}

impl TryFrom<&Value> for Key {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value.into_key()
    }
}
