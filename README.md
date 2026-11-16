# rustable (`data-table`)

A Lua-inspired **table**: one flexible data structure for arrays, dictionaries, objects, records, sets, and modules.

```rust
use data_table::{table, Table, Value};

let t = Table::new();
t.push("first").unwrap();             // t[1] = "first"
t.set("name", "rustable").unwrap();   // t.name = "rustable"
assert_eq!(t.get(1), Value::from("first"));
assert_eq!(t.len(), 1);               // #t

let seq = table![3, 1, 2];            // {3, 1, 2}
seq.sort().unwrap();
assert_eq!(seq.concat(", ").unwrap(), "1, 2, 3");

let point = table! { "x" => 1, "y" => 2 };   // {x = 1, y = 2}
for (key, value) in point.pairs() {
    println!("{key} = {value}");
}
```

## Semantics

- Sequence positions are **1-based**.
- Assigning `nil` deletes a field; `nil` and NaN are never valid keys.
- `1` and `1.0` are the same key. Integers and floats compare exactly.
- Tables are **references**: cloning a `Table` is a new handle to the same table, and tables compare and hash by identity. Use `copy()` or `deep_clone()` for a new table.
- `len()` is Lua's `#t`: a border of the sequence. `field_count()` counts every field.

## API

| Lua | Rust |
| --- | --- |
| `t[k]`, `t[k] = v` | `get`, `set`, `get_field`, `set_field`, `contains_key`, `get_integer`, `get_number`, `get_bool` |
| `#t` | `len` (also `field_count`, `is_empty`) |
| `t[#t + 1] = v` | `push`, `pop` |
| `table.insert` / `table.remove` | `insert(pos, v)`, `remove(pos)` |
| `table.concat` | `concat(sep)`, `concat_range(sep, i, j)` |
| `table.unpack` / `table.pack` | `unpack`, `unpack_range(i, j)`, `Table::pack` |
| `table.move` | `move_within(f, e, pos)`, `move_to(f, e, pos, &dest)` |
| `table.sort` | `sort`, `sort_by`, `sort_unstable_by` |
| `pairs` / `ipairs` / `next` | `pairs`, `ipairs`, `next`, `keys`, `values`, `for_each` |
| `{...}` | `table![...]`, `table!{k => v}`, `from_seq`, `from_pairs`, `collect()` |
| sets | `set_add`, `set_contains`, `set_remove` |
| — | `copy`, `deep_clone` (cycle-safe), `reverse`, `clear`, `with_slice`, `dump` |
| — | `borrow()` → `TableRef`: `get`, `iter`, `as_slice` by reference, with no cloning |

Not implemented: metatables, weak tables, and function values.

## Design

| Module | Role |
| --- | --- |
| `value` | Dynamic values (`nil`, bool, integer, float, string, table). Strings are `Rc<str>`, so every value clones in O(1). |
| `key` | `Key` (owned), `KeyRef` (borrowed), and the `AsKey` trait. Lookups hash a borrowed key and never allocate. |
| `table` | Array part + hash part, and the Lua `table` library. |
| `hash` | The hash part: a compact insertion-ordered map. |
| `iter` | `Pairs` and `IPairs`. |

**Array part.** Keys `1..=n` live in a `Vec`. Two invariants are maintained on every write: the last array slot is never `nil`, and the hash part never holds the key `n + 1`. So `len()` is O(1) with no binary search, `push` is a plain vector push, and keys set out of order (for example filling `t[n]` down to `t[1]`) migrate into the array as soon as they become contiguous.

**Hash part.** Entries sit in a dense vector in insertion order; a SwissTable (`hashbrown::HashTable`) of 4-byte indices points into it. Iteration is a linear scan, `next` is O(1), each entry caches its hash so growth never re-hashes a string, and removal leaves a tombstone that is reclaimed on a later insert. That is what makes it safe to clear fields while traversing, as in Lua.

**Sorting.** `sort` checks the element types once, then sorts all-integer sequences as raw `i64`s, all-float sequences as raw `f64`s, and strings as (first 8 bytes, value) pairs so most comparisons never follow the string pointer.

**Hot paths.** Overwriting a sequence slot, appending, and popping are forced inline and kept free of drop and unwind code; rarer cases live in separate out-of-line functions. Hash lookups have one specialized function per key type, taking and returning everything in registers.

**Reading without cloning.** `get` returns an owned `Value`, which for strings and tables means a reference-count update. `t.borrow()` returns a `TableRef` whose `get` and `iter` hand out plain references instead; `get_integer`, `get_number` and `get_bool` do the same for scalar fields.

The crate contains no `unsafe` code.

## Caveats

- Tables are single-threaded (`Rc`), like Lua states.
- A table that contains itself is not freed until the cycle is broken (`clear()`).
- Closures passed to `sort_by`, `for_each` and `with_slice` must not modify the table they were called on (it panics), and neither may anything else while a `TableRef` from `borrow()` is alive. The `pairs`/`ipairs` iterators have no such restriction.
- `concat` and `Display` format floats with Rust's shortest round-trip representation rather than C's `%.14g`.

## Testing and benchmarks

```bash
cargo test
```

```bash
cargo run --release --example bench
```
