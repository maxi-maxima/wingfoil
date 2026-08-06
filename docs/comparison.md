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
There is no compiled-graph tier. Its adapter and venue coverage is far ahead of
ours. If you want to be trading next month, use Nautilus.

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


## A note on the numbers

The figures in our [README](../README.md) — around 27 ns of engine overhead per
node cycle, and compiled running 4.4×–37× faster — are Wingfoil measured
against **itself**, on our own [benchmarks](../crates/wingfoil/benches/). They
are not head-to-head comparisons against anything on this page, and should not
be read as such. We have not run cross-project benchmarks; if you do, we would
like to see them.


---

*Assessed August 2026, against csp 0.18.0, nautilus_trader 1.231.0 and barter
0.12.5. Comparisons go stale faster than anything else we write — if we have
described your project wrongly or unfairly, please open an issue or a PR and we
will fix it.*
