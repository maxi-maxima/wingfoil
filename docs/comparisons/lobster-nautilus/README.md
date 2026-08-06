# The wingfoil order-book example, on NautilusTrader

This is the working port behind the "same example, both frameworks" section of
[`docs/comparison.md`](../../comparison.md). It replays the **same LOBSTER file**
as [`examples/core/order_book`](../../../crates/wingfoil/examples/core/order_book/)
— `crates/wingfoil/examples/core/order_book/data/aapl.csv`, 91,997 rows — through
a Nautilus `BacktestEngine` with a `DataActor` subscribed to `L3_MBO` book
deltas, and writes the distinct top-of-book prices to `prices.csv`.

It exists so the comparison page makes a claim you can rerun rather than one you
have to take on trust.

## Run

```sh
cd docs/comparisons/lobster-nautilus
cargo run --release
```

Nothing else in the repository needs building, and building this changes nothing
else — see [Why it is outside the workspace](#why-it-is-outside-the-workspace).

## What it prints

```
  84 messages referenced orders resting before the file starts
91997 messages -> 89712 deltas
  16161 two-way prices -> .../prices.csv
  89712 batches, 89712 deltas, mean batch 1.00
in 205.741ms
```

against the wingfoil example's

```
  15040 two-way prices -> examples/core/order_book/data/prices.csv
  4169 fills          -> examples/core/order_book/data/fills.csv
in 150.020ms
```

Do not read the timings as a benchmark. The two programs do different work —
this one stands up a venue, an account and a portfolio, and prints a full
performance report — and the ingest and fan-out numbers in
[`comparison.md`](../../comparison.md) are the measurements that were actually
controlled for.

## The three things the port teaches

**1. Same-instant grouping.** Wingfoil delivers same-instant messages as one
`Burst`, so the book reaches a consistent state before its top is read. The
actor here counts what it is actually handed: **89,712 batches for 89,712
deltas, mean batch size 1.00** — every delta arrives alone, so top of book is
sampled *mid-instant*. That is the whole of the 15,040 → 16,161 difference in
price count. Setting `RecordFlag::F_LAST` on the last delta of each instant did
not change it for an unmanaged subscription; there may be a managed-subscription
or data-client path that does batch, and this port does not exhaust their
options. The point is that a straightforward translation reads a half-updated
book and nothing warns you.

**2. The fills do not translate.** Wingfoil's book *matches* —
`lobster::OrderBook::execute` returns fill metadata, so `fills.csv` falls out of
applying a message. Nautilus's `OrderBook` applies deltas and returns
`Result<(), BookIntegrityError>`; matching lives in `OrderMatchingEngine`, which
matches *your* orders against the book rather than reconstructing the tape's
executions. So `lobster_to_deltas` has to do the size bookkeeping itself: track
every resting order, decrement it on execution, and emit the resulting `Update`
or `Delete` on the resting order's own side.

That last detail is worth dwelling on, because getting it wrong is silent. The
first version of this port copied the wingfoil example's `!side` flip for LOBSTER
type-4 — correct when you are replaying an execution as an aggressor into a
*matching* book, wrong when you are mutating one. It targeted order ids resting
on the opposite side, every batch failed its integrity check, and the run
produced 121 prices instead of 16,161. It compiled, it ran, it printed a
plausible-looking report.

**3. A trading context is mandatory.** The port needs a venue, an OMS type, an
account type, starting balances and a fully specified instrument for a program
that never places an order — and prints a Sharpe ratio, a Sortino ratio and a PnL
summary regardless. That is the framework being an application framework, and it
is exactly what you want when you *are* trading.

## Why it is outside the workspace

It is in the root `Cargo.toml`'s `exclude` list, so `cargo build`, `cargo test`
and CI at the repository root never see it. Two reasons, both deliberate:

- **Licence.** NautilusTrader is LGPL-3.0; wingfoil is Apache-2.0. This directory
  is our own code that *depends* on theirs to run, and keeping it out of the
  workspace keeps their crates out of the wingfoil build graph and out of
  `Cargo.lock`.
- **Toolchain.** It carries its own `rust-toolchain.toml` pinning the version
  their crates expect, which is generally ahead of what wingfoil's CI uses.

## Versions

Pinned to the published `nautilus-*` **0.61** crates from crates.io, so it builds
without a clone of their repository. The figures above were reproduced against
0.61; they were first obtained against a 0.62.0 development checkout and are
identical on both.
