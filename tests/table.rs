// `Key` can hold a table handle, but hashes it by identity, so it is a sound
// map key.
#![allow(clippy::mutable_key_type)]

use flextable::{table, Error, Key, Table, Value};
use std::cmp::Ordering;
use std::collections::HashMap;

fn ints(t: &Table) -> Vec<i64> {
    t.unpack().iter().map(|v| v.as_integer().unwrap()).collect()
}

// --- keys ---------------------------------------------------------------

#[test]
fn float_keys_normalize_to_integers() {
    let t = Table::new();
    t.set(1.0, "a").unwrap();
    assert_eq!(t.get(1), Value::from("a"));
    assert_eq!(t.len(), 1);
    t.set(2, "b").unwrap();
    assert_eq!(t.get(2.0), Value::from("b"));
    t.set(1.5, "frac").unwrap();
    assert_eq!(t.get(1.5), Value::from("frac"));
    assert_eq!(t.field_count(), 3);
}

#[test]
fn nil_and_nan_keys_are_rejected() {
    let t = Table::new();
    assert_eq!(t.set(Value::Nil, 1), Err(Error::NilKey));
    assert_eq!(t.set(f64::NAN, 1), Err(Error::InvalidKey));
    assert_eq!(t.get(Value::Nil), Value::Nil);
    assert_eq!(t.get(f64::NAN), Value::Nil);
    assert!(t.is_empty());
}

#[test]
fn all_key_types_coexist() {
    let t = Table::new();
    let k = Table::new();
    t.set(true, "bool").unwrap();
    t.set(-3, "neg").unwrap();
    t.set(0, "zero").unwrap();
    t.set("s", "str").unwrap();
    t.set(&k, "tbl").unwrap();
    t.set(String::from("owned"), 1).unwrap();
    assert_eq!(t.get(true), Value::from("bool"));
    assert_eq!(t.get(false), Value::Nil);
    assert_eq!(t.get(-3), Value::from("neg"));
    assert_eq!(t.get(0), Value::from("zero"));
    assert_eq!(t.get(&k), Value::from("tbl"));
    assert_eq!(
        t.get(Table::new()),
        Value::Nil,
        "tables are keyed by identity"
    );
    assert_eq!(t.get(Value::from("owned")), Value::from(1));
    assert_eq!(t.len(), 0);
    assert_eq!(t.field_count(), 6);
}

#[test]
fn integer_float_equality_is_exact() {
    let big = (1_i64 << 53) + 1;
    assert_ne!(Value::from(big), Value::from((1_i64 << 53) as f64));
    assert_eq!(Value::from(3), Value::from(3.0));
    assert_eq!(
        Value::from(big).lua_cmp(&Value::from((1_i64 << 53) as f64)),
        Some(Ordering::Greater)
    );
    assert_eq!(
        Value::from(1).lua_cmp(&Value::from(1.5)),
        Some(Ordering::Less)
    );
    assert_eq!(Value::from(1).lua_cmp(&Value::from("1")), None);
}

// --- array / hash interplay ---------------------------------------------

#[test]
fn reverse_fill_ends_up_as_a_sequence() {
    let t = Table::new();
    for i in (1..=1000).rev() {
        t.set(i, i).unwrap();
        if i > 1 {
            assert_eq!(t.len(), 0);
        }
    }
    assert_eq!(t.len(), 1000);
    assert_eq!(t.field_count(), 1000);
    assert_eq!(ints(&t), (1..=1000).collect::<Vec<_>>());
}

#[test]
fn holes_keep_len_a_valid_border() {
    let t = table![1, 2, 3, 4, 5];
    t.set(3, Value::Nil).unwrap();
    assert_eq!(t.len(), 5);
    assert_eq!(t.field_count(), 4);
    t.set(5, Value::Nil).unwrap();
    assert_eq!(t.len(), 4);
    t.set(4, Value::Nil).unwrap();
    assert_eq!(t.len(), 2, "trailing holes are trimmed");
    assert_eq!(t.field_count(), 2);
    t.set(3, "back").unwrap();
    assert_eq!(t.len(), 3);
}

