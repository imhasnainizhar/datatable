//! Dynamic values stored in tables.

use crate::error::{Error, Result};
use crate::key::Key;
use crate::table::Table;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

/// A dynamic value.
///
/// Every variant is cheap to clone: strings are reference-counted
/// (`Rc<str>`) and tables are shared handles, so reading a value out of a
/// table never copies its contents.
#[derive(Clone, Default)]
pub enum Value {
    #[default]
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(Rc<str>),
    Table(Table),
}

/// `2^63` as a float; the first float above the `i64` range.
const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;

/// Convert a float to an integer only when the conversion is exact.
#[inline]
pub(crate) fn f64_to_i64_exact(n: f64) -> Option<i64> {
    // NaN fails both range checks.
    if (-TWO_POW_63..TWO_POW_63).contains(&n) {
        let i = n as i64;
        if i as f64 == n {
            return Some(i);
        }
    }
    None
}

/// Exact mathematical comparison of an integer with a float (no rounding of
/// the integer to 53 bits). `None` only when `f` is NaN.
#[inline]
fn cmp_int_float(i: i64, f: f64) -> Option<Ordering> {
    if f.is_nan() {
        None
    } else if f >= TWO_POW_63 {
        Some(Ordering::Less)
    } else if f < -TWO_POW_63 {
        Some(Ordering::Greater)
    } else {
        let floor = f.floor();
        // `floor` is integral and in range, so the cast is exact.
        match i.cmp(&(floor as i64)) {
            Ordering::Equal if f > floor => Some(Ordering::Less),
            ord => Some(ord),
        }
    }
}

/// Drop whatever `slot` holds, leaving `nil`.
///
/// Deliberately `extern "C"`: such functions abort instead of unwinding, so
/// the compiler knows this call cannot unwind. Dropping a value never panics
/// (it only releases reference counts), and without an unwind edge the
/// callers' hot paths keep their state in registers instead of spilling it
/// around the call.
#[cold]
#[inline(never)]
#[allow(improper_ctypes_definitions)]
extern "C" fn release(slot: &mut Value) {
    *slot = Value::Nil;
}

impl Value {
    /// Overwrite `self` with `value`. Same effect as `*self = value`, but
    /// the common case (the old value owns no heap data) is two plain
    /// stores with no drop glue.
    #[inline(always)]
    pub(crate) fn store(&mut self, value: Value) {
        if matches!(self, Self::String(_) | Self::Table(_)) {
            release(self);
        }
        // The old value owns nothing at this point, so forgetting it leaks
        // nothing.
        std::mem::forget(std::mem::replace(self, value));
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Table(_) => "table",
        }
    }

    /// Truthiness: only `nil` and `false` are false.
    #[inline]
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }

    /// Borrow the table if this value is one.
    #[inline]
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Self::Table(t) => Some(t),
            _ => None,
        }
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The value as an integer: integers as-is, floats only when they hold an
    /// exact integer (`3.0` yes, `3.5` no). Strings are not coerced.
    #[inline]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Number(n) => f64_to_i64_exact(*n),
            _ => None,
        }
    }

    /// The value as a float, converting integers.
    #[inline]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Integer(i) => Some(*i as f64),
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Integer(_) | Self::Number(_))
    }

    /// Take the value, leaving `nil` behind.
    #[inline]
    pub fn take(&mut self) -> Value {
        std::mem::take(self)
    }

    /// Ordering for numbers and strings: numbers compare with numbers (integers and floats
    /// mix exactly), strings with strings (bytewise). `None` for any other
    /// pairing, and for comparisons involving NaN.
    pub fn lua_cmp(&self, other: &Value) -> Option<Ordering> {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => Some(a.cmp(b)),
            (Self::Number(a), Self::Number(b)) => a.partial_cmp(b),
            (Self::Integer(a), Self::Number(b)) => cmp_int_float(*a, *b),
            (Self::Number(a), Self::Integer(b)) => cmp_int_float(*b, *a).map(Ordering::reverse),
            (Self::String(a), Self::String(b)) => Some(a.as_bytes().cmp(b.as_bytes())),
            _ => None,
        }
    }

    /// [`lua_cmp`](Self::lua_cmp) as a `Result`, for use with `?`.
    pub fn try_cmp(&self, other: &Value) -> Result<Ordering> {
        self.lua_cmp(other).ok_or(Error::Incomparable {
            left: self.type_name(),
            right: other.type_name(),
        })
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            // Exact: 2^53 + 1 is not equal to the float 2^53.
            (Self::Integer(i), Self::Number(n)) | (Self::Number(n), Self::Integer(i)) => {
                f64_to_i64_exact(*n) == Some(*i)
            }
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Table(a), Self::Table(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

/// Write a float so that integral floats keep a `.0`.
pub(crate) fn write_number(f: &mut impl fmt::Write, n: f64) -> fmt::Result {
    if n.is_nan() {
        f.write_str(if n.is_sign_negative() { "-nan" } else { "nan" })
    } else if n.is_infinite() {
        f.write_str(if n > 0.0 { "inf" } else { "-inf" })
    } else if n == n.trunc() && n.abs() < 1e16 {
        write!(f, "{n:.1}")
    } else {
        write!(f, "{n}")
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s:?}"),
            other => fmt::Display::fmt(other, f),
        }
    }
}

