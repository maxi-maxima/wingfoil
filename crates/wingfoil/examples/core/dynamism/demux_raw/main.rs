#![doc = include_str!("./README.md")]
//!
//! ```sh
//! cargo run --manifest-path crates/wingfoil/Cargo.toml --example demux_raw --features dynamic-graph
//! ```

#[path = "../market_data.rs"]
mod market_data;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use wingfoil::prelude::*;
use wingfoil::{NanoTime, RunFor, RunMode};

use market_data::{Instrument, Price, market_data_offset};

type PriceBook = BTreeMap<Instrument, Price>;

const PERIOD: Duration = Duration::from_secs(1);

/// Same half-period phase shift as the `demux_map` example: the raw primitive
/// routes one value per cycle, so lifecycle and price events must not coincide.
const OFFSET: Duration = Duration::from_millis(500);

const HISTORICAL: RunMode = RunMode::HistoricalFrom(NanoTime::ZERO);
const RUN_FOR: RunFor = RunFor::Duration(Duration::from_secs(19));

/// Number of real slots. The raw primitive treats any route `>= SIZE` as
/// overflow, so `SIZE` itself is the "no slot" answer.
const SIZE: usize = 10;

#[derive(Clone, Debug, Default, PartialEq)]
enum InstEvent {
    #[default]
    None,
    Price(Instrument, Price),
    Delete(Instrument),
}

fn inst_key(event: &InstEvent) -> Instrument {
    match event {
        InstEvent::Price(inst, _) | InstEvent::Delete(inst) => inst.clone(),
        InstEvent::None => String::new(),
    }
}

fn round(price: Price) -> Price {
    (price * 100.0).round()
}

/// The key→slot bookkeeping that [`Builder::demux_map`] does for you, written
/// out. Handing a new key the *lowest* free slot (a [`BTreeSet`], not a
/// `HashSet`) is what makes assignment deterministic — the safer backtest
/// default, and the same choice wingfoil's own [`DemuxMap`] makes.
///
/// [`Builder::demux_map`]: wingfoil::interp::Builder::demux_map
/// [`DemuxMap`]: wingfoil::interp::Builder::demux_map
struct SlotPool {
    assigned: HashMap<Instrument, usize>,
    free: BTreeSet<usize>,
}

impl SlotPool {
    fn new(size: usize) -> Self {
        Self {
            assigned: HashMap::new(),
            free: (0..size).collect(),
        }
    }

    /// The key's slot, claiming a free one if this is the first time it is seen.
    /// `None` once the pool is exhausted — the caller routes that to overflow.
    fn assign(&mut self, key: Instrument) -> Option<usize> {
        if let Some(&slot) = self.assigned.get(&key) {
            return Some(slot);
        }
        let slot = self.free.pop_first()?;
        self.assigned.insert(key, slot);
        Some(slot)
    }

    /// The key's slot one last time, returning it to the pool afterwards.
    fn release(&mut self, key: &Instrument) -> Option<usize> {
        let slot = self.assigned.remove(key)?;
        self.free.insert(slot);
        Some(slot)
    }
}

/// Wire the price book with the **raw**
/// [`Builder::demux`](wingfoil::interp::Builder::demux) primitive: pre-wire
/// `SIZE` children plus an overflow child, and each cycle route the source
/// value to one of them by index. There is no key lifecycle here — `route`
/// hands back a `usize` and nothing else — so the [`SlotPool`] above supplies
/// the assignment and release that `demux_map` layers on top for you.
///
/// The wiring is otherwise identical to the `demux_map` example, and the test
/// asserts both produce the same book, state for state.
fn build() -> (GraphBuilder, Stream<PriceBook>, Stream<InstEvent>) {
    let g = GraphBuilder::new();
    let src = market_data_offset(&g, PERIOD, OFFSET);

    let price_events = src
        .inst_price
        .map(|(inst, px): &(Instrument, Price)| InstEvent::Price(inst.clone(), *px));
    let del_events = src
        .del_instrument
        .map(|inst: &Instrument| InstEvent::Delete(inst.clone()));
    let events = g.combine(&[price_events, del_events]).collapse().handle();

    // `route` must be `Fn`, so the pool lives behind a `RefCell`.
    let pool = RefCell::new(SlotPool::new(SIZE));
    let (slots, overflow) = g.with_builder(|b| {
        b.demux(events, SIZE, move |event: &InstEvent| {
            let key = inst_key(event);
            let mut pool = pool.borrow_mut();
            match event {
                InstEvent::Delete(_) => pool.release(&key),
                _ => pool.assign(key),
            }
            // Anything `>= SIZE` is the overflow child.
            .unwrap_or(SIZE)
        })
    });

    let processed: Vec<Stream<InstEvent>> = slots
        .iter()
        .map(|&slot| {
            g.wrap(slot).map(|event: &InstEvent| match event {
                InstEvent::Price(inst, px) => InstEvent::Price(inst.clone(), round(*px)),
                other => other.clone(),
            })
        })
        .collect();

    let book = g.combine(&processed).fold(
        PriceBook::new(),
        |book: &mut PriceBook, events: &Burst<InstEvent>| {
            for event in events {
                match event {
                    InstEvent::Price(inst, px) => {
                        book.insert(inst.clone(), *px);
                    }
                    InstEvent::Delete(inst) => {
                        book.remove(inst);
                    }
                    InstEvent::None => {}
                }
            }
        },
    );

    let overflow = g.wrap(overflow);
    (g, book, overflow)
}

fn render(book: &PriceBook) -> String {
    let body = book
        .iter()
        .map(|(inst, px)| format!("{inst}={px}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{body}}}")
}

fn main() -> anyhow::Result<()> {
    let (g, book, _overflow) = build();

    let _printed = book
        .with_time()
        .for_each(|(t, book): &(NanoTime, PriceBook)| {
            let secs = f64::from(*t) * NanoTime::SECONDS_PER_NANO;
            println!("t={secs:>5.1}s  price book (demux_raw): {}", render(book));
            Ok(())
        });

    let mut runner = g.build();
    runner.run(HISTORICAL, RUN_FOR)?;
    Ok(())
}

#[cfg(test)]
#[path = "../oracle.rs"]
mod oracle;

#[cfg(test)]
mod tests {
    use super::*;

    /// The hand-rolled pool reproduces `demux_map` exactly — same states, same
    /// order — which is the whole claim this example makes.
    #[test]
    fn hand_rolled_pool_matches_demux_map() {
        let (g, book, overflow) = build();
        let states = book.accumulate();
        let overflowed = overflow.accumulate();
        let mut runner = g.build();
        runner.run(HISTORICAL, RUN_FOR).unwrap();

        assert_eq!(
            runner.value(&states),
            oracle::offset_book_states(),
            "book states"
        );
        assert!(
            runner.value(&overflowed).is_empty(),
            "pool of {SIZE} covers peak concurrency; overflow must never fire"
        );
    }
}