#[test]
fn sparse_integers_do_not_allocate_an_array() {
    let t = Table::new();
    t.set(1_000_000_000, "far").unwrap();
    assert_eq!(t.len(), 0);
    assert_eq!(t.get(1_000_000_000), Value::from("far"));
    assert_eq!(t.field_count(), 1);
}

// --- table library ------------------------------------------------------

#[test]
fn push_pop() {
    let t = Table::new();
    for i in 1..=5 {
        t.push(i).unwrap();
    }
    t.push(Value::Nil).unwrap();
    assert_eq!(t.len(), 5);
    assert_eq!(t.pop(), Value::from(5));
    assert_eq!(t.len(), 4);
    let e = Table::new();
    assert_eq!(e.pop(), Value::Nil);
}

#[test]
fn insert_and_remove_shift() {
    let t = table![1, 2, 4];
    t.insert(3, 3).unwrap();
    t.insert(1, 0).unwrap();
    t.insert(6, 5).unwrap();
    assert_eq!(ints(&t), [0, 1, 2, 3, 4, 5]);
    assert_eq!(
        t.insert(8, 9),
        Err(Error::PositionOutOfBounds {
            position: 8,
            len: 6
        })
    );
    assert!(t.insert(0, 9).is_err());

    assert_eq!(t.remove(1).unwrap(), Value::from(0));
    assert_eq!(t.remove(5).unwrap(), Value::from(5));
    assert_eq!(t.remove(5).unwrap(), Value::Nil, "#t + 1 is allowed");
    assert_eq!(ints(&t), [1, 2, 3, 4]);
    assert!(t.remove(6).is_err());
    assert!(t.remove(0).is_err());

    let e = Table::new();
    e.set(0, "zero").unwrap();
    assert_eq!(e.remove(0).unwrap(), Value::from("zero"));
    assert!(e.is_empty());
}

#[test]
fn insert_pulls_following_keys_out_of_the_hash() {
    let t = table![1, 2];
    t.set(4, 4).unwrap();
    assert_eq!(t.len(), 2);
    t.insert(1, 0).unwrap(); // sequence becomes 1..=3, and 4 joins it
    assert_eq!(t.len(), 4);
    assert_eq!(ints(&t), [0, 1, 2, 4]);
}

#[test]
fn concat() {
    let t = table![1, "a", 2.5, 3.0];
    assert_eq!(t.concat(", ").unwrap(), "1, a, 2.5, 3.0");
    assert_eq!(t.concat_range("-", 2, 3).unwrap(), "a-2.5");
    assert_eq!(t.concat_range("-", 3, 2).unwrap(), "");
    assert_eq!(Table::new().concat(",").unwrap(), "");
    t.push(true).unwrap();
    assert_eq!(
        t.concat(""),
        Err(Error::TypeMismatch {
            expected: "string or number",
            found: "boolean"
        })
    );
    assert!(t.concat_range("", 1, 9).is_err(), "nil past the end");
}

#[test]
fn pack_and_unpack() {
    let t = Table::pack([Value::from(1), Value::Nil, Value::from(3), Value::Nil]);
    assert_eq!(t.get("n"), Value::from(4));
    assert_eq!(t.len(), 3);
    assert_eq!(t.unpack(), [Value::from(1), Value::Nil, Value::from(3)]);
    assert_eq!(
        t.unpack_range(2, 5),
        [Value::Nil, Value::from(3), Value::Nil, Value::Nil]
    );
    assert!(t.unpack_range(3, 2).is_empty());
}

