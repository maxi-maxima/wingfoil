//! Fan-out: one event delivered to N consumers.
//!
//! The companion to [`vs_nautilus.rs`](vs_nautilus.rs), which measured a single
//! consumer. This one sweeps the width, because that is where a monomorphised
//! graph and a per-event dispatch model should diverge.
//!
//! The Nautilus reference is again **their own unmodified bench**:
//! `crates/common/benches/msgbus.rs`, function `bench_router_multiple_subscribers`,
//! which sweeps `sub_count` over `[1, 5, 10]` for two router flavours — an
//! `Any`-based router that downcasts per delivery, and a typed router that does
//! not. Run it with `cargo bench -p nautilus-common --bench msgbus`.
//!
//! # Matching the per-consumer work
//!
//! Their handler body is
//! `COUNTER.fetch_add(quote.bid_price.raw as u64, Ordering::Relaxed)` — an
//! atomic read-modify-write on a global, chosen to defeat dead-code
//! elimination. Ours is **the same atomic on the same kind of global**, not a
//! `black_box` arithmetic op, because an uncontended `fetch_add` is several
//! nanoseconds and using anything cheaper would silently credit Wingfoil with
//! the difference. With the per-consumer work identical, what the sweep
//! isolates is dispatch.
//!
//! # Read the slope, not the intercept
//!
//! The two harnesses do not share a fixed cost and are not trying to. Their
//! `b.iter()` calls `router.publish` directly; ours runs a graph whose ticker,
//! `count` and `TimeQueue` re-arm are inside the measurement. So the intercepts
//! are not comparable and the doc does not compare them.
//!
//! What *is* comparable is the **marginal cost of one more consumer** —
//! `(t(10) - t(1)) / 9` on each side — because whatever fixed overhead each
//! harness carries cancels out of a difference. That slope is the number
//! `docs/comparison.md` quotes.
//!
//! Run with:
//! `cargo bench --manifest-path crates/wingfoil/Cargo.toml --bench vs_nautilus_fanout`

use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use wingfoil::prelude::*;
use wingfoil::{NanoTime, RunFor, RunMode};

const HISTORICAL: RunMode = RunMode::HistoricalFrom(NanoTime::ZERO);
const STEP: Duration = Duration::from_nanos(100);
const CYCLES: u32 = 100_000;

/// Their `COUNTER`, same purpose: give each consumer work that cannot be
/// optimised away, and make it identical on both sides.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The shape of `nautilus_model::data::quote::QuoteTick`, minus the newtypes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Quote {
    instrument_id: u64,
    bid_price: i64,
    ask_price: i64,
    bid_size: u64,
    ask_size: u64,
    ts_event: u64,
    ts_init: u64,
}

fn quote_from(seq: u64) -> Quote {
    Quote {
        instrument_id: 1,
        bid_price: 110_000,
        ask_price: 110_010,
        bid_size: 100_000,
        ask_size: 100_000,
        ts_event: seq,
        ts_init: seq,
    }
}

/// Byte-for-byte the work in their `CountingTypedHandler::handle`.
fn consume(quote: &Quote) -> u64 {
    COUNTER.fetch_add(quote.bid_price as u64, Ordering::Relaxed)
}

/// One `nitro!` block per width: a ticker-driven source fanned into N `map`
/// consumers. Invoked as `fanout_graph!(fan_5, [b0 b1 b2 b3 b4], b4)` — the
/// bracketed list names every branch, the trailing name is the one returned.
///
/// Naming the branches at the call site rather than generating them keeps the
/// expansion a flat run of `let`s, which `nitro!` requires: it reads the body as
/// straight-line wiring and rejects loops, and nodes built in a loop would be
/// invisible to the compiled emission anyway.
macro_rules! fanout_graph {
    ($name:ident, [$($b:ident)*], $last:ident) => {
        wingfoil::nitro! {
            fn $name(g: &GraphBuilder) -> Stream<u64> {
                let src = g.ticker(STEP).count().map(|c: &u64| black_box(quote_from(*c)));
                $(let $b = src.map(consume);)*
                $last
            }
        }
    };
}

fanout_graph!(fan_1, [b0], b0);
fanout_graph!(fan_5, [b0 b1 b2 b3 b4], b4);
fanout_graph!(fan_10, [b0 b1 b2 b3 b4 b5 b6 b7 b8 b9], b9);

macro_rules! width_group {
    ($crit:expr, $width:literal, $module:ident) => {{
        let run_for = RunFor::Cycles(CYCLES);
        let mut grp = $crit.benchmark_group(concat!("fanout_", $width));
        grp.sample_size(20);
        grp.throughput(Throughput::Elements(CYCLES as u64));

        grp.bench_function("interpreted", |b| {
            b.iter_batched(
                || $module::interpreted(),
                |(mut runner, out)| {
                    runner.run(HISTORICAL, run_for).unwrap();
                    black_box(runner.value(out))
                },
                BatchSize::PerIteration,
            )
        });

        grp.bench_function("compiled", |b| {
            b.iter(|| black_box($module::compiled(HISTORICAL, run_for).unwrap()))
        });

        grp.finish();
    }};
}

fn bench(crit: &mut Criterion) {
    width_group!(crit, "1", fan_1);
    width_group!(crit, "5", fan_5);
    width_group!(crit, "10", fan_10);
}

criterion_group!(benches, bench);
criterion_main!(benches);
