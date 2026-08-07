# Stream processing, dataflow and trading frameworks: a comparison

A curated survey of the frameworks people actually choose when they need a graph
of calculations over event streams — reactive DAG engines, trading engines and
backtesters, distributed stream processors, and the dataflow substrates
underneath several of them.

> **Who wrote this.** We build [Wingfoil](https://github.com/wingfoil-io/wingfoil),
> which is one of the rows. We have tried to write the page we wanted when we
> started and could not find: comprehensive, specific about trade-offs, and
> explicit about where Wingfoil loses. Two of the projects here beat Wingfoil on
> the only benchmarks we ran, and the page says so with numbers. Corrections are
> welcome — see [the end](#corrections).

**Contents** — [Reactive / DAG compute engines](#reactive--dag-compute-engines) ·
[Trading engines and backtesters](#trading-engines-and-backtesters) ·
[Distributed stream processors](#distributed-stream-processors) ·
[Dataflow and incremental substrates](#dataflow-and-incremental-substrates) ·
[Which should I use?](#which-should-i-use) ·
[Measured performance](#measured-one-market-data-event-end-to-end)

A note on the **Performance** column throughout: cells marked ⬥ were measured by
us, back to back on one machine, with the method and caveats
[below](#measured-one-market-data-event-end-to-end). Everything else is the
project's own published claim or is unmeasured, and is labelled. **The column is
not a ranking** — these rows do not do the same work, and several are not
comparable even in principle.


## Reactive / DAG compute engines

Graphs of stateful nodes, pushed through by events. Write the logic once, replay
it over history, run it live.

| Project | Core language | User language | Performance | Primary use cases | Pro | Con |
|---|---|---|---|---|---|---|
| [**Wingfoil**](https://github.com/wingfoil-io/wingfoil) | Rust | **Rust** first; Python, TypeScript | ~27 ns/node-cycle; compiled ~3× Nautilus on ingest, 1.4× on fan-out slope ⬥ | Latency-critical compute graphs; backtest then live unchanged | Native API, no interpreter in-process; one wiring runs interpreted *or* compiled; per-hop latency tracing | Youngest here; no trading domain model; adapter breadth behind csp and Nautilus |
| [**csp**](https://github.com/Point72/csp) (Point72) | C++ | **Python only** | Unmeasured. C++ engine, but node bodies run in Python unless hand-written in C++ | Reactive DAGs, research → production, in Python shops | Mature and production-proven; excellent ergonomics; sim/realtime parity; `csp-gateway` for services | No non-Python way to build a graph; no compiled tier; interpreter always in-process |
| [**Deephaven**](https://github.com/deephaven/deephaven-core) | Java / C++ | Python, Java, Groovy | Unmeasured | Live incremental tables; real-time analytics and dashboards | Table semantics over streams; strong notebook and UI story | JVM; a table surface rather than stream combinators |
| [**Tributary**](https://github.com/streamlet-dev/tributary) | Python | Python | Unmeasured; pure Python | Small reactive pipelines, glue, prototyping | Very easy; no build step | Python throughput; not for latency-critical work |
| [**Streamz**](https://github.com/python-streamz/streamz) | Python | Python | Unmeasured; pure Python | Pipelines over Pandas/Dask | Integrates with the PyData stack | Largely dormant; no real-time guarantees |

## Trading engines and backtesters

These bring a domain model — instruments, orders, positions, venues — rather
than a general compute graph.

| Project | Core language | User language | Performance | Primary use cases | Pro | Con |
|---|---|---|---|---|---|---|
| [**NautilusTrader**](https://github.com/nautechsystems/nautilus_trader) | Rust | **Python** in practice; Rust API growing | 149–158 ns/event ingest; 7.5 ns per extra subscriber ⬥ | Complete trading systems: venues, orders, portfolio, risk | Batteries-included trading domain; broad venue coverage; deterministic single-threaded core; serious benchmarking culture | Closed `Data` ontology — your types ride as `Arc<dyn Trait>` routed by string; a venue and account must exist to compute anything |
| [**Barter**](https://github.com/barter-rs/barter-rs) | Rust | Rust | Unmeasured | Event-driven live, paper and backtest engines | tokio-native; thousands of concurrent backtests; O(1) state lookups | Async on the hot path; no graph model; no execution tiers |
| [**Lean**](https://github.com/QuantConnect/Lean) (QuantConnect) | C# | C#, Python | Unmeasured | Multi-asset research → live, with a hosted platform behind it | Huge data and broker coverage; cloud backtesting | .NET runtime; heavy; opinionated platform coupling |
| [**hftbacktest**](https://github.com/nkaz001/hftbacktest) | Rust | Python, Rust | Unmeasured | Tick-level backtesting with queue-position models | Models queue position and latency honestly — rare and hard | A backtester, not an engine; no live path |
| [**VectorBT**](https://github.com/polakowo/vectorbt) | Python (NumPy/Numba) | Python | Unmeasured; vectorised, very fast for what it does | Large-scale parameter sweeps and vectorised research | Extremely fast sweeps; excellent analytics | Not event-driven — execution mechanics and ordering are not modelled |
| [**Backtrader**](https://github.com/mementum/backtrader) | Python | Python | Unmeasured; pure Python | Teaching, prototyping, simple strategies | Gentle learning curve; large body of examples | Unmaintained; slow; no realistic live path |

## Distributed stream processors

Horizontally scaled, broker-backed, usually SQL-first. A different problem from
a single-process compute graph.

| Project | Core language | User language | Performance | Primary use cases | Pro | Con |
|---|---|---|---|---|---|---|
| [**Arroyo**](https://github.com/ArroyoSystems/arroyo) | Rust | SQL, Rust | Unmeasured; millions of events/sec across a cluster (their figure) | Distributed stream processing over Kafka | Serverless operations; SQL-first; checkpointed state | Cluster-shaped; not single-process low latency |
| [**RisingWave**](https://github.com/risingwavelabs/risingwave) | Rust | SQL | Unmeasured | Streaming database, materialised views over Kafka | Postgres-compatible surface; managed offering | A database, not an embeddable engine |
| [**Materialize**](https://github.com/MaterializeInc/materialize) | Rust (timely/differential) | SQL | Unmeasured | Incrementally maintained views over streams | Strong consistency story; mature incremental core | Cluster-shaped; SQL-only surface |
| [**Bytewax**](https://github.com/bytewax/bytewax) | Rust (timely) | **Python only** | Unmeasured; ~25× less memory than a comparable Flink cluster (their figure) | Python-native dataflow pipelines | Full Python ecosystem with code-level control | Python throughput ceiling; no compiled tier |
| [**Pathway**](https://github.com/pathwaycom/pathway) | Rust | Python | Unmeasured | Real-time ETL, RAG and AI pipelines | Unified batch/stream semantics; strong AI story | Younger; smaller community |
| [**Apache Flink**](https://github.com/apache/flink) | Java / Scala | SQL, Java, Python | Unmeasured; the industry reference at scale | Large-scale stateful stream processing | Enormous ecosystem; battle-tested | JVM; heavy operationally; high latency floor |
| [**Fluvio / SDF**](https://github.com/infinyon/fluvio) | Rust | SQL, WASM (Rust, Python) | Unmeasured | Edge-friendly streaming with programmable operators | Lightweight broker plus compute in one | Smaller ecosystem; WASM operator model is niche |
| [**Quix Streams**](https://github.com/quixio/quix-streams) / [**Faust**](https://github.com/robinhood/faust) | Python | Python | Unmeasured | Kafka stream processing from Python | Simple Kafka-native model | Python throughput; Faust is largely unmaintained |

## Dataflow and incremental substrates

Lower-level engines that several rows above are built on.

| Project | Core language | User language | Performance | Primary use cases | Pro | Con |
|---|---|---|---|---|---|---|
| [**Timely / Differential Dataflow**](https://github.com/TimelyDataflow/timely-dataflow) | Rust | Rust | Unmeasured | Distributed dataflow with progress tracking | One program scales laptop → cluster; the research is excellent | Low-level; no domain model; steep |
| [**Feldera (DBSP)**](https://github.com/feldera/feldera) | Rust | SQL, Rust | Unmeasured; work tracks the size of the change, not the dataset | Incremental view maintenance over relations | Genuinely incremental, with theory behind it | Relational, not event-stream ops — a different problem that shares a diagram |
| [**kdb+ / q**](https://kx.com) | C | q | Unmeasured; the long-standing bar for tick analytics | Tick capture and timeseries analytics | Unmatched columnar timeseries speed; decades of production | Proprietary and expensive; q is a niche language |

**Not in the tables:** the proprietary bank dependency graphs — Goldman's SecDB,
JPMorgan's Athena (including Reactive Athena), Bank of America's Quartz, and
[Beacon](https://www.beacon.io) commercially. Mostly pull-based,
memoise-and-invalidate designs built for scenarios and greeks, where the reactive
engines above are push-based event streaming. Culturally this is where the whole
idiom comes from, and a large share of the people building these systems learned
it inside one of them.


## Which should I use?

- **A trading system — orders, positions, venues, risk.** NautilusTrader, or Lean
  if you want a hosted platform. A general compute graph will cost you months
  rebuilding a domain model that already exists.
- **Backtesting realism at the microstructure level.** hftbacktest, for queue
  position; NautilusTrader for the full execution path.
- **Parameter sweeps over vectorisable signals.** VectorBT. If your strategy is
  array operations, event-driven is the wrong tool.
- **A reactive graph, and your team is Python.** csp. Mature, ergonomic, proven.
- **A reactive graph, and you want no interpreter in the process.** Wingfoil.
  That is the gap it exists to fill.
- **Streaming SQL over Kafka, horizontally scaled.** Arroyo, RisingWave,
  Materialize, or Flink if the ecosystem matters more than the latency floor.
- **Incrementally maintained views over relational data.** Feldera or
  Materialize.
- **Python-native dataflow at scale.** Bytewax or Pathway.

The axis that separates most of these is not speed. It is whether you want a
**framework** that supplies a domain model and calls your code, or a **library**
that supplies composition and lets you call it — and, if the latter, whether
your fast path can afford a language boundary.


## The three closest to Wingfoil

**csp** is Wingfoil's twin in design — reactive DAG, switchable
simulation/realtime, adapters at the boundary — and it got there first. The one
structural difference: it has **no non-Python way to build a graph**. `@csp.node`
reads your function's source with `inspect.getsource()` and rewrites the Python
AST; even a C++ node is a CPython extension attached to a Python declaration via
`@csp.node(cppimpl=...)`, which owns the signature. The engine carries an
internal *dialect* abstraction and links no Python, but Python is the only
dialect that exists, and generating its C++ type headers runs a Python module at
build time.

**NautilusTrader** is closest in audience, and architecturally closer to Wingfoil
than Barter is: their docs describe a "single-threaded core [that] provides
deterministic event ordering and helps maintain backtest-live parity", the
`MessageBus` is `thread_local!` in the source, and adapter I/O sits on a separate
multi-threaded tokio runtime — the same executor-free-core shape as ours, arrived
at independently. The difference is that it is a *framework* where Wingfoil is a
*library*: everything entering the engine is an
`enum Data { Delta, Quote, Trade, Bar, … }`, a closed list of their concepts.
Your own type goes on the bus as `Custom(CustomData)` — `Arc<dyn CustomDataTrait>`
routed by a string `DataType` — against Wingfoil's `Stream<T>` in your own type,
statically, with edges resolved at wiring time. In practice it is used from
Python: roughly 283k PyPI downloads a month against ~7.5k crates.io downloads in
90 days for `nautilus-core`, and the Rust figure is generous, since that crate is
a dependency of every other `nautilus-*` one. That is a statement about which
surface people are on, not about who they are or how good the project is.

**Barter** is the one project here genuinely async on the hot path: tokio-native,
`Strategy` and `RiskManager` as plugin traits, one thread per trader instance, a
data-oriented state store with O(1) lookups, and a focus on running thousands of
concurrent backtests. A different philosophy rather than a competing
implementation of ours — no DAG, no execution tiers.


## Honest about maturity

csp and NautilusTrader have both run in production for years and have
substantially broader adapter and venue coverage than Wingfoil does. Wingfoil
still has the [legacy cutover](cutover-plan.md) ahead of it. If breadth of
connectivity and production mileage are what you are buying, they are further
along, and that gap is not closing this quarter.

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

## Corrections

Assessed **August 2026**, against csp 0.18.0, nautilus_trader 1.231.0 and barter
0.12.5; download figures from crates.io and PyPI as of that date.

Comparisons go stale faster than anything else we write, and a page like this is
wrong somewhere the day it is published. **If we have described your project
inaccurately or unfairly — or you maintain one we have missed — please open an
issue or a pull request** on
[wingfoil-io/wingfoil](https://github.com/wingfoil-io/wingfoil/issues) and we
will fix it. Maintainers get the benefit of the doubt: if you tell us a cell is
wrong about your own project, we will change it.
