//! A Lua-inspired table: one flexible data structure.
//!
//! A [`Table`] can stand in for:
//!
//! - **arrays** / lists — integer keys in a dense sequence
//! - **dictionaries** — arbitrary keys
//! - **objects** / **records** — string fields
//! - **sets** — keys with a true-like value
//! - **modules** — a namespace of named fields
//!
//! ```
//! use datatable::{table, Table, Value};
//!
//! let t = Table::new();
//! t.push("first").unwrap();            // t[1]
//! t.set("name", "datatable").unwrap();  // t.name
//! assert_eq!(t.len(), 1);
//! assert_eq!(t.get("name"), Value::from("datatable"));
//!
//! let seq = table![3, 1, 2];
//! seq.sort().unwrap();
//! assert_eq!(seq.concat(", ").unwrap(), "1, 2, 3");
//!
//! let point = table! { "x" => 1, "y" => 2 };
//! assert_eq!(point.field_count(), 2);
//! ```
//!
//! Sequence positions are 1-based, as in Lua. Assigning `nil` deletes a
//! field. See [`Table`] for the full API, which mirrors Lua's `table`
//! library (`insert`, `remove`, `concat`, `unpack`, `pack`, `move`, `sort`)
//! together with `pairs`, `ipairs` and `next`.

#![forbid(unsafe_code)]

pub mod error;
mod hash;
pub mod iter;
pub mod key;
pub mod table;
pub mod value;

pub use error::{Error, Result};
pub use iter::{IPairs, Iter, Pairs};
pub use key::{AsKey, Key, KeyRef};
pub use table::{Table, TableRef};
pub use value::Value;

/// Build a [`Table`] like a Lua table constructor.
///
/// ```
/// use datatable::table;
///
/// let empty = table![];
/// let seq = table![1, "two", 3.0];           // {1, "two", 3.0}
/// let rec = table! { "x" => 1, "y" => 2 };   // {x = 1, y = 2}
/// assert!(empty.is_empty());
/// assert_eq!(seq.len(), 3);
/// assert_eq!(rec.field_count(), 2);
/// ```
///
/// The key/value form panics on a `nil` or NaN key.
#[macro_export]
macro_rules! table {
    () => {
        $crate::Table::new()
    };
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let t = $crate::Table::new();
        $( t.set($key, $value).expect("invalid table key"); )+
        t
    }};
    ($($value:expr),+ $(,)?) => {
        $crate::Table::from_seq([$($crate::Value::from($value)),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_table_is_a_distinct_reference() {
        let a = Table::new();
        let b = a.clone();
        let c = Table::new();
        assert!(a.ptr_eq(&b));
        assert!(!a.ptr_eq(&c));
    }

    #[test]
    fn value_truthiness_matches_lua() {
        assert!(!Value::Nil.is_truthy());
        assert!(!Value::from(false).is_truthy());
        assert!(Value::from(true).is_truthy());
        assert!(Value::from(0_i64).is_truthy());
        assert!(Value::from("").is_truthy());
    }

    #[test]
    fn array_and_dict_in_one_table() {
        let t = Table::new();
        t.set(1, "alpha").unwrap();
        t.set("kind", "mixed").unwrap();
        assert_eq!(t.get(1), Value::from("alpha"));
        assert_eq!(t.get("kind"), Value::from("mixed"));
        assert_eq!(t.get(2), Value::Nil);
    }

    #[test]
    fn setting_nil_removes_the_field() {
        let t = Table::new();
        t.set("x", 1_i64).unwrap();
        t.set("x", Value::Nil).unwrap();
        assert_eq!(t.get("x"), Value::Nil);
        assert!(!t.contains_key("x"));
    }

    #[test]
    fn len_is_the_array_border() {
        let t = Table::new();
        t.set(1, "a").unwrap();
        t.set(2, "b").unwrap();
        t.set("n", 99_i64).unwrap();
        assert_eq!(t.len(), 2);
    }
}
