# Stream processing, dataflow and trading frameworks: a comparison

A survey of the frameworks people actually choose when they need a graph of
calculations over event streams — reactive DAG engines, trading engines and
backtesters, distributed stream processors, and the dataflow substrates several
of them are built on.

> **Who wrote this.** We build [Wingfoil](https://github.com/wingfoil-io/wingfoil),
> which is one of the rows. Two of the projects here beat it on the only
> benchmarks we ran, and the page says so with numbers.
> [Corrections welcome](#corrections).

[Reactive / DAG engines](#reactive--dag-compute-engines) ·
[Trading engines and backtesters](#trading-engines-and-backtesters) ·
[Distributed stream processors](#distributed-stream-processors) ·
[Dataflow substrates](#dataflow-and-incremental-substrates) ·
[Which should I use?](#which-should-i-use) ·
[Benchmarks](#measured-wingfoil-vs-nautilustrader)

**On the Performance column:** cells marked ⬥ we
[measured](#measured-wingfoil-vs-nautilustrader) back to back on one machine.
Every other cell is the project's own claim or unmeasured, and says so. **It is
not a ranking** — these rows do not do the same work.

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

**[csp](https://github.com/Point72/csp)** is Wingfoil's twin in design, and got
there first. One structural difference: it has **no non-Python way to build a
graph**. `@csp.node` reads your function's source with `inspect.getsource()` and
rewrites the Python AST; a C++ node is a CPython extension attached to a Python
declaration via `cppimpl=`, which owns the signature. The engine carries an
internal *dialect* abstraction and links no Python — but Python is the only
dialect that exists, and generating its C++ headers runs a Python module at
build time.

**[NautilusTrader](https://github.com/nautechsystems/nautilus_trader)** is closest
in audience, and architecturally close to us: their docs describe a
"single-threaded core [that] provides deterministic event ordering and helps
maintain backtest-live parity", the `MessageBus` is `thread_local!`, and adapter
I/O sits on a separate tokio runtime — the same executor-free-core shape,
arrived at independently. The difference is framework versus library. Everything
entering the engine is an `enum Data { Delta, Quote, Trade, Bar, … }`, a closed
list of their concepts; your own type rides as `Custom(CustomData)` —
`Arc<dyn CustomDataTrait>` routed by a string — against Wingfoil's `Stream<T>` in
your type, statically, resolved at wiring time. And it is used from Python:
~283k PyPI downloads a month against ~7.5k crates.io downloads in 90 days for
`nautilus-core`, the latter generous since every other `nautilus-*` crate depends
on it. That is about which surface people are on, not who they are.

**[Barter](https://github.com/barter-rs/barter-rs)** is the one project here
genuinely async on the hot path: tokio-native, `Strategy` and `RiskManager` as
plugin traits, one thread per trader instance, thousands of concurrent
backtests. A different philosophy, not a competing implementation — no DAG, no
execution tiers.

**Maturity, plainly:** csp and NautilusTrader have run in production for years
with far broader adapter and venue coverage; Wingfoil still has its
[legacy cutover](cutover-plan.md) ahead. If breadth and mileage are what you are
buying, they are further along. What Wingfoil has that neither does is a
Rust-native core with no FFI tax and a compiled tier derived from the same
wiring as the interpreted one.


## Measured: Wingfoil vs NautilusTrader

The only head-to-head numbers on this page. Both sides use **NautilusTrader's own
unmodified benchmarks** — `nautilus-data --bench engine` and
`nautilus-common --bench msgbus` — against Wingfoil graphs matched to the same
work, run back to back on one 4-core machine. Method and every caveat are in the
harnesses: [`vs_nautilus.rs`](../crates/wingfoil/benches/vs_nautilus.rs) and
[`vs_nautilus_fanout.rs`](../crates/wingfoil/benches/vs_nautilus_fanout.rs).

**Ingest — one trade event into a cache**, ns/event, two independent runs:

| | run 1 | run 2 |
|---|---|---|
| Nautilus `DataEngine::process_data` | 149.0 | 158.3 |
| Wingfoil interpreted | 156.3 | 150.8 |
| Wingfoil **compiled** | **54.1** | **50.4** |

**Fan-out — marginal cost of one more consumer**, ns, `(t(10) − t(1))/9`:

| | run 1 | run 2 |
|---|---|---|
| Nautilus typed router | 7.58 | 7.58 |
| Nautilus `Any`-based router | 7.52 | 7.45 |
| Wingfoil interpreted | 17.45 | 17.54 |
| Wingfoil **compiled** | **5.53** | **5.52** |

**What this says.** Interpreted Wingfoil is **not faster than Nautilus**: on
ingest the two are indistinguishable and the ordering flips between runs; on
fan-out theirs is 2.3× ahead per consumer, because their router resolves the
topic once and then walks a subscriber vector. Wingfoil wins only through the
compiled tier — ~3× on ingest, 1.4× on fan-out slope. We expected fan-out to
widen the gap and it narrowed it.

Four things that shape the numbers, in both directions:

- **Our arm carries machinery theirs does not** — a ticker, a `count` node and a
  `TimeQueue` re-arm every cycle, against their `b.iter()` calling one method. The
  interpreted tie flatters us; the compiled win is understated.
- **Their path buys capability we lack.** Of 149 ns, 20.5 ns is the cache write
  and most of the rest is dispatch plus a msgbus publish *with no subscribers* —
  the price of runtime-subscribable, topic-addressed pub/sub and a queryable
  cache. Wingfoil resolves edges at wiring time, so it cannot pay that cost or
  offer that capability.
- **Read slopes, not absolutes.** Fan-out slopes reproduce within 1% across runs;
  ingest absolutes moved 6% and flipped the lead. Two runs are reported for that
  reason.
- **One workload, a cloud sandbox.** Nautilus's own `BENCHMARKING.md` says local
  figures are not authoritative. That applies to ours.

The README's other figures — ~27 ns/node-cycle, compiled 4.4×–37× — are Wingfoil
measured against **itself**, not against anything on this page.


## The same example, both frameworks

Numbers say how fast; this says how different. We ported
[`examples/core/order_book`](../crates/wingfoil/examples/core/order_book/) — a
LOBSTER file replayed into a book — onto a Nautilus `BacktestEngine` with a
`DataActor` over `L3_MBO` deltas. Both run against the identical 91,997-row file.
The port is [runnable](comparisons/lobster-nautilus/): `cargo run --release`, no
clone of their repository needed.

| | Wingfoil | Nautilus |
|---|---|---|
| Top-of-book prices | 15,040 | 16,161 |
| Fills | 4,169 | *not expressible* |
| Non-comment lines | 154 | 202 |

Wingfoil's 154 lines produce **both** outputs; the port's 202 produce only prices.
Three findings, and only the first is about verbosity:

- **Same-instant grouping is a guarantee here and absent there.** Instrumenting
  the actor gives **89,712 batches for 89,712 deltas — mean batch 1.00**. Every
  delta arrives alone, so top of book is sampled *mid-instant* where a
  [`Burst`](https://docs.rs/wingfoil/latest/wingfoil/struct.Burst.html) would have
  grouped it. That is the entire 7% price-count difference. `RecordFlag::F_LAST`
  did not change it for an unmanaged subscription; we did not exhaust their
  configuration space.
- **The fills have nowhere to come from.** Wingfoil's book *matches* —
  `lobster::OrderBook::execute` returns fill metadata. Nautilus's `OrderBook`
  applies deltas and returns `Result<(), BookIntegrityError>`; matching lives in
  `OrderMatchingEngine`, which matches *your* orders rather than reconstructing
  the tape's. The wrangler must track every resting order's size itself.
- **A trading context is mandatory.** The port needs a venue, an OMS type, an
  account type, starting balances and a full instrument for a program that never
  trades — and prints a Sharpe ratio regardless. That is the framework being an
  application framework, and it is exactly what you want when you *are* trading.

The full write-up, including the silent bug that made a first attempt emit 121
prices instead of 16,161, is in the
[port's README](comparisons/lobster-nautilus/README.md).


## Corrections

Assessed **August 2026**, against csp 0.18.0, nautilus_trader 1.231.0 and barter
0.12.5; download figures from crates.io and PyPI as of that date.

A page like this is wrong somewhere the day it is published. **If we have
described your project inaccurately or unfairly — or you maintain one we have
missed — please open an issue or a pull request** on
[wingfoil-io/wingfoil](https://github.com/wingfoil-io/wingfoil/issues).
Maintainers get the benefit of the doubt: if you tell us a cell is wrong about
your own project, we will change it.