#[test]
fn move_within_handles_overlap() {
    let t = table![1, 2, 3, 4, 5];
    t.move_within(1, 3, 2).unwrap(); // shift right, overlapping
    assert_eq!(ints(&t), [1, 1, 2, 3, 5]);
    let t = table![1, 2, 3, 4, 5];
    t.move_within(2, 5, 1).unwrap(); // shift left, overlapping
    assert_eq!(ints(&t), [2, 3, 4, 5, 5]);
    let t = table![1, 2, 3];
    t.move_within(1, 3, 4).unwrap(); // append a copy of itself
    assert_eq!(ints(&t), [1, 2, 3, 1, 2, 3]);
    t.move_within(3, 1, 9).unwrap(); // empty range
    assert_eq!(t.len(), 6);
    assert!(t.move_within(1, 2, i64::MAX).is_err());
}

#[test]
fn move_to_another_table() {
    let a = table![1, 2, 3];
    let b = table![9];
    a.move_to(1, 3, 2, &b).unwrap();
    assert_eq!(ints(&b), [9, 1, 2, 3]);
    let c = Table::new();
    a.move_to(2, 3, 5, &c).unwrap();
    assert_eq!(c.len(), 0);
    assert_eq!(c.get(5), Value::from(2));
    assert_eq!(c.get(6), Value::from(3));
    a.move_to(1, 3, 2, &a).unwrap();
    assert_eq!(ints(&a), [1, 1, 2, 3]);
}

#[test]
fn sort_numbers_and_strings() {
    let t = table![5, 3, 9, 1, 3];
    t.sort().unwrap();
    assert_eq!(ints(&t), [1, 3, 3, 5, 9]);

    let m = table![2.5, 1, f64::NAN, -1.0, 2];
    m.sort().unwrap();
    assert_eq!(m.concat_range(" ", 1, 4).unwrap(), "-1.0 1 2 2.5");
    assert!(matches!(m.get(5), Value::Number(n) if n.is_nan()));

    let s = table!["pear", "apple", "fig"];
    s.sort().unwrap();
    assert_eq!(s.concat(" ").unwrap(), "apple fig pear");

    let bad = table![1, "x"];
    assert_eq!(
        bad.sort(),
        Err(Error::Incomparable {
            left: "integer",
            right: "string"
        })
    );
    assert_eq!(
        bad.get(1),
        Value::from(1),
        "failed sort leaves the table alone"
    );
    Table::new().sort().unwrap();
}

#[test]
fn sort_by_comparator() {
    let t = table![1, 4, 2, 3];
    t.sort_by(|a, b| b.lua_cmp(a).unwrap());
    assert_eq!(ints(&t), [4, 3, 2, 1]);
    t.sort_unstable_by(|a, b| a.lua_cmp(b).unwrap());
    assert_eq!(ints(&t), [1, 2, 3, 4]);
    t.reverse();
    assert_eq!(ints(&t), [4, 3, 2, 1]);
}

// --- traversal ----------------------------------------------------------

#[test]
fn pairs_order_is_sequence_then_insertion() {
    let t = table![10, 20];
    t.set("b", 1).unwrap();
    t.set("a", 2).unwrap();
    t.set(true, 3).unwrap();
    let keys: Vec<String> = t.keys().map(|k| k.to_string()).collect();
    assert_eq!(keys, ["1", "2", "b", "a", "true"]);
    let sum: i64 = t.values().map(|v| v.as_integer().unwrap()).sum();
    assert_eq!(sum, 36);
    let mut n = 0;
    t.for_each(|_, _| n += 1);
    assert_eq!(n, 5);
    assert_eq!((&t).into_iter().count(), 5);
}

#[test]
fn ipairs_stops_at_the_first_hole() {
    let t = table![1, 2, 3, 4];
    t.set(3, Value::Nil).unwrap();
    assert_eq!(t.ipairs().count(), 2);
    assert_eq!(t.pairs().count(), 3);
}

#[test]
fn fields_can_be_cleared_while_traversing() {
    let t = Table::new();
    for i in 1..=50 {
        t.push(i).unwrap();
        t.set(format!("k{i}"), i).unwrap();
    }
    let mut seen = 0;
    for (k, _) in t.pairs() {
        t.set(k, Value::Nil).unwrap();
        seen += 1;
    }
    assert_eq!(seen, 100);
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
}

