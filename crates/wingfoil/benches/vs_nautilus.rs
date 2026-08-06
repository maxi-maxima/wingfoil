//! Matched-workload comparison against NautilusTrader's `DataEngine` ingest bench.
//!
//! It produces the Wingfoil half of the table in `docs/comparison.md`. **Read
//! that page's caveats before quoting anything from here** — in particular,
//! this graph carries a ticker source and a `TimeQueue` re-arm per cycle that
//! the Nautilus harness has no equivalent of.
//!
//! Nothing here depends on `nautilus_trader`. Their half is produced by running
//! their own unmodified bench in their own tree:
//! `cargo bench -p nautilus-data --bench engine`.
//!
//! The reference is `nautilus_trader`'s own `crates/data/benches/engine.rs`,
//! which measures `DataEngine::process_data(Data::Trade(..))` — engine enum
//! dispatch, then `Cache::add_trade`, then `msgbus::publish_trade` with no
//! subscribers attached.
//!
//! Two figures, because one alone would mislead:
//!
//! - `engine_only` — a source and one map. Wingfoil's per-event engine floor,
//!   with no cache and no publish. This is *not* comparable to their 149 ns.
//! - `engine_plus_cache_compiled` — the same wiring through `nitro!`'s
//!   `compiled()` tier, which is where the difference actually appears.
//! - `engine_plus_cache` — the same, plus a node writing into a
//!   `HashMap<u64, VecDeque<Trade>>` bounded at `TICK_CAPACITY`, mirroring what
//!   `Cache::add_trade` does (entry lookup by instrument id, `push_front` into
//!   a bounded deque). This is the like-for-like line, and it still omits the
//!   msgbus publish, which Wingfoil has no equivalent of.
//!
//! Method follows `topological_vs_per_path/wingfoil.rs`: the driving ticker
//! lives inside the graph, each sample runs `RunFor::Cycles(CYCLES)`, and the
//! reported figure is whole-run time divided by the cycle count. The
//! `bencher::add_bench` harness is deliberately *not* used — its cross-thread
//! handshake costs a few hundred nanoseconds, which would swamp graphs costing
//! tens (see that file's module docs).
//!
//! Run with:
//! `cargo bench --manifest-path crates/wingfoil/Cargo.toml --bench vs_nautilus`

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use wingfoil::prelude::*;
use wingfoil::{NanoTime, RunFor, RunMode};

const HISTORICAL: RunMode = RunMode::HistoricalFrom(NanoTime::ZERO);
const STEP: Duration = Duration::from_nanos(100);
const CYCLES: u32 = 100_000;

/// Matches `nautilus_common::cache::CacheConfig::tick_capacity`'s default shape:
/// the deque is bounded, so steady state is a pop alongside every push.
const TICK_CAPACITY: usize = 1_000;

/// Field-for-field the shape of `nautilus_model::data::trade::TradeTick`, minus
/// their newtype wrappers — same payload width, same per-event copy cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Trade {
    instrument_id: u64,
    price: i64,
    size: u64,
    aggressor_side: u8,
    trade_id: u64,
    ts_event: u64,
    ts_init: u64,
}

fn trade_from(seq: u64) -> Trade {
    Trade {
        instrument_id: 1,
        price: 110_000,
        size: 100_000,
        aggressor_side: 1,
        trade_id: seq,
        ts_event: seq,
        ts_init: seq,
    }
}

/// The `Cache::add_trade` equivalent: entry lookup keyed by instrument, then a
/// bounded `push_front`.
fn cache_write(cache: &RefCell<HashMap<u64, VecDeque<Trade>>>, trade: &Trade) -> u64 {
    let mut cache = cache.borrow_mut();
    let deque = cache
        .entry(trade.instrument_id)
        .or_insert_with(|| VecDeque::with_capacity(TICK_CAPACITY));
    if deque.len() == TICK_CAPACITY {
        deque.pop_back();
    }
    deque.push_front(*trade);
    deque.len() as u64
}

/// Source and one map — the engine floor, no cache, no publish.
fn engine_only(g: &GraphBuilder) -> Stream<Trade> {
    g.ticker(STEP)
        .count()
        .map(|c: &u64| black_box(trade_from(*c)))
}

/// The like-for-like arm: engine dispatch plus a bounded per-instrument write.
fn engine_plus_cache(g: &GraphBuilder) -> Stream<u64> {
    let cache: RefCell<HashMap<u64, VecDeque<Trade>>> = RefCell::new(HashMap::new());
    engine_only(g).map(move |t: &Trade| black_box(cache_write(&cache, t)))
}

/// Returns the cache-writing closure with its own state, so the `nitro!` body
/// stays straight-line wiring (closures are construction-time `Cfg`).
fn cache_op() -> impl Fn(&Trade) -> u64 {
    let cache: RefCell<HashMap<u64, VecDeque<Trade>>> = RefCell::new(HashMap::new());
    move |t: &Trade| black_box(cache_write(&cache, t))
}

wingfoil::nitro! {
    fn cached(g: &GraphBuilder) -> Stream<u64> {
        let src = g.ticker(STEP).count().map(|c: &u64| black_box(trade_from(*c)));
        let out = src.map(cache_op());
        out
    }
}

fn bench(crit: &mut Criterion) {
    let run_for = RunFor::Cycles(CYCLES);
    let mut grp = crit.benchmark_group("vs_nautilus");
    grp.sample_size(20);
    grp.throughput(Throughput::Elements(CYCLES as u64));

    grp.bench_function("engine_only", |b| {
        b.iter_batched(
            || {
                let g = GraphBuilder::new();
                let out = engine_only(&g);
                (g.build(), out)
            },
            |(mut runner, out)| {
                runner.run(HISTORICAL, run_for).unwrap();
                black_box(runner.value(&out))
            },
            BatchSize::PerIteration,
        )
    });

    grp.bench_function("engine_plus_cache", |b| {
        b.iter_batched(
            || {
                let g = GraphBuilder::new();
                let out = engine_plus_cache(&g);
                (g.build(), out)
            },
            |(mut runner, out)| {
                runner.run(HISTORICAL, run_for).unwrap();
                black_box(runner.value(&out))
            },
            BatchSize::PerIteration,
        )
    });

    grp.bench_function("engine_plus_cache_compiled", |b| {
        b.iter(|| black_box(cached::compiled(HISTORICAL, run_for).unwrap()))
    });

    grp.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
