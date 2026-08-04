//! The parity oracle shared by the dynamism examples' tests — the wingfoil
//! twin of legacy `examples/dynamic/tests.rs`.
//!
//! Every approach maintains the *same* price book over the *same* market-data
//! scenario, so the expected state sequences live here once rather than being
//! restated five times. Where an approach legitimately observes a different
//! sequence (see [`demux_book_states`]) the difference is spelled out at the
//! function, not hidden in a per-example copy.

#![allow(dead_code)]

use std::collections::BTreeMap;

use super::market_data::{Instrument, Price};

#[cfg(test)]
mod source {
    use std::time::Duration;

    use wingfoil::prelude::*;
    use wingfoil::{NanoTime, RunFor, RunMode};

    use crate::market_data::{Instrument, Price, market_data};

    /// The scenario itself, checked once per example so a change to
    /// `market_data.rs` cannot silently shift every book oracle at once.
    ///
    /// 9 lifecycle ticks, both tickers at the same period:
    ///
    ///   n=1: add  inst1          n=2: add  inst2          n=3: del  inst1
    ///   n=4: add  inst3          n=5: add  inst4          n=6: del  inst2
    ///   n=7: add  inst5          n=8: add  inst6          n=9: del  inst3
    ///
    ///   price id = (n-1)/3 + (n-1)%3 + 1  →  sliding window inst1..inst5
    #[test]
    fn lifecycle_and_price_sequence() {
        let g = GraphBuilder::new();
        let src = market_data(&g, Duration::from_secs(1));
        let new_insts = src.new_instrument.accumulate();
        let del_insts = src.del_instrument.accumulate();
        let prices = src.inst_price.accumulate();

        let mut runner = g.build();
        runner
            .run(RunMode::HistoricalFrom(NanoTime::ZERO), RunFor::Cycles(9))
            .unwrap();

        assert_eq!(
            runner.value(&new_insts),
            ["inst1", "inst2", "inst3", "inst4", "inst5", "inst6"]
                .map(String::from)
                .to_vec()
        );
        assert_eq!(
            runner.value(&del_insts),
            ["inst1", "inst2", "inst3"].map(String::from).to_vec()
        );
        assert_eq!(
            runner.value(&prices),
            vec![
                ("inst1".into(), 1.01),
                ("inst2".into(), 2.02),
                ("inst3".into(), 3.03),
                ("inst2".into(), 2.04),
                ("inst3".into(), 3.05),
                ("inst4".into(), 4.06),
                ("inst3".into(), 3.07),
                ("inst4".into(), 4.08),
                ("inst5".into(), 5.09),
            ] as Vec<(Instrument, Price)>
        );
    }
}

pub type PriceBook = BTreeMap<Instrument, Price>;

fn b(pairs: &[(&str, f64)]) -> PriceBook {
    pairs.iter().map(|&(k, v)| (k.into(), v)).collect()
}

/// Expected price-book state after each emission over 20 lifecycle ticks, for
/// the approaches driven by the **explicit `new_instrument` stream** —
/// `dynamic_group` and `dynamic_manual`.
///
/// Lifecycle (event ticker, every period):
///   n=1  add inst1    n=2  add inst2    n=3  del inst1
///   n=4  add inst3    n=5  add inst4    n=6  del inst2
///   n=7  add inst5    n=8  add inst6    n=9  del inst3
///   n=10 add inst7    n=11 add inst8    n=12 del inst4
///   n=13 add inst9    n=14 add inst10   n=15 del inst5
///   n=16 add inst11   n=17 add inst12   n=18 del inst6
///   n=19 add inst13   n=20 add inst14
///
/// Price id = (n-1)/3 + (n-1)%3 + 1; rounded price = (id + n/100) * 100.
///
/// At n=3 only a deletion fires and the price is for inst3 (not yet live), so
/// the aggregator does not emit — 19 states, not 20.
pub fn dynamic_book_states() -> Vec<PriceBook> {
    vec![
        b(&[("inst1", 101.0)]),                   // n=1
        b(&[("inst1", 101.0), ("inst2", 202.0)]), // n=2
        // n=3 skipped: del inst1, price for inst3 (not yet live)
        b(&[("inst2", 204.0)]),                   // n=4
        b(&[("inst2", 204.0), ("inst3", 305.0)]), // n=5
        b(&[("inst3", 305.0), ("inst4", 406.0)]), // n=6
        b(&[("inst3", 307.0), ("inst4", 406.0)]), // n=7
        b(&[("inst3", 307.0), ("inst4", 408.0)]), // n=8
        b(&[("inst4", 408.0), ("inst5", 509.0)]), // n=9
        b(&[("inst4", 410.0), ("inst5", 509.0)]), // n=10
        b(&[("inst4", 410.0), ("inst5", 511.0)]), // n=11
        b(&[("inst5", 511.0), ("inst6", 612.0)]), // n=12
        b(&[("inst5", 513.0), ("inst6", 612.0)]), // n=13
        b(&[("inst5", 513.0), ("inst6", 614.0)]), // n=14
        b(&[("inst6", 614.0), ("inst7", 715.0)]), // n=15
        b(&[("inst6", 616.0), ("inst7", 715.0)]), // n=16
        b(&[("inst6", 616.0), ("inst7", 717.0)]), // n=17
        b(&[("inst7", 717.0), ("inst8", 818.0)]), // n=18
        b(&[("inst7", 719.0), ("inst8", 818.0)]), // n=19
        b(&[("inst7", 719.0), ("inst8", 820.0)]), // n=20
    ]
}