#[test]
fn next_walks_and_survives_removal() {
    let t = table![1, 2, 3];
    t.set("x", 4).unwrap();
    t.set("y", 5).unwrap();
    let mut key = Value::Nil;
    let mut seen = Vec::new();
    while let Some((k, v)) = t.next(&key).unwrap() {
        seen.push(v.as_integer().unwrap());
        t.set(&k, Value::Nil).unwrap(); // clear the field we are standing on
        key = k.into();
    }
    assert_eq!(seen, [1, 2, 3, 4, 5]);
    assert!(t.is_empty());
    assert_eq!(t.next(&Value::from("nope")), Err(Error::InvalidKey));
    assert_eq!(Table::new().next(&Value::Nil), Ok(None));
}

#[test]
fn removed_keys_are_reclaimed_and_reusable() {
    let t = Table::new();
    for round in 0..200 {
        let k = format!("key{round}");
        t.set(k.as_str(), round).unwrap();
        assert_eq!(t.get(k.as_str()), Value::from(round));
        t.set(k.as_str(), Value::Nil).unwrap();
        assert!(!t.contains_key(k.as_str()));
    }
    assert!(t.is_empty());
    t.set("a", 1).unwrap();
    t.set("a", Value::Nil).unwrap();
    t.set("a", 2).unwrap(); // revives the slot
    assert_eq!(t.get("a"), Value::from(2));
    assert_eq!(t.field_count(), 1);
}

// --- sets, records, copies ----------------------------------------------

#[test]
fn set_view() {
    let s = Table::new();
    s.set_add("a").unwrap();
    s.set_add(7).unwrap();
    assert!(s.set_contains("a"));
    assert!(s.set_contains(7));
    assert!(!s.set_contains("b"));
    s.set("off", false).unwrap();
    assert!(!s.set_contains("off"), "false is not membership");
    assert!(s.set_remove("a"));
    assert!(!s.set_remove("a"));
    assert!(s.set_remove(7));
    assert_eq!(s.field_count(), 1);
}

#[test]
fn record_view_and_nesting() {
    let inner = table! { "x" => 1 };
    let t = table! { "name" => "n", "pos" => &inner };
    assert_eq!(t.get_field("name"), Value::from("n"));
    t.get_field("pos")
        .as_table()
        .unwrap()
        .set_field("x", 2)
        .unwrap();
    assert_eq!(
        inner.get("x"),
        Value::from(2),
        "tables are shared references"
    );
}

#[test]
fn copy_is_shallow_and_deep_clone_is_deep() {
    let child = table![1];
    let t = table! { "c" => &child, "v" => 1 };
    t.push("seq").unwrap();

    let shallow = t.copy();
    assert!(!shallow.ptr_eq(&t));
    assert_eq!(shallow.get("c"), Value::from(&child));

    let deep = t.deep_clone();
    let deep_child = deep.get("c");
    assert_ne!(deep_child, Value::from(&child));
    deep_child.as_table().unwrap().push(2).unwrap();
    assert_eq!(child.len(), 1);
    assert_eq!(deep.get(1), Value::from("seq"));
    assert_eq!(deep.field_count(), 3);
}

