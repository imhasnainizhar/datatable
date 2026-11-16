//! Quick throughput check against the std collections a table replaces.
//!
//! ```text
//! cargo run --release --example bench
//! ```
//!
//! `Vec<i64>` and `HashMap<String, i64>` are statically typed, so they are a
//! floor rather than a fair fight: a table stores dynamically typed values
//! behind a shared handle.

use data_table::{Table, Value};
use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

const N: usize = 1_000_000;

fn time<R>(label: &str, ops: usize, mut f: impl FnMut() -> R) -> R {
    // Best of a few runs keeps scheduler noise out of the number.
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..5 {
        let start = Instant::now();
        out = Some(black_box(f()));
        best = best.min(start.elapsed().as_secs_f64());
    }
    println!("{label:<44} {:>7.2} ns/op", best * 1e9 / ops as f64);
    out.unwrap()
}

/// Small tables that fit in cache: measures instructions per operation
/// rather than memory latency.
fn small() {
    const LEN: usize = 1024;
    const REPS: usize = 2000;
    const OPS: usize = LEN * REPS;
    println!("-- small tables ({LEN} elements, in cache) --");
    time("small  push (preallocated)", OPS, || {
        let mut last = 0;
        for _ in 0..REPS {
            let t = Table::with_capacity(LEN, 0);
            for i in 0..LEN as i64 {
                t.push(i).unwrap();
            }
            last += t.len();
        }
        last
    });
    let t = Table::from_seq(0..LEN as i64);
    time("small  get(i)", OPS, || {
        let mut sum = 0;
        for _ in 0..REPS {
            for i in 1..=LEN as i64 {
                sum += t.get(black_box(i)).as_integer().unwrap();
            }
        }
        sum
    });
    time("small  set(i)", OPS, || {
        for _ in 0..REPS {
            for i in 1..=LEN as i64 {
                t.set(black_box(i), i).unwrap();
            }
        }
    });
    time("small  push all, pop all (stack)", 2 * OPS, || {
        let stack = Table::with_capacity(LEN, 0);
        let mut sum = 0;
        for _ in 0..REPS {
            for i in 0..LEN as i64 {
                stack.push(i).unwrap();
            }
            for _ in 0..LEN {
                sum += stack.pop().as_integer().unwrap();
            }
        }
        sum
    });

    // A record with a handful of fields: the "object" use case.
    let names = [
        "id", "name", "position", "velocity", "health", "inventory", "target", "state",
    ];
    let obj = Table::new();
    for (i, n) in names.iter().enumerate() {
        obj.set(*n, i).unwrap();
    }
    obj.set("label", "a string value").unwrap();
    let field_ops = names.len() * 250_000;
    time("record get (integer field)", field_ops, || {
        let mut sum = 0;
        for _ in 0..250_000 {
            for n in names {
                sum += obj.get(black_box(n)).as_integer().unwrap();
            }
        }
        sum
    });
    time("record set (overwrite)", field_ops, || {
        for i in 0..250_000 {
            for n in names {
                obj.set(black_box(n), i).unwrap();
            }
        }
    });
    time("record get (string field)", 2_000_000, || {
        let mut len = 0;
        for _ in 0..2_000_000 {
            len += obj.get(black_box("label")).as_str().unwrap().len();
        }
        len
    });
    time("record get_integer (no clone)", field_ops, || {
        let mut sum = 0;
        for _ in 0..250_000 {
            for n in names {
                sum += obj.get_integer(black_box(n)).unwrap();
            }
        }
        sum
    });
    time("record borrow().get (string field)", 2_000_000, || {
        let mut len = 0;
        let view = obj.borrow();
        for _ in 0..2_000_000 {
            len += view.get(black_box("label")).and_then(Value::as_str).unwrap().len();
        }
        len
    });
    time("record borrow().iter", 9 * 250_000, || {
        let mut n = 0;
        for _ in 0..250_000 {
            n += obj.borrow().iter().count();
        }
        n
    });
    time("record pairs", 9 * 250_000, || {
        let mut n = 0;
        for _ in 0..250_000 {
            n += obj.pairs().count();
        }
        n
    });
    let floats = Table::from_seq((0..LEN).map(|i| ((i * 7919) % LEN) as f64 + 0.5));
    time("small  sort floats", OPS / 4, || {
        let mut n = 0;
        for _ in 0..REPS / 4 {
            let s = floats.copy();
            s.sort().unwrap();
            n += s.len();
        }
        n
    });
    let strs = Table::from_seq((0..LEN).map(|i| format!("s{}", (i * 7919) % LEN)));
    time("small  sort strings", OPS / 4, || {
        let mut n = 0;
        for _ in 0..REPS / 4 {
            let s = strs.copy();
            s.sort().unwrap();
            n += s.len();
        }
        n
    });
}

