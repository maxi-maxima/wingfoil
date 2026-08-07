//! Topology introspection — `Runner::topology` / `print_topology`, and the
//! window they share with node-error context.
//!
//! The wingfoil twin of legacy `Graph::print` and `Graph::format_context`
//! (`legacy/wingfoil/src/graph.rs:1347` and `:1330`). Legacy renders each
//! node's `Display`; wingfoil renders the op label plus its upstream edges,
//! which is the one deliberate difference — see `Runner::topology`.
//!
//! Cutover row 3.11: `print` was the second legacy public API with no twin
//! (after `Graph::export`, dropped by ruling 2.1), and the only one the
//! migration guide had not named.

use std::time::Duration;

use anyhow::bail;
use wingfoil::prelude::*;
use wingfoil::{NanoTime, RunFor, RunMode};

const HISTORICAL: RunMode = RunMode::HistoricalFrom(NanoTime::ZERO);

/// A chain, a fan-out, and a fan-in: every node listed once, indexed, indented
/// three spaces per dispatch layer, with its active upstreams after `<-`.
#[test]
fn topology_lists_every_node_indented_by_layer() {
    let g = GraphBuilder::new();
    let count = g.ticker(Duration::from_nanos(10)).count();
    let doubled = count.map(|v: &u64| v * 2);
    let incremented = count.map(|v: &u64| v + 1);
    let _merged = doubled.merge(&incremented);

    let expected = "\
[00] Ticker
[01]    Count <- [00]
[02]       Map <- [01]
[03]       Map <- [01]
[04]          Merge2 <- [02], [03]
";
    assert_eq!(expected, g.build().topology());
}

/// A passive edge — read but not triggering — is marked `~`. `sample` is the
/// canonical case: it fires on its trigger and *reads* its data leg, so the
/// data leg must not read as an active upstream.
#[test]
fn topology_marks_passive_edges() {
    let g = GraphBuilder::new();
    let data = g.ticker(Duration::from_nanos(10)).count();
    let trigger = g.ticker(Duration::from_nanos(25));
    let _sampled = data.sample(&trigger);

    let dump = g.build().topology();
    assert!(
        dump.contains("Sample <- [02], ~[01]"),
        "sample's trigger is active and its data leg passive: {dump}"
    );
}

/// `print_topology` is `topology` on stdout, and chains like legacy's
/// `Graph::print` did.
#[test]
fn print_topology_chains() {
    let g = GraphBuilder::new();
    let _count = g.ticker(Duration::from_nanos(10)).count();
    let r = g.build();
    assert_eq!(r.topology(), r.print_topology().topology());
}

/// A node error carries the neighbourhood it failed in, `>>>` on the culprit.
/// Legacy attaches the same window to every node error; wingfoil used to
/// attach only `node 3 (TryMap) cycle`, which names the failure without
/// locating it.
#[test]
fn cycle_error_carries_a_topology_window() {
    let g = GraphBuilder::new();
    let count = g.ticker(Duration::from_nanos(10)).count();
    let doubled = count.map(|v: &u64| v * 2);
    let checked = doubled.try_map(|i: &u64| if *i >= 6 { bail!("boom {i}") } else { Ok(*i) });
    let _acc = checked.accumulate();

    let err = g
        .build()
        .run(HISTORICAL, RunFor::Cycles(10))
        .expect_err("try_map must abort the run");
    let msg = format!("{err:#}");

    // The failing node is named, marked, and shown with its upstream edge.
    assert!(
        msg.contains(">>> [03]          TryMap <- [02]"),
        "the culprit is marked and shows its edges: {msg}"
    );
    // Its neighbours are present and unmarked, so the window locates it.
    assert!(
        msg.contains("    [02]       Map <- [01]"),
        "the upstream is in the window: {msg}"
    );
    assert!(
        msg.contains("    [04]             Accumulate <- [03]"),
        "the downstream is in the window: {msg}"
    );
    // The op's own cause still chains.
    assert!(msg.contains("boom 6"), "cause is preserved: {msg}");
}

/// The window is bounded at ±3 nodes and clamps at both ends rather than
/// panicking or dumping the whole graph. Legacy uses the same range.
#[test]
fn error_window_is_bounded_and_clamps() {
    // 12 nodes; the failure sits at index 2 (ticker, count, try_map), so the
    // window clamps at the start and cannot reach the tail.
    let g = GraphBuilder::new();
    let count = g.ticker(Duration::from_nanos(10)).count();
    let checked = count.try_map(|i: &u64| if *i >= 3 { bail!("early") } else { Ok(*i) });
    let mut tail = checked.map(|v: &u64| *v);
    for _ in 0..9 {
        tail = tail.map(|v: &u64| *v);
    }

    let err = g
        .build()
        .run(HISTORICAL, RunFor::Cycles(10))
        .expect_err("try_map must abort the run");
    let msg = format!("{err:#}");

    let window: Vec<&str> = msg
        .lines()
        .filter(|l| l.contains('[') && (l.starts_with(">>> ") || l.starts_with("    [")))
        .collect();
    // focus 2 → nodes 0..=5: clamped to 6 lines, not the 7 an interior node
    // gets, because there is no node -1.
    assert_eq!(6, window.len(), "window should clamp at the start: {msg}");
    assert!(window[0].starts_with("    [00]"), "{msg}");
    assert!(window[2].starts_with(">>> [02]"), "{msg}");
    assert!(
        !msg.contains("[08]"),
        "the window must not run to the tail: {msg}"
    );
}

/// A `start` failure gets the same window as a `cycle` failure — the context
/// is attached at every lifecycle phase, not just the hot one.
#[test]
fn start_error_carries_the_same_window() {
    let g = GraphBuilder::new();
    let count = g.ticker(Duration::from_nanos(10)).count();
    // `finally` runs its closure at teardown; a failing one exercises the
    // cleanup phase's context path.
    let _guard = count.finally(|_last| bail!("teardown boom"));

    let err = g
        .build()
        .run(HISTORICAL, RunFor::Cycles(3))
        .expect_err("a failing finally must surface");
    let msg = format!("{err:#}");
    assert!(msg.contains("teardown boom"), "cause is preserved: {msg}");
    assert!(
        msg.contains(">>> ["),
        "a lifecycle error carries the marked window too: {msg}"
    );
}
