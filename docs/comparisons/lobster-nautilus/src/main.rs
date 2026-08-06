//! The wingfoil `order_book` example, rewritten on NautilusTrader.
//!
//! Reads the same LOBSTER message file, maintains an L3 book, and writes the
//! distinct top-of-book prices to CSV — the `prices` half of the wingfoil
//! example. See the README for why the `fills` half does not translate.

use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;

use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
};
use nautilus_common::{
    actor::{DataActor, DataActorConfig, DataActorCore},
    nautilus_actor,
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{Data, OrderBookDelta, OrderBookDeltas, order::BookOrder},
    enums::{AccountType, BookAction, BookType, OmsType, OrderSide, RecordFlag},
    identifiers::{InstrumentId, Venue},
    instruments::{Instrument, InstrumentAny, stubs::equity_aapl_itch},
    orderbook::OrderBook,
    types::{Money, Price, Quantity},
};
use serde::{Deserialize, Serialize};

/// 2012-06-21T00:00:00Z — LOBSTER's `seconds` column is seconds since midnight,
/// so it needs a session date to become UNIX nanos.
const SESSION_BASE_NS: u64 = 1_340_236_800_000_000_000;

/// LOBSTER prices are in ten-thousandths of a dollar.
const PRICE_SCALE: f64 = 10_000.0;

/// One row of the LOBSTER message file — identical to the wingfoil example's.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Message {
    seconds: f64,
    message_type: u8,
    order_id: u128,
    quantity: u64,
    price: u64,
    direction: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
struct TwoWayPrice {
    bid_price: Option<i64>,
    ask_price: Option<i64>,
}

/// LOBSTER messages -> Nautilus book deltas.
///
/// There is no LOBSTER loader in Nautilus (they ship Binance, Databento and
/// Tardis), so this is the wrangler the framework would otherwise provide.
fn lobster_to_deltas(rows: &[Message], id: InstrumentId) -> anyhow::Result<Vec<OrderBookDelta>> {
    // Nautilus's OrderBook applies deltas; it does not match. LOBSTER's type-4
    // executions therefore cannot be replayed as aggressor orders the way the
    // wingfoil example does (its `lobster::OrderBook::execute` matches) — the
    // wrangler has to do the size bookkeeping itself and emit the resulting
    // Update/Delete on the *resting* order's own side.
    let mut resting: std::collections::HashMap<u64, (OrderSide, u64)> =
        std::collections::HashMap::new();
    let mut out = Vec::with_capacity(rows.len());
    let mut orphans = 0usize;

    for (seq, m) in rows.iter().enumerate() {
        // Type 5 is a hidden-order execution: no book impact, same as wingfoil.
        if m.message_type == 5 {
            continue;
        }
        let oid = m.order_id as u64;
        // For every LOBSTER type, `direction` is the side of the limit order
        // resting in (or entering) the book — including type 4, where it is the
        // side of the order being executed, not the aggressor's.
        let msg_side = match m.direction {
            1 => OrderSide::Buy,
            -1 => OrderSide::Sell,
            d => anyhow::bail!("failed to parse direction: {d}"),
        };
        let px = Price::new(m.price as f64 / PRICE_SCALE, 4);

        let (action, side, size) = match m.message_type {
            1 => {
                let e = resting.entry(oid).or_insert((msg_side, 0));
                e.0 = msg_side;
                e.1 += m.quantity;
                (BookAction::Add, msg_side, e.1)
            }
            // 2 partial cancel, 3 full cancel, 4 execution against a resting
            // order — all three reduce a resting order by `quantity`.
            2 | 3 | 4 => {
                let Some((side, remaining)) = resting.get_mut(&oid) else {
                    // Order rested before the file's first row; nothing to reduce.
                    orphans += 1;
                    continue;
                };
                let side = *side;
                let full = m.message_type == 3 || m.quantity >= *remaining;
                if full {
                    resting.remove(&oid);
                    (BookAction::Delete, side, 0)
                } else {
                    *remaining -= m.quantity;
                    (BookAction::Update, side, *remaining)
                }
            }
            other => anyhow::bail!("unrecognised message type: {other}"),
        };

        // Nautilus rejects a zero size on Add/Update; Delete carries the last
        // known size, which the book matches on order id anyway.
        let qty = if matches!(action, BookAction::Delete) {
            m.quantity.max(1)
        } else {
            size.max(1)
        };
        let order = BookOrder::new(side, px, Quantity::new(qty as f64, 0), oid);
        let ts: UnixNanos = (SESSION_BASE_NS + (m.seconds * 1e9) as u64).into();
        out.push(OrderBookDelta::new(id, action, order, 0, seq as u64, ts, ts));
    }
    // Wingfoil groups same-instant messages into one `Burst`, so the book is
    // advanced to a consistent state before its top is read. Nautilus's
    // equivalent is the F_LAST record flag: the engine buffers deltas and
    // delivers one `OrderBookDeltas` batch when it sees the flag. Without it
    // every delta is its own batch and intermediate states leak into the output.
    for i in 0..out.len() {
        let last_of_instant = i + 1 == out.len() || out[i + 1].ts_event != out[i].ts_event;
        if last_of_instant {
            out[i].flags = RecordFlag::F_LAST as u8;
        }
    }
    if orphans > 0 {
        println!("  {orphans} messages referenced orders resting before the file starts");
    }
    Ok(out)
}