fn main() {
    small();
    let keys: Vec<String> = (0..N).map(|i| format!("field_{i}")).collect();
    // Visit keys in a scattered order so lookups are not cache-friendly.
    let order: Vec<usize> = (0..N).map(|i| (i * 7919) % N).collect();

    println!("{N} elements\n-- sequence --");
    let t = time("table  push", N, || {
        let t = Table::new();
        for i in 0..N as i64 {
            t.push(i).unwrap();
        }
        t
    });
    time("Vec    push", N, || {
        let mut v = Vec::new();
        for i in 0..N as i64 {
            v.push(black_box(i));
        }
        v
    });
    time("table  get(i)", N, || {
        let mut sum = 0;
        for &i in &order {
            sum += t.get(i + 1).as_integer().unwrap();
        }
        sum
    });
    time("table  set(i)", N, || {
        for &i in &order {
            t.set(i + 1, i).unwrap();
        }
    });
    time("table  ipairs", N, || t.ipairs().map(|(_, v)| v.as_integer().unwrap()).sum::<i64>());
    time("table  with_slice sum", N, || {
        t.with_slice(|s| s.iter().map(|v| v.as_integer().unwrap()).sum::<i64>())
    });
    time("table  sort (shuffled ints)", N, || {
        let s = Table::from_seq(order.iter().map(|&i| i as i64));
        s.sort().unwrap();
        s
    });
    time("Vec    sort_unstable (shuffled ints)", N, || {
        let mut v: Vec<i64> = order.iter().map(|&i| i as i64).collect();
        v.sort_unstable();
        v
    });
    time("table  reverse fill t[n..1]", N, || {
        let t = Table::new();
        for i in (1..=N as i64).rev() {
            t.set(i, i).unwrap();
        }
        t
    });

    println!("-- string keys --");
    let d = time("table  insert", N, || {
        let t = Table::new();
        for (i, k) in keys.iter().enumerate() {
            t.set(k, i).unwrap();
        }
        t
    });
    let h = time("HashMap insert (String keys, SipHash)", N, || {
        let mut m = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            m.insert(k.clone(), i as i64);
        }
        m
    });
    time("table  get", N, || {
        let mut sum = 0;
        for &i in &order {
            sum += d.get(&keys[i]).as_integer().unwrap();
        }
        sum
    });
    time("HashMap get", N, || {
        let mut sum = 0;
        for &i in &order {
            sum += h[&keys[i]];
        }
        sum
    });
    time("table  overwrite", N, || {
        for &i in &order {
            d.set(&keys[i], i).unwrap();
        }
    });
    time("table  pairs", N, || d.pairs().count());
    time("table  borrow().iter", N, || {
        d.borrow().iter().map(|(_, v)| v.as_integer().unwrap()).sum::<i64>()
    });
    time("table  for_each", N, || {
        let mut sum = 0;
        d.for_each(|_, v| sum += v.as_integer().unwrap());
        sum
    });
    time("HashMap iter", N, || h.values().sum::<i64>());
    time("table  remove all + reinsert", 2 * N, || {
        for k in &keys {
            d.set(k, Value::Nil).unwrap();
        }
        for (i, k) in keys.iter().enumerate() {
            d.set(k, i).unwrap();
        }
    });
}
