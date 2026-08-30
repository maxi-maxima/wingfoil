## Per-hop latency in one process

Latency stamps travel with a value, so the same pipeline can explain where it
spent time during a historical replay or a live run. This example keeps the
whole path in one process: it declares three named stages, performs two small
transformations between them, and prints the two hop distributions when the
run ends.

```rust
latency_stages! {
    pub PipelineLatency { received, normalized, decided }
}

let decisions = g
    .ticker(Duration::from_millis(1))
    .count()
    .map(|n: &u64| Traced::<u64, PipelineLatency>::new(*n))
    .stamp_precise::<pipeline_latency::received>()
    .map(normalize)
    .stamp_precise::<pipeline_latency::normalized>()
    .map(decide)
    .stamp_precise::<pipeline_latency::decided>();

let (_sink, latency) = decisions.latency_report(ReportOutput::Stdout);
```

Run it without feature flags or external services:

```sh
cargo run -p wingfoil --example latency
```

The report has two adjacent hops and one end-to-end row. This is the shape of
a real run; the timing columns are deliberately elided because stamps read the
wall clock even under `HistoricalFrom`, so their values depend on the machine:

```text
latency report (delta from previous stage, nanoseconds):
  stage                            count          min         mean          p50          p99        p99.9          max
  received -> normalized             ...          ...          ...          ...          ...          ...          ...
  normalized -> decided               ...          ...          ...          ...          ...          ...          ...
  received -> decided (end to end)    ...          ...          ...          ...          ...          ...          ...
captured 2 named hops
```

`HistoricalFrom` makes the graph's engine-time schedule reproducible, but
latency measurement intentionally uses wall time: it is measuring how long
the work took, not when the source says the event occurred. The stage names,
their order, the three report rows, and the five source observations stay
fixed; the measured counts and nanosecond figures do not. On a coarse clock,
two precise reads can still collide, in which case the affected row reports a
`same-cycle` note and `count + same-cycle` accounts for all five observations.

`stamp_precise` takes a fresh clock reading at each stage. The cheaper `stamp`
uses one wall-clock snapshot per engine cycle, so stages reached in the same
cycle would be reported as unmeasured rather than as a false zero-duration
hop. Use the precise form for in-process work like this, and the cycle form for
cross-cycle or cross-process boundaries where one read per cycle is enough.

The returned `LatencyHandle` is not print-only. Here it supplies the final hop
count; applications can also read snapshots or expose rolling windows to
metrics and alerting code.
