# Where Wingfoil fits

Wingfoil is not the only way to build a graph of calculations over event
streams, and it is not the right way for every job. This page is an orientation,
not a scorecard: what the neighbouring projects are for, and when you should use
one of them instead.


## The landscape

| Project | Core language | User language | Performance | Primary use cases | Pro | Con |
|---|---|---|---|---|---|---|
| **Wingfoil** | Rust | **Rust** first; Python, TypeScript | ~27 ns/node-cycle; compiled ~3× Nautilus on ingest, 1.4× on fan-out slope ⬥ | Latency-critical compute graphs over event streams; backtest then live unchanged | Native API, no interpreter in-process; one wiring runs interpreted *or* compiled | Youngest here; no trading domain model; adapter breadth behind csp and Nautilus |
| **csp** | C++ | **Python only** | Not measured here. C++ engine, but node bodies run in Python unless hand-written in C++ | Reactive DAGs, research → production, in Python shops | Mature, production-proven, excellent Python ergonomics, sim/realtime parity | No non-Python way to build a graph; no compiled tier; interpreter always in-process |
| **NautilusTrader** | Rust | **Python** in practice; Rust API growing | 149–158 ns/event ingest; 7.5 ns per extra subscriber ⬥ | Complete trading systems: venues, orders, portfolio, risk | Batteries-included trading domain; broad venue coverage; deterministic core | Closed `Data` ontology — custom types go behind `Arc<dyn Trait>` routed by string; a venue and account must exist to compute anything |
| **Barter** | Rust | Rust | Not measured | Event-driven live, paper and backtest trading engines | tokio-native; thousands of concurrent backtests; O(1) state lookups | Async on the hot path; no graph model; no execution tiers |
| **Feldera (DBSP)** | Rust | SQL, Rust | Not measured. Incremental: cost tracks the size of the change | Incremental view maintenance over relations | Work proportional to the delta, not the dataset | Relational, not event-stream ops — a different problem that shares a diagram |
| **Timely / Differential Dataflow** | Rust | Rust | Not measured | Distributed dataflow with progress tracking | One program scales from a laptop to a cluster | Low-level; no domain model; steep learning curve |
| **Arroyo** | Rust | SQL, Rust | Not measured. Scales to millions of events/sec across a cluster | Distributed stream processing over Kafka | Serverless operations, SQL-first, checkpointed state | Cluster-shaped; not a single-process low-latency engine |
| **Bytewax** | Rust (timely) | **Python only** | Not measured. ~25× less memory than a comparable Flink cluster (their figure) | Python-native dataflow pipelines | Full Python ecosystem with code-level control | Python throughput ceiling; no compiled tier |
| **Deephaven** | Java / C++ | Python, Java, Groovy | Not measured | Live incremental tables, real-time analytics | Table semantics over streams; strong notebook and UI story | JVM; a table surface rather than stream combinators |
| **kdb+ / q** | C | q | Not measured. The long-standing bar for tick analytics | Tick capture and timeseries analytics | Unmatched columnar timeseries speed; decades of production | Proprietary and expensive; q is a niche language; Wingfoil ships a KDB+ adapter partly so you can migrate off it |
| **hftbacktest** | Rust | Python, Rust | Not measured | Tick-level backtesting with queue-position models | Models queue position and latency honestly | A backtester, not an engine — no live path |