/// Strings print without quotes; whole-number floats keep a `.0`.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => f.write_str("nil"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Number(n) => write_number(f, *n),
            Self::String(s) => f.write_str(s),
            Self::Table(t) => write!(f, "table: {:p}", t.as_ptr()),
        }
    }
}

impl From<()> for Value {
    #[inline]
    fn from(_: ()) -> Self {
        Self::Nil
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

macro_rules! from_small_int {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            #[inline]
            fn from(i: $t) -> Self {
                Self::Integer(i64::from(i))
            }
        }
    )*};
}
from_small_int!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! from_wide_int {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            /// Becomes an integer when it fits in `i64`, otherwise a float.
            #[inline]
            fn from(i: $t) -> Self {
                match i64::try_from(i) {
                    Ok(i) => Self::Integer(i),
                    Err(_) => Self::Number(i as f64),
                }
            }
        }
    )*};
}
from_wide_int!(isize, usize, u64);

impl From<f64> for Value {
    #[inline]
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<f32> for Value {
    #[inline]
    fn from(n: f32) -> Self {
        Self::Number(n.into())
    }
}

impl From<String> for Value {
    #[inline]
    fn from(s: String) -> Self {
        Self::String(Rc::from(s))
    }
}

impl From<&String> for Value {
    #[inline]
    fn from(s: &String) -> Self {
        Self::String(Rc::from(s.as_str()))
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(s: &str) -> Self {
        Self::String(Rc::from(s))
    }
}

impl From<Rc<str>> for Value {
    #[inline]
    fn from(s: Rc<str>) -> Self {
        Self::String(s)
    }
}

impl From<char> for Value {
    fn from(c: char) -> Self {
        Self::String(Rc::from(c.encode_utf8(&mut [0; 4]) as &str))
    }
}

impl From<Table> for Value {
    #[inline]
    fn from(t: Table) -> Self {
        Self::Table(t)
    }
}

impl From<&Table> for Value {
    #[inline]
    fn from(t: &Table) -> Self {
        Self::Table(t.clone())
    }
}

impl From<&Value> for Value {
    #[inline]
    fn from(v: &Value) -> Self {
        v.clone()
    }
}

/// `None` becomes `nil`.
impl<T: Into<Value>> From<Option<T>> for Value {
    #[inline]
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Nil, Into::into)
    }
}

/// A vector becomes a sequence table.
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Self::Table(Table::from_seq(v))
    }
}

impl From<Key> for Value {
    #[inline]
    fn from(k: Key) -> Self {
        match k {
            Key::Boolean(b) => Self::Boolean(b),
            Key::Integer(i) => Self::Integer(i),
            Key::Number(bits) => Self::Number(f64::from_bits(bits)),
            Key::String(s) => Self::String(s),
            Key::Table(t) => Self::Table(t),
        }
    }
}
