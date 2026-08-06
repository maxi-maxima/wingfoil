## The raw demux primitive — routing by slot index

`Builder::demux` is what [`demux_map`](../demux_map/) and
[`demux_it`](../demux_it/) are built on. It pre-wires `SIZE` children plus one
overflow child and, each cycle, asks a routing closure for a single `usize`;
the chosen child re-emits the source value and the rest stay quiet. There is no
key lifecycle at this level — `route` hands back an index and nothing else.

This example builds the same price book as `demux_map`, over the same
phase-shifted scenario, with the key→slot bookkeeping written out by hand. Put
side by side with `demux_map`, it is exactly what that API does for you:

```rust,ignore
struct SlotPool {
    assigned: HashMap<Instrument, usize>,
    free: BTreeSet<usize>,   // BTreeSet, not HashSet: lowest free slot wins,
}                            // so assignment is deterministic across runs

// `route` must be `Fn`, so the pool lives behind a `RefCell`.
b.demux(events, SIZE, move |event: &InstEvent| {
    let key = inst_key(event);
    let mut pool = pool.borrow_mut();
    match event {
        InstEvent::Delete(_) => pool.release(&key),
        _ => pool.assign(key),
    }
    .unwrap_or(SIZE)         // anything >= SIZE is the overflow child
})
```

```text
t=  0.0s  price book (demux_raw): {inst1=101}
t=  1.0s  price book (demux_raw): {inst1=101, inst2=202}
t=  2.0s  price book (demux_raw): {inst1=101, inst2=202, inst3=303}
t=  2.5s  price book (demux_raw): {inst2=202, inst3=303}
t=  3.0s  price book (demux_raw): {inst2=204, inst3=303}
t=  4.0s  price book (demux_raw): {inst2=204, inst3=305}
t=  5.0s  price book (demux_raw): {inst2=204, inst3=305, inst4=406}
t=  5.5s  price book (demux_raw): {inst3=305, inst4=406}
t=  6.0s  price book (demux_raw): {inst3=307, inst4=406}
t=  7.0s  price book (demux_raw): {inst3=307, inst4=408}
t=  8.0s  price book (demux_raw): {inst3=307, inst4=408, inst5=509}
t=  8.5s  price book (demux_raw): {inst4=408, inst5=509}
t=  9.0s  price book (demux_raw): {inst4=410, inst5=509}
t= 10.0s  price book (demux_raw): {inst4=410, inst5=511}
t= 11.0s  price book (demux_raw): {inst4=410, inst5=511, inst6=612}
t= 11.5s  price book (demux_raw): {inst5=511, inst6=612}
t= 12.0s  price book (demux_raw): {inst5=513, inst6=612}
t= 13.0s  price book (demux_raw): {inst5=513, inst6=614}
t= 14.0s  price book (demux_raw): {inst5=513, inst6=614, inst7=715}
t= 14.5s  price book (demux_raw): {inst6=614, inst7=715}
t= 15.0s  price book (demux_raw): {inst6=616, inst7=715}
t= 16.0s  price book (demux_raw): {inst6=616, inst7=717}
t= 17.0s  price book (demux_raw): {inst6=616, inst7=717, inst8=818}
t= 17.5s  price book (demux_raw): {inst7=717, inst8=818}
t= 18.0s  price book (demux_raw): {inst7=719, inst8=818}
t= 19.0s  price book (demux_raw): {inst7=719, inst8=820}
```

Byte for byte what `demux_map` prints — which is the claim this example makes,
and what its test asserts against the shared oracle.

```bash
cargo run --manifest-path crates/wingfoil/Cargo.toml --example demux_raw --features dynamic-graph
```

Use the raw form only when the slot policy is the point: sticky assignment that
survives a key's absence, priority slots, partitioning by hash rather than by
first-seen order, or a pool shared between several demuxes. For the ordinary
"give each new key a free slot, hand it back on `Close`" rule, `demux_map` and
`demux_it` already say it in one call.