/// Maintains the book and records each *change* in top of book — the wingfoil
/// example's `.filter_none().distinct()`, hand-rolled.
struct TopOfBookActor {
    core: DataActorCore,
    instrument_id: InstrumentId,
    book: OrderBook,
    prices: Vec<TwoWayPrice>,
    last: Option<TwoWayPrice>,
    batches: u64,
    delta_count: u64,
    out_path: PathBuf,
}

impl TopOfBookActor {
    fn new(instrument_id: InstrumentId, out_path: PathBuf) -> Self {
        Self {
            core: DataActorCore::new(DataActorConfig::default()),
            instrument_id,
            book: OrderBook::new(instrument_id, BookType::L3_MBO),
            prices: Vec::new(),
            last: None,
            batches: 0,
            delta_count: 0,
            out_path,
        }
    }
}

impl DataActor for TopOfBookActor {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_book_deltas(
            self.instrument_id,
            BookType::L3_MBO,
            None,  // depth
            None,  // client_id
            false, // managed — we keep our own book
            None,  // params
        );
        Ok(())
    }

    fn on_book_deltas(&mut self, deltas: &OrderBookDeltas) -> anyhow::Result<()> {
        // A book integrity error is data-dependent (a delete for an unknown
        // order id, say); skip the batch rather than abort the run.
        self.batches += 1;
        self.delta_count += deltas.deltas.len() as u64;
        if self.book.apply_deltas(deltas).is_err() {
            return Ok(());
        }
        let px = TwoWayPrice {
            bid_price: self.book.best_bid_price().map(|p| p.raw),
            ask_price: self.book.best_ask_price().map(|p| p.raw),
        };
        if self.last.as_ref() != Some(&px) {
            self.prices.push(px.clone());
            self.last = Some(px);
        }
        Ok(())
    }

    fn on_stop(&mut self) -> anyhow::Result<()> {
        let mut w = csv::Writer::from_path(&self.out_path)?;
        for p in &self.prices {
            w.serialize(p)?;
        }
        w.flush()?;
        println!("  {} two-way prices -> {}", self.prices.len(), self.out_path.display());
        println!("  {} batches, {} deltas, mean batch {:.2}", self.batches, self.delta_count, self.delta_count as f64 / self.batches.max(1) as f64);
        Ok(())
    }
}

nautilus_actor!(TopOfBookActor);

impl Debug for TopOfBookActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(TopOfBookActor))
            .field("instrument_id", &self.instrument_id)
            .finish()
    }
}

fn main() -> anyhow::Result<()> {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // The very same file the wingfoil example replays.
    let source_path = here
        .join("../../../crates/wingfoil/examples/core/order_book/data/aapl.csv");
    let prices_path = here.join("prices.csv");

    let rows: Vec<Message> = csv::Reader::from_path(&source_path)?
        .deserialize()
        .collect::<Result<_, _>>()?;

    let instrument = InstrumentAny::Equity(equity_aapl_itch());
    let instrument_id = instrument.id();
    let deltas = lobster_to_deltas(&rows, instrument_id)?;
    println!("{} messages -> {} deltas", rows.len(), deltas.len());

    let mut engine = BacktestEngine::new(BacktestEngineConfig::default())?;
    engine.add_venue(
        SimulatedVenueConfig::builder()
            .venue(Venue::from("XNAS"))
            .oms_type(OmsType::Netting)
            .account_type(AccountType::Cash)
            .book_type(BookType::L3_MBO)
            .starting_balances(vec![Money::from("1_000_000 USD")])
            .build()?,
    )?;
    engine.add_instrument(&instrument)?;
    engine.add_data(
        deltas.into_iter().map(Data::Delta).collect(),
        None,
        false,
        true,
    )?;
    engine.add_actor(TopOfBookActor::new(instrument_id, prices_path))?;

    let started = Instant::now();
    engine.run(None, None, None, false)?;
    println!("in {:.3?}", started.elapsed());
    Ok(())
}