/// Expected price-book state after each emission for the **single-value** demux
/// approaches — `demux_map` and the raw `demux` primitive — which run against
/// the *phase-shifted* market data (`market_data_offset`, half a period).
///
/// Both APIs route exactly one value per cycle, so a price and a delete may not
/// land on the same cycle; the lifecycle feed is offset by half a period to
/// guarantee that. Nothing is lost — every price and every delete still
/// arrives — but the book emits at 26 instants rather than 20:
///
/// - prices on the second (`t = 0, 1, … 19s`) — 20 emissions;
/// - deletes on the half second (`t = 2.5, 5.5, … 17.5s`) — 6 emissions.
///
/// So this sequence relates to [`demux_book_states`] by *splitting* each of
/// legacy's six delete cycles into two: the pre-delete state (that cycle's price
/// applied while the doomed instrument is still in the book) and then the
/// post-delete state, which is exactly legacy's. On every other cycle the two
/// agree.
pub fn offset_book_states() -> Vec<PriceBook> {
    vec![
        b(&[("inst1", 101.0)]),                                     // t= 0.0  price
        b(&[("inst1", 101.0), ("inst2", 202.0)]),                   // t= 1.0  price
        b(&[("inst1", 101.0), ("inst2", 202.0), ("inst3", 303.0)]), // t= 2.0  price
        b(&[("inst2", 202.0), ("inst3", 303.0)]),                   // t= 2.5  del inst1
        b(&[("inst2", 204.0), ("inst3", 303.0)]),                   // t= 3.0  price
        b(&[("inst2", 204.0), ("inst3", 305.0)]),                   // t= 4.0  price
        b(&[("inst2", 204.0), ("inst3", 305.0), ("inst4", 406.0)]), // t= 5.0  price
        b(&[("inst3", 305.0), ("inst4", 406.0)]),                   // t= 5.5  del inst2
        b(&[("inst3", 307.0), ("inst4", 406.0)]),                   // t= 6.0  price
        b(&[("inst3", 307.0), ("inst4", 408.0)]),                   // t= 7.0  price
        b(&[("inst3", 307.0), ("inst4", 408.0), ("inst5", 509.0)]), // t= 8.0  price
        b(&[("inst4", 408.0), ("inst5", 509.0)]),                   // t= 8.5  del inst3
        b(&[("inst4", 410.0), ("inst5", 509.0)]),                   // t= 9.0  price
        b(&[("inst4", 410.0), ("inst5", 511.0)]),                   // t=10.0  price
        b(&[("inst4", 410.0), ("inst5", 511.0), ("inst6", 612.0)]), // t=11.0  price
        b(&[("inst5", 511.0), ("inst6", 612.0)]),                   // t=11.5  del inst4
        b(&[("inst5", 513.0), ("inst6", 612.0)]),                   // t=12.0  price
        b(&[("inst5", 513.0), ("inst6", 614.0)]),                   // t=13.0  price
        b(&[("inst5", 513.0), ("inst6", 614.0), ("inst7", 715.0)]), // t=14.0  price
        b(&[("inst6", 614.0), ("inst7", 715.0)]),                   // t=14.5  del inst5
        b(&[("inst6", 616.0), ("inst7", 715.0)]),                   // t=15.0  price
        b(&[("inst6", 616.0), ("inst7", 717.0)]),                   // t=16.0  price
        b(&[("inst6", 616.0), ("inst7", 717.0), ("inst8", 818.0)]), // t=17.0  price
        b(&[("inst7", 717.0), ("inst8", 818.0)]),                   // t=17.5  del inst6
        b(&[("inst7", 719.0), ("inst8", 818.0)]),                   // t=18.0  price
        b(&[("inst7", 719.0), ("inst8", 820.0)]),                   // t=19.0  price
    ]
}

/// Expected price-book state after each emission over 20 cycles for the
/// **`demux_it`** approach.
///
/// Demux discovers instruments from *price* events rather than from an explicit
/// add stream, so it diverges from [`dynamic_book_states`] at the start: at n=3
/// the combined burst carries both `Price(inst3, 303)` **and** `Delete(inst1)`,
/// so a slot opens for inst3 immediately and the book emits — 20 states (one
/// per cycle) rather than 19. From n=6 onward the two sequences agree.
pub fn demux_book_states() -> Vec<PriceBook> {
    vec![
        b(&[("inst1", 101.0)]),                   // n=1
        b(&[("inst1", 101.0), ("inst2", 202.0)]), // n=2
        b(&[("inst2", 202.0), ("inst3", 303.0)]), // n=3: inst3 slot opened via price
        b(&[("inst2", 204.0), ("inst3", 303.0)]), // n=4
        b(&[("inst2", 204.0), ("inst3", 305.0)]), // n=5
        b(&[("inst3", 305.0), ("inst4", 406.0)]), // n=6
        b(&[("inst3", 307.0), ("inst4", 406.0)]), // n=7
        b(&[("inst3", 307.0), ("inst4", 408.0)]), // n=8
        b(&[("inst4", 408.0), ("inst5", 509.0)]), // n=9
        b(&[("inst4", 410.0), ("inst5", 509.0)]), // n=10
        b(&[("inst4", 410.0), ("inst5", 511.0)]), // n=11
        b(&[("inst5", 511.0), ("inst6", 612.0)]), // n=12
        b(&[("inst5", 513.0), ("inst6", 612.0)]), // n=13
        b(&[("inst5", 513.0), ("inst6", 614.0)]), // n=14
        b(&[("inst6", 614.0), ("inst7", 715.0)]), // n=15
        b(&[("inst6", 616.0), ("inst7", 715.0)]), // n=16
        b(&[("inst6", 616.0), ("inst7", 717.0)]), // n=17
        b(&[("inst7", 717.0), ("inst8", 818.0)]), // n=18
        b(&[("inst7", 719.0), ("inst8", 818.0)]), // n=19
        b(&[("inst7", 719.0), ("inst8", 820.0)]), // n=20
    ]
}