⬥ **Measured on the same machine, back to back** — method, both runs and every
caveat are in [Measured performance](#measured-one-market-data-event-end-to-end)
below. Every other cell in that column is the project's own claim or is
unmeasured, and is marked as such. Do not read the column as a ranking: the rows
do not do the same work.

**Not in the table:** the bank dependency graphs — Goldman's SecDB, JPMorgan's
Athena (including Reactive Athena), Bank of America's Quartz, and Beacon
commercially. Mostly pull-based, memoise-and-invalidate designs built for
scenarios and greeks, where Wingfoil and csp are push-based event streaming.
Culturally this is where the whole idiom comes from. Also omitted: Tributary,
Streamz, Faust and Quix Streams — the Python-native reactive tier, much easier
and much slower.


## Don't use Wingfoil if

- **You want a trading *system*, not a compute engine.** Orders, positions,
  portfolio, risk and venue connectivity as first-class objects —
  [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) has all of
  it. Wingfoil gives you a graph; the domain model is yours to write.
- **You are Python-first and staying that way.**
  [csp](https://github.com/Point72/csp) has years of production mileage and
  genuinely excellent ergonomics. If nothing in your stack wants a native API,
  csp is the better-trodden path.
- **You want distributed SQL over Kafka.** [Arroyo](https://github.com/ArroyoSystems/arroyo),
  [RisingWave](https://github.com/risingwavelabs/risingwave) and Flink solve a
  different problem. Wingfoil is a single-process engine.
- **Your strategy is expressible as array operations.** Vectorised backtesting
  will be faster to write and to run. Reach for Wingfoil when execution
  mechanics, ordering and per-event state are what you are modelling.
- **You need incremental view maintenance over relations.**
  [Feldera](https://github.com/feldera/feldera) and
  [Materialize](https://github.com/MaterializeInc/materialize) propagate changes
  proportional to the size of the change.

Wingfoil is for a graph of stateful calculations over event streams, where
per-event latency matters, which must run identically over history and live, in
a process you would rather not put an interpreter in.


## The three closest, in one paragraph each

**csp** is Wingfoil's twin in design — reactive DAG, switchable
simulation/realtime, adapters at the boundary — and it got there first. The one
structural difference: it has **no non-Python way to build a graph**. `@csp.node`
reads your function's source with `inspect.getsource()` and rewrites the Python
AST; even a C++ node is a CPython extension attached to a Python declaration via
`@csp.node(cppimpl=...)`, which owns the signature. The engine carries an
internal *dialect* abstraction and links no Python, but Python is the only
dialect that exists, and generating its C++ type headers runs a Python module at
build time.

**NautilusTrader** is closest in audience, and architecturally closer to
Wingfoil than Barter is: their docs describe a "single-threaded core [that]
provides deterministic event ordering and helps maintain backtest-live parity",
the `MessageBus` is `thread_local!` in the source, and adapter I/O sits on a
separate multi-threaded tokio runtime — the same executor-free-core shape as
ours, arrived at independently. The difference is that it is a *framework* where
Wingfoil is a *library*: everything entering the engine is an
`enum Data { Delta, Quote, Trade, Bar, … }`, a closed list of their concepts.
Your own type goes on the bus as `Custom(CustomData)` — `Arc<dyn CustomDataTrait>`
routed by a string `DataType` — against Wingfoil's `Stream<T>` in your own type,
statically, with edges resolved at wiring time. In practice it is used from
Python: roughly 283k PyPI downloads a month against ~7.5k crates.io downloads in
90 days for `nautilus-core`, and the Rust figure is generous, since that crate
is a dependency of every other `nautilus-*` one. That is a statement about which
surface people are on, not about who they are or how good it is.

**Barter** is the one project here genuinely async on the hot path: tokio-native,
`Strategy` and `RiskManager` as plugin traits, one thread per trader instance, a
data-oriented state store with O(1) lookups, and a focus on running thousands of
concurrent backtests. A different philosophy rather than a competing
implementation of ours — no DAG, no execution tiers.


## Honest about maturity

csp and NautilusTrader have both run in production for years and have
substantially broader adapter and venue coverage than we do. Wingfoil still has
the [legacy cutover](cutover-plan.md) ahead of it. If breadth of connectivity and
production mileage are what you are buying, they are further along, and that gap
is not closing this quarter.

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

## The same example, both frameworks

Numbers say how fast; this says how different. We ported
[`examples/core/order_book`](../crates/wingfoil/examples/core/order_book/) — a
LOBSTER message file replayed into an order book, emitting top-of-book prices
and fills — onto NautilusTrader, and ran both against the identical 91,997-row
file. Both versions compile and run; the Nautilus port is a `BacktestEngine`
with a `DataActor` subscribing to `L3_MBO` book deltas, and it lives in
[`docs/comparisons/lobster-nautilus/`](comparisons/lobster-nautilus/) so you can
rerun it — `cargo run --release` in that directory, no clone of their repository
needed.

| | Wingfoil | Nautilus |
|---|---|---|
| Top-of-book prices | 15,040 | 16,161 |
| Fills | 4,169 | *not expressible* |
| Non-comment lines | 154 | 202 |

The line counts understate it: Wingfoil's 154 produce **both** outputs, Nautilus's
202 produce only the prices.

Three differences, and only the first is about verbosity.

**Same-instant grouping is a guarantee here and absent there.** Wingfoil delivers
same-instant messages as one [`Burst`], so the book reaches a consistent state
before its top is read. Porting that faithfully turned out to be the hard part:
in the Nautilus actor we instrumented the delivered batches and measured
**89,712 batches for 89,712 deltas — a mean batch size of 1.00**. Every delta
arrives alone, so top of book is sampled *mid-instant*, which is the whole of the
7% price-count difference. Setting `RecordFlag::F_LAST` on the last delta of each
instant did not change it for an unmanaged subscription; there may be a
configuration or data-client path that does batch, and we did not exhaust their
options. But a straightforward port silently samples a book that is halfway
through an update, and nothing warns you.

**The fills have nowhere to come from.** Wingfoil's book *matches* — 
`lobster::OrderBook::execute` returns fill metadata, so the fills output falls
out of applying a message. Nautilus's `OrderBook` applies deltas and returns
`Result<(), BookIntegrityError>`; matching lives in `OrderMatchingEngine`, which
matches *your* orders against the book rather than reconstructing the tape's own
executions. So the wrangler has to do the size bookkeeping itself — track every
resting order, decrement on execution, emit Update or Delete — and the fills
would have to come from re-reading the raw messages. That is not a defect; it is
a different decomposition. But it means the shape "one node maintains the book
and emits both outputs, `split()` them" has no counterpart.

**You declare a trading context to do a non-trading computation.** The port needs
a venue, an OMS type, an account type, starting balances and a fully specified
instrument, for an example that never places an order — and it prints a Sharpe
ratio, a Sortino ratio and a PnL summary at the end regardless. That is the
framework being an application framework: the domain model is the product, and
you pay for it whether or not you use it.

The honest summary is that Wingfoil's version reads as an expression — source,
transform, split, two sinks — and Nautilus's reads as a lifecycle: register,
subscribe, mutate state in a callback, flush on stop. Ours is more direct for
this task. Theirs would be far more direct for placing an order against that
book, which ours does not model at all.

[`Burst`]: https://docs.rs/wingfoil/latest/wingfoil/struct.Burst.html

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
