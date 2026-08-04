//! Shared source module for the graph-dynamism examples: instrument lifecycle
//! events and a price feed. The wingfoil twin of legacy
//! `examples/dynamic/market_data.rs`, so all five approaches below are compared
//! against the same oracle.
//!
//! Every example under `examples/core/dynamism/` pulls this in with
//! `#[path = "../market_data.rs"] mod market_data;`, exactly as the legacy tree
//! does — one scenario, five wirings.

// Each example uses a different subset of the three streams (the demux ones
// never look at `new_instrument`, and only two call `market_data_offset`).
#![allow(dead_code)]

use std::time::Duration;

use wingfoil::prelude::*;

pub type Instrument = String;
pub type Price = f64;

/// Bundles all three streams produced by [`market_data`].
pub struct MarketData {
    /// Fires when a new instrument is added.
    pub new_instrument: Stream<Instrument>,
    /// Fires when an instrument is deleted.
    pub del_instrument: Stream<Instrument>,
    /// Continuous price stream, cycling through instrument IDs.
    pub inst_price: Stream<(Instrument, Price)>,
}

/// State threaded through the lifecycle [`fold`](StreamOps::fold).
///
/// Instruments are added sequentially. Every 3rd event deletes the oldest
/// live instrument (if any), producing an unbounded stream of additions and
/// periodic deletions.
#[derive(Clone, Debug)]
struct LifecycleState {
    /// Counter for naming the next instrument (`inst1`, `inst2`, …).
    next_id: u64,
    /// Currently live instruments, in insertion order.
    live: Vec<Instrument>,
    /// The event to emit on this tick: `(is_add, instrument)`.
    event: (bool, Instrument),
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            next_id: 1,
            live: Vec::new(),
            event: (true, String::new()),
        }
    }
}

/// Two tickers, both firing every `period` **in the same engine cycle**:
///
/// - `price_ticker` — continuous price feed, cycling through instrument IDs
///   in a sliding window: `id = (n-1)/3 + (n-1)%3 + 1`
/// - `event_ticker` — instrument lifecycle; every 3rd tick deletes the oldest
///   live instrument, all other ticks add a fresh one
///
/// Prices for unknown or deleted instruments are silently dropped by the
/// aggregator's per-instrument filters.
pub fn market_data(g: &GraphBuilder, period: Duration) -> MarketData {
    market_data_offset(g, period, Duration::ZERO)
}

/// [`market_data`] with the lifecycle ticker phase-shifted by `offset`.
///
/// With `offset == ZERO` a price and a delete for *different* instruments land
/// on the same engine cycle — fine for [`Builder::demux_it`], which routes each
/// item of a burst independently, but impossible for the single-value demux
/// APIs ([`Builder::demux`] / [`Builder::demux_map`]), which route exactly one
/// value per cycle. Offsetting by half a period interleaves the two feeds so at
/// most one event lands per cycle, and every price and every delete survives —
/// only *when* they land changes.
///
/// [`Builder::demux`]: wingfoil::interp::Builder::demux
/// [`Builder::demux_it`]: wingfoil::interp::Builder::demux_it
/// [`Builder::demux_map`]: wingfoil::interp::Builder::demux_map
pub fn market_data_offset(g: &GraphBuilder, period: Duration, offset: Duration) -> MarketData {
    let price_ticker = g.ticker(period);
    let event_count = {
        let count = g.ticker(period).count();
        if offset.is_zero() {
            count
        } else {
            count.delay(offset)
        }
    };

    let lifecycle = event_count
        .fold(
            LifecycleState::default(),
            |state: &mut LifecycleState, n: &u64| {
                if n.is_multiple_of(3) && !state.live.is_empty() {
                    // Delete the oldest live instrument.
                    let inst = state.live.remove(0);
                    state.event = (false, inst);
                } else {
                    // Add a fresh instrument.
                    let inst = format!("inst{}", state.next_id);
                    state.next_id += 1;
                    state.live.push(inst.clone());
                    state.event = (true, inst);
                }
            },
        )
        .map(|state: &LifecycleState| state.event.clone());

    // `map_filter` returns `(value, emit?)` — the wingfoil spelling of
    // legacy's `filter_value(..).map(..)` pair.
    let new_instrument =
        lifecycle.map_filter(|(is_new, inst): &(bool, Instrument)| (inst.clone(), *is_new));
    let del_instrument =
        lifecycle.map_filter(|(is_new, inst): &(bool, Instrument)| (inst.clone(), !*is_new));

    let inst_price = price_ticker.count().map(|n: &u64| {
        let id = (n - 1) / 3 + (n - 1) % 3 + 1;
        (format!("inst{id}"), id as f64 + *n as f64 / 100.0)
    });

    MarketData {
        new_instrument,
        del_instrument,
        inst_price,
    }
}