#[test]
fn deep_clone_preserves_cycles_and_sharing() {
    let a = Table::new();
    let shared = table![1];
    a.set("me", &a).unwrap();
    a.set("s1", &shared).unwrap();
    a.set("s2", &shared).unwrap();
    let b = a.deep_clone();
    assert_eq!(b.get("me"), Value::from(&b));
    assert_eq!(b.get("s1"), b.get("s2"));
    assert_ne!(b.get("s1"), Value::from(&shared));
    assert_eq!(a.dump(), r#"{me = <cycle>, s1 = {1}, s2 = {1}}"#);
    a.clear(); // break the cycles so both tables are freed
    b.clear();
}

#[test]
fn dump_and_display() {
    let t = table![1, "two"];
    t.set("k", 1.5).unwrap();
    t.set("with space", true).unwrap();
    t.set(-1, 0).unwrap();
    assert_eq!(
        t.dump(),
        r#"{1, "two", k = 1.5, ["with space"] = true, [-1] = 0}"#
    );
    assert_eq!(Value::from(2.0).to_string(), "2.0");
    assert_eq!(Value::from("s").to_string(), "s");
    assert_eq!(format!("{:?}", Value::from("s")), "\"s\"");
}

#[test]
fn conversions_and_collect() {
    let seq: Table = (1..=3).map(Value::from).collect();
    assert_eq!(seq.len(), 3);
    let map: Table = [(Key::from("a"), Value::from(1))].into_iter().collect();
    assert_eq!(map.get("a"), Value::from(1));
    let nested = Value::from(vec![1, 2, 3]);
    assert_eq!(nested.as_table().unwrap().len(), 3);
    assert_eq!(Value::from(None::<i64>), Value::Nil);
    assert_eq!(Value::from(u64::MAX), Value::Number(u64::MAX as f64));
    let mut ext = Table::new();
    ext.extend([Value::from(1), Value::from(2)]);
    assert_eq!(ext.len(), 2);
    assert_eq!(seq.with_slice(|s| s.len()), 3);
}

#[test]
fn borrowed_view_reads_without_cloning() {
    let child = table![1];
    let t = table![10, 20, 30];
    t.set(2, Value::Nil).unwrap();
    t.set("name", "n").unwrap();
    t.set("child", &child).unwrap();
    t.set(99, 1.5).unwrap();
    {
        let view = t.borrow();
        assert_eq!(view.len(), 3);
        assert_eq!(view.field_count(), 5);
        assert!(!view.is_empty());
        assert_eq!(view.get(1), Some(&Value::from(10)));
        assert_eq!(view.get(2), None);
        assert_eq!(view.get(99), Some(&Value::from(1.5)));
        assert_eq!(view.get("name").and_then(Value::as_str), Some("n"));
        assert_eq!(view.get("missing"), None);
        assert_eq!(view.get(Value::Nil), None);
        assert!(view
            .get("child")
            .unwrap()
            .as_table()
            .unwrap()
            .ptr_eq(&child));
        assert_eq!(view.as_slice().len(), 3);
        let keys: Vec<String> = view.iter().map(|(k, _)| k.to_key().to_string()).collect();
        assert_eq!(keys, ["1", "3", "name", "child", "99"]);
        assert_eq!((&view).into_iter().count(), 5);
    }
    t.push(40).unwrap(); // the guard is gone, so writes work again
    assert_eq!(t.get_integer(4), Some(40));
    assert_eq!(t.get_integer("name"), None);
    assert_eq!(t.get_number(1), Some(10.0));
    assert_eq!(t.get_number(99), Some(1.5));
    assert_eq!(t.get_bool("name"), None);
    t.set("flag", true).unwrap();
    assert_eq!(t.get_bool("flag"), Some(true));
}

#[test]
fn sort_edge_cases() {
    // negative zero, infinities and both NaN signs
    let t = table![
        f64::INFINITY,
        -0.0,
        0.5,
        f64::NEG_INFINITY,
        -f64::NAN,
        f64::NAN,
        -2.5
    ];
    t.sort().unwrap();
    assert_eq!(t.concat_range(" ", 1, 5).unwrap(), "-inf -2.5 -0.0 0.5 inf");
    assert!(t.get_number(6).unwrap().is_nan() && t.get_number(7).unwrap().is_nan());

    // all floats, no NaN: the raw-f64 path
    let f = table![2.5, -1.5, 0.25];
    f.sort().unwrap();
    assert_eq!(f.concat(" ").unwrap(), "-1.5 0.25 2.5");

    // strings that share a prefix longer than 8 bytes, and embedded NULs
    let s = table!["prefix__b", "prefix__a", "a\0", "a", "", "prefix__", "b"];
    s.sort().unwrap();
    let got: Vec<String> = s.unpack().iter().map(|v| v.to_string()).collect();
    assert_eq!(
        got,
        ["", "a", "a\0", "b", "prefix__", "prefix__a", "prefix__b"]
    );
    assert_eq!(s.field_count(), 7);
}

#[test]
fn overwriting_heap_values_releases_them() {
    use std::rc::Rc;
    let text: Rc<str> = Rc::from("shared");
    let t = Table::new();
    t.push(Rc::clone(&text)).unwrap();
    t.set("k", Rc::clone(&text)).unwrap();
    assert_eq!(Rc::strong_count(&text), 3);
    t.set(1, 5).unwrap(); // overwrite in the array part
    t.set("k", 6).unwrap(); // overwrite in the hash part
    assert_eq!(Rc::strong_count(&text), 1);
    t.set(1, Rc::clone(&text)).unwrap();
    t.set(1, Value::Nil).unwrap();
    assert_eq!(Rc::strong_count(&text), 1);
}

// --- randomized check against a reference model -------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The model is the obvious implementation: one map from key to value.
fn check(t: &Table, model: &HashMap<Key, Value>) {
    assert_eq!(t.field_count(), model.len());
    assert_eq!(t.is_empty(), model.is_empty());
    let len = t.len() as i64;
    if len > 0 {
        assert!(
            model.contains_key(&Key::Integer(len)),
            "t[#t] must be non-nil"
        );
    }
    assert!(
        !model.contains_key(&Key::Integer(len + 1)),
        "t[#t + 1] must be nil"
    );
    let mut seen = 0;
    for (k, v) in t.pairs() {
        assert_eq!(model.get(&k), Some(&v), "pairs yielded a stale field {k}");
        seen += 1;
    }
    assert_eq!(seen, model.len());
    for (k, v) in model {
        assert_eq!(&t.get(k), v);
    }
}

#[test]
fn random_operations_match_the_model() {
    for seed in 1..=20u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let t = Table::new();
        let mut model: HashMap<Key, Value> = HashMap::new();
        for step in 0..3000 {
            let key = match rng.below(10) {
                0..=5 => Key::Integer(rng.below(40) as i64 - 3),
                6..=8 => Key::from(format!("s{}", rng.below(25))),
                _ => Key::Boolean(rng.below(2) == 0),
            };
            match rng.below(10) {
                0..=5 => {
                    let v = Value::from(step);
                    t.set(&key, v.clone()).unwrap();
                    model.insert(key, v);
                }
                6..=8 => {
                    t.set(&key, Value::Nil).unwrap();
                    model.remove(&key);
                }
                _ => {
                    // push / pop go through the border
                    let n = t.len() as i64;
                    if rng.below(2) == 0 {
                        t.push(step).unwrap();
                        model.insert(Key::Integer(n + 1), Value::from(step));
                    } else {
                        let popped = t.pop();
                        // An empty sequence pops nil and leaves t[0] alone.
                        let expect = if n == 0 {
                            Value::Nil
                        } else {
                            model.remove(&Key::Integer(n)).unwrap()
                        };
                        assert_eq!(popped, expect);
                    }
                }
            }
            if step % 64 == 0 {
                check(&t, &model);
            }
        }
        check(&t, &model);
    }
}

#[test]
fn random_insert_remove_match_a_vec() {
    let mut rng = Rng(42);
    let t = Table::new();
    let mut model: Vec<i64> = Vec::new();
    for step in 0..5000 {
        if model.is_empty() || rng.below(3) != 0 {
            let pos = rng.below(model.len() as u64 + 1) as usize;
            t.insert(pos as i64 + 1, step).unwrap();
            model.insert(pos, step);
        } else {
            let pos = rng.below(model.len() as u64) as usize;
            assert_eq!(
                t.remove(pos as i64 + 1).unwrap(),
                Value::from(model.remove(pos))
            );
        }
    }
    assert_eq!(ints(&t), model);
    t.sort().unwrap();
    model.sort_unstable();
    assert_eq!(ints(&t), model);
}
