# Where Wingfoil fits

Wingfoil is not the only way to build a graph of calculations over event
streams, and it is not the right way for every job. This page is an
orientation, not a scorecard: what the neighbouring projects are good at, and
when you should use one of them instead.


## Don't use Wingfoil if

- **You want a trading *system*, not a compute engine.** Orders, positions,
  portfolio, risk and venue connectivity as first-class domain objects —
  [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) gives you
  all of that out of the box. Wingfoil gives you a graph; the domain model is
  yours to write.
- **You are Python-first and staying that way.**
  [csp](https://github.com/Point72/csp) has years of production mileage and
  genuinely excellent Python ergonomics. If nothing in your stack wants a
  native API, csp's is the better-trodden path.
- **You want distributed SQL over Kafka.**
  [Arroyo](https://github.com/ArroyoSystems/arroyo),
  [RisingWave](https://github.com/risingwavelabs/risingwave) and Flink solve a
  different problem — horizontally scaled streaming over a message broker, with
  SQL as the interface. Wingfoil is a single-process engine.
- **Your strategy is expressible as array operations.** Vectorised backtesting
  (VectorBT and friends) will be faster to write and faster to run. Wingfoil is
  event-driven; reach for it when execution mechanics, ordering and per-event
  state are what you are modelling.
- **You need incremental view maintenance over relations.**
  [Feldera](https://github.com/feldera/feldera) and
  [Materialize](https://github.com/MaterializeInc/materialize) propagate
  changes proportional to the size of the change, not the data. That is a
  different machine that happens to share a diagram.

Wingfoil is for a graph of stateful calculations over event streams, where
per-event latency matters, which must run identically over history and live, in
a process you would rather not put an interpreter in.


## Two axes that actually discriminate

Feature tables mostly measure how long a project has existed. These two axes
separate the designs:

| Project | Async on the hot path? | Native-language API? |
|---|---|---|
| **Wingfoil** | No — core is executor-free; tokio behind the `async` feature at graph edges | Rust first; Python and TypeScript are bindings on top |
| **NautilusTrader** | No — deterministic single-threaded core; tokio for network I/O, asyncio/uvloop for live | Rust API exists and is growing; Python is the primary documented surface |
| **Barter** | Yes — tokio-native throughout, one thread per trader instance | Rust only |
| **csp** | No — dedicated engine thread; push adapters feed it from their own threads | Python only (see below) |
| **Timely / Differential Dataflow** | No — own scheduler and worker threads | Rust only |
| **Feldera (DBSP)** | No — own scheduler | Rust, with SQL as the primary surface |
| **Arroyo** | Yes (tokio) | SQL first |
| **Bytewax** | No — timely workers underneath | Python only |

**Async on the hot path** matters because futures executors trade tail latency
and determinism for throughput and ergonomics. Almost everyone uses tokio for
sockets; the question is what sits behind it. Wingfoil's core is
executor-free — async lives at the edges, behind the `async` feature, and never
on a `cycle`.

**A native-language API** matters because it decides whether your fast path can
avoid a language boundary at all. This is the axis where the field is more
lopsided than it looks — see csp below.


## The closest neighbours

### Point72 csp — closest in concept

csp is Wingfoil's twin in design: a reactive DAG, switchable
simulation/realtime timesteps, adapters at the boundary, write it once and
deploy it live. The README pitch is nearly the same sentence as ours, and we
consider that a compliment — csp got there first and got it right.

Three real differences:

- **csp has no non-Python way to build a graph.** The README's "written in C++
  and Python" describes the implementation, not the API. `@csp.node` works by
  reading your function's source with `inspect.getsource()` and rewriting the
  Python AST; even a C++ node is a CPython extension module attached to a
  Python declaration via `@csp.node(cppimpl=...)`, which owns the signature.
  The engine does carry an internal *dialect* abstraction and builds as a
  static library that doesn't link Python — but Python is the only dialect that
  exists, and generating the C++ type headers runs a Python module at build
  time. Wingfoil inverts this: Rust is the primary API and the bindings sit on
  top, so a Rust user crosses no boundary and ships no interpreter.
- **csp has no compiled tier.** Its answer to "make this node fast" is "write
  it in C++" — a second language, a second build system and a duplicated
  signature per node. Wingfoil's `#[op]` produces interpreted *and* compiled
  coverage from one definition.
- **csp is more mature.** Broader adapter coverage, `csp-gateway` for the
  service layer, and Point72 behind it.

### NautilusTrader — closest in audience

Nautilus targets the same people: production trading, backtest-live parity,
many venues. Architecturally it is closer to Wingfoil than Barter is — a
single-threaded deterministic core processing events sequentially, with I/O
pushed out to separate threads and runtimes, and horizontal scaling as the
answer to load.

Where it differs: it is an application framework rather than a general compute
graph, so the trading domain model comes included and the graph model does not.
Its adapter and venue coverage is far ahead of ours. If you want to be trading
next month, use Nautilus.

**In practice it is used from Python.** A Rust API exists and is growing, but at
the time of writing `nautilus-trader` draws roughly 283k PyPI downloads a month
against roughly 7.5k crates.io downloads in 90 days for `nautilus-core` — about
two orders of magnitude, and the Rust figure is the generous reading, since
`nautilus-core` is a dependency of every other `nautilus-*` crate and so counts
internal resolution and CI as well as humans. That is a statement about how the
project is used, not about who uses it or how good it is: the engineering is
serious, and firms do run it. But if your reason for choosing a Rust-native
engine is avoiding an interpreter in the process, that is the surface almost
everyone is actually on.

On performance, see the two measurements below. The short version:
**interpreted Wingfoil is not faster than Nautilus — on ingest the two are
indistinguishable, and on fan-out theirs is 2.3× ahead per consumer.** Wingfoil
wins only through the compiled tier, by ~3× on ingest and 1.4× on fan-out slope.
Anyone who tells you the margin is larger than that has not measured it.

### Barter — the async-first counterpoint

Barter is the one project here that is genuinely async on the hot path:
tokio-native, `Strategy` and `RiskManager` as plugin traits, one thread per
trader instance, and a data-oriented state store with O(1) index lookups. It
leans into running thousands of concurrent backtests.

It is a different philosophy rather than a competing implementation of ours —
no DAG, no per-node engine overhead to quote, no execution tiers. If you want a
Rust trading engine and the graph model is not what you came for, Barter is a
good answer.


## The wider field

- **Deephaven** — JVM; incremental real-time tables. Same update-graph idea,
  table surface instead of streams.
- **KX / kdb+ tick** — the incumbent much of this field is migrating off.
  Wingfoil ships a KDB+ adapter for exactly that reason.
- **Bank dependency graphs** — Goldman's SecDB, JPMorgan's Athena (including
  Reactive Athena), Bank of America's Quartz, and Beacon commercially. Mostly
  pull-based, memoise-and-invalidate designs built for scenarios and greeks,
  where Wingfoil and csp are push-based event streaming. Culturally this is
  where the idiom comes from.
- **Tributary, Streamz, Faust, Quix Streams** — the Python-native reactive
  tier. Much easier, much slower.
- **hftbacktest** — tick-level backtesting with queue-position models. Not an
  engine; a very good backtester.


## Honest about maturity

csp and NautilusTrader have both been running in production for years and have
substantially broader adapter and venue coverage than we do. Wingfoil still has
the [legacy cutover](cutover-plan.md) ahead of it. If breadth of connectivity
and production mileage are what you are buying, they are further along, and
that gap is not going to close this quarter.

What Wingfoil has that neither does is a Rust-native core with no FFI tax and a
compiled tier derived from the same wiring as the interpreted one. If that is
what you need, the youth of the project is the price.


## Measured: one market-data event, end to end

One comparison, run back to back on the same machine. Read the caveats — they
change what the numbers mean.

The Nautilus side is **their own benchmark, unmodified**:
`crates/data/benches/engine.rs`, which measures
`DataEngine::process_data(Data::Trade(..))` — engine dispatch, then
`Cache::add_trade`, then `msgbus::publish_trade`. We did not choose their
workload or write their harness. The Wingfoil side is
[`benches/vs_nautilus.rs`](../crates/wingfoil/benches/vs_nautilus.rs), a graph
whose terminal node writes into a `HashMap<u64, VecDeque<Trade>>` bounded the
same way their cache is.

Two independent runs, ns/event:

| | run 1 | run 2 |
|---|---|---|
| Nautilus `DataEngine::process_data` | 149.0 | 158.3 |
| Wingfoil interpreted, engine + cache write | 156.3 | 150.8 |
| Wingfoil **compiled**, engine + cache write | **54.1** | **50.4** |
| *Nautilus `Cache::add_trade` alone* | *20.5* | *20.1* |
| *Wingfoil interpreted, engine only (no cache)* | *99.1* | *99.7* |

**Interpreted Wingfoil and Nautilus are indistinguishable on this workload.**
Run 1 put Nautilus 5% ahead; run 2 put Wingfoil 5% ahead. The ordering flips
between runs, so neither side gets to claim it — we report both runs rather than
the one that suits us. The compiled tier runs the same workload in 50–54 ns,
roughly 3× faster than Nautilus, and that is the only speed claim this page
makes about ingestion.

Five things that shape those numbers, in both directions:

- **Wingfoil's arm carries engine machinery theirs does not.** Our figure
  includes a ticker source, a `count` node and a `TimeQueue` re-arm on every
  cycle; Nautilus's is a `b.iter()` loop calling one method. That handicaps us,
  so the interpreted tie flatters us and the compiled win is understated.
- **Nautilus's path buys features ours does not have.** Of their 149 ns, 20.5 ns
  is the cache write and most of the rest is engine dispatch plus a msgbus
  publish — *with zero subscribers attached*. That is what runtime-subscribable,
  topic-addressed pub/sub and a queryable cache cost per event. Wingfoil's edges
  are resolved at wiring time, so it cannot pay that cost and cannot offer that
  capability.
- **This is one workload.** Trade ingestion into a cache. It is not a strategy,
  an order path, or a backtest. Do not generalise it.
- **The machine is a 4-core cloud sandbox**, not a tuned benchmark host.
  Nautilus's own `BENCHMARKING.md` says local numbers should not be quoted as
  authoritative, and that applies equally to ours. The ratio is more durable
  than the absolutes, and even the ratio moves with cache and microarchitecture.
- **Both sides were run back to back on the same machine**, which is the only
  reason the comparison means anything at all. Every figure on this page comes
  from a strictly sequential run with nothing else building or benchmarking —
  an earlier pass overlapped two benchmarks on a 4-core box and moved the ingest
  absolutes by 6%, enough to flip which side led. That is why two runs are
  reported rather than one, and why the fan-out section quotes slopes.

## Measured: fan-out, one event to N consumers

The second comparison, and the one that went against our expectation. Again
their own unmodified bench — `bench_router_multiple_subscribers` in
`crates/common/benches/msgbus.rs`, sweeping subscriber count over `[1, 5, 10]`
— against [`benches/vs_nautilus_fanout.rs`](../crates/wingfoil/benches/vs_nautilus_fanout.rs).
Per-consumer work is identical on both sides: the same
`AtomicU64::fetch_add(.., Relaxed)` on a static that their handler uses.

The quantity to read is the **slope** — marginal cost of one more consumer,
`(t(10) - t(1)) / 9`. The intercepts are not comparable, because their
`b.iter()` calls `router.publish` directly while our graph carries a ticker, a
`count` and a `TimeQueue` re-arm inside the measurement. Fixed cost cancels out
of a difference; it does not cancel out of an absolute.

| | ns per additional consumer | run 2 |
|---|---|---|
| Nautilus `Any`-based router | 7.52 | 7.45 |
| Nautilus typed router | 7.58 | 7.58 |
| Wingfoil interpreted | 17.45 | 17.54 |
| Wingfoil **compiled** | **5.53** | **5.52** |

Slopes reproduce across runs to within 1%, which is the point of reading them
instead of the absolutes — whatever varies in the fixed cost cancels out.

**We expected the gap to widen here and it narrowed.** Compiled is ~3× on the
single-consumer ingest workload above, but only 1.4× on fan-out slope; the
interpreted tier is 2.3× *slower* per consumer than their message bus.

Fan-out is where their design is strongest, and the numbers show why: the router
resolves the topic once and then walks a subscriber vector, so routing amortises
across consumers. Their `Any`-based router does a `downcast_ref` on every
delivery and still matches their typed router to within 1%, which means the
per-subscriber path has essentially nothing left in it.

One caveat on all four figures: the shared atomic sits inside every slope. If it
costs on the order of 5 ns, the dispatch components are nearer 2.5 ns
(Nautilus), 0.5 ns (compiled) and 12.5 ns (interpreted) — so the true dispatch
ratios are wider than the table in both directions. That is arithmetic from an
unmeasured constant, not a result, and it is why the table reports what was
measured instead.

## A note on the numbers

The other figures in our [README](../README.md) — around 27 ns of engine
overhead per node cycle, and compiled running 4.4×–37× faster — are Wingfoil
measured against **itself** on our own [benchmarks](../crates/wingfoil/benches/),
and are not comparisons against anything on this page.

If you rerun any of this and get something different, please tell us.


---

*Assessed August 2026, against csp 0.18.0, nautilus_trader 1.231.0 and barter
0.12.5. Download figures from crates.io and PyPI as of that date. Comparisons go stale faster than anything else we write — if we have
described your project wrongly or unfairly, please open an issue or a PR and we
will fix it.*
