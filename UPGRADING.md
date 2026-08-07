# Upgrading

## Fluent graph construction (`GraphBuilder`)

*Status: on `main`, ships in the next release after 8.0.0.  Additive — no
existing code stops compiling.*

### Why

Wingfoil's API is fluent everywhere except the last step.  A pipeline reads as
one chain — `ticker(period).count().map(f).print()` — and then execution had to
be spelled a different way: collect the roots into a `Vec`, and pass them
positionally to a constructor along with two configuration enums.

```rust
Graph::new(vec![prices_export, fills_export], RunMode::HistoricalFrom(NanoTime::ZERO), RunFor::Forever)
```

Three problems with that shape:

- **It breaks the chain.**  Wiring had to be assigned to intermediate `let`
  bindings purely so the roots could be named again inside a `vec![]`.
- **The arguments are positional and untyped at the call site.**  Two enums in
  a row, and nothing at the call site says which is the run mode and which is
  the duration.
- **There is no default.**  Every graph spells out both, even the common cases
  (real time, forever).

`GraphBuilder` closes the gap: wiring, configuration and execution are one
expression, each option is named, and the common cases are the defaults.

### What's new

A builder, reachable two ways:

- `NodeOperators::graph()` — continues an existing stream/node chain.
- `Graph::builder()` — for graphs with several roots.

```rust
// single root — the chain just keeps going
ticker(period).count().print().graph().historical().cycles(5).run().unwrap();

// several roots — `add` takes a Node, a Stream, or a Vec of either
Graph::builder()
    .add(prices.filter_none().distinct().csv_write("prices.csv"))
    .add(fills.csv_write("fills.csv"))
    .historical()
    .forever()
    .run()
    .unwrap();
```

Builder methods:

| Group | Methods |
| --- | --- |
| Roots | `add(impl AsUpstreamNodes)` — repeatable |
| Run mode | `real_time()` *(default)*, `historical()`, `historical_from(t)`, `run_mode(m)` |
| Duration | `forever()` *(default)*, `cycles(n)`, `duration(d)`, `run_for(f)` |
| Other | `start_time(t)`, `tokio_runtime(rt)` *(feature `async`)*, `print()` |
| Terminal | `run() -> anyhow::Result<()>`, `build() -> Graph` |

`RunMode` and `RunFor` now implement `Default` (`RealTime` / `Forever`), which
is what backs the builder's defaults — so a real-time production graph need only
name its roots:

```rust
Graph::builder().add(roots).run().unwrap();
```

### Migration

Mechanical, one call site at a time.

| Before | After |
| --- | --- |
| `stream.run(RunMode::RealTime, RunFor::Forever)` | `stream.graph().run()` |
| `stream.run(RunMode::RealTime, RunFor::Duration(d))` | `stream.graph().duration(d).run()` |
| `stream.run(RunMode::HistoricalFrom(NanoTime::ZERO), RunFor::Cycles(n))` | `stream.graph().historical().cycles(n).run()` |
| `stream.run(RunMode::HistoricalFrom(t), RunFor::Forever)` | `stream.graph().historical_from(t).forever().run()` |
| `Graph::new(vec![a, b], mode, run_for).run()` | `Graph::builder().add(a).add(b).run_mode(mode).run_for(run_for).run()` |
| `let mut g = Graph::new(roots, mode, f); g.print(); g.run()` | `Graph::builder().add(roots).run_mode(mode).run_for(f).print().run()` |
| `stream.into_graph(mode, run_for)` | `stream.graph().run_mode(mode).run_for(run_for).build()` |
| `Graph::new_with(roots, rt, mode, f, t)` | `Graph::builder().add(roots).tokio_runtime(rt).run_mode(mode).run_for(f).start_time(t).build()` |

Holding a `RunMode`/`RunFor` in a variable (parameterised tests, config-driven
runs) still works — that is what `run_mode(m)` and `run_for(f)` are for:

```rust
fn check(mode: RunMode, run_for: RunFor) {
    ticker(period).count().graph().run_mode(mode).run_for(run_for).run().unwrap();
}
```

An existing `vec![]` of roots does not need unpacking — `add` accepts it whole:

```rust
Graph::builder().add(vec![a, b]).historical().forever().run().unwrap();
```

### Notes and gotchas

- **Nothing is deprecated.**  `Graph::new`, `Graph::new_with`,
  `NodeOperators::run(mode, run_for)` and `into_graph` remain, now as thin
  wrappers over the builder.  Migrate opportunistically; mixed styles compile.
- **`AsUpstreamNodes` must be in scope** for `add`.  `use wingfoil::*` covers
  it; explicit importers need `use wingfoil::AsUpstreamNodes`.
- **Builder methods consume `self` and are `#[must_use]`.**  Chain them, or
  rebind — `let b = b.historical();` — but don't discard the return value.
- **`historical()` means `HistoricalFrom(NanoTime::ZERO)`**, the replay start
  used by most tests.  Use `historical_from(t)` for any other epoch.
- **Two different `print()`s.**  `stream.print()` is the node operator that
  prints values as they tick; `builder.print()` is the debugging aid that dumps
  the wired graph once it is built (the old `Graph::print`).  Both still exist
  and do the same things they always did.
- **`build()` instead of `run()`** when the `Graph` itself is needed — e.g. to
  hold it, inspect it, or call `export`.  `run()` is `build()` followed by
  `Graph::run`.
- **Python and TypeScript users are unaffected.**  `wingfoil-python` uses the
  builder internally; its public API is unchanged, as is `@wingfoil/client`.

### Checklist

1. `rg 'Graph::new|\.run\(RunMode' src/` to find call sites.
2. Rewrite each using the table above — single-root chains via `.graph()`,
   multi-root via `Graph::builder()`.
3. Drop the `let` bindings that existed only to name roots for `vec![]`.
4. Drop `.real_time()` / `.forever()` where they were the only configuration —
   they are the defaults.
5. `cargo fmt --all && cargo lint && cargo test`.
