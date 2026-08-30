#![doc = include_str!("./README.md")]
//!
//! ```sh
//! cargo run -p wingfoil --example latency
//! ```

use std::time::Duration;

use wingfoil::latency::{LatencyReportOps, LatencyStreamOps, ReportOutput, Traced, latency_stages};
use wingfoil::prelude::*;
use wingfoil::{NanoTime, RunFor, RunMode};

latency_stages! {
    pub PipelineLatency {
        received,
        normalized,
        decided,
    }
}

fn main() -> anyhow::Result<()> {
    let g = GraphBuilder::new();

    let decisions = g
        .ticker(Duration::from_millis(1))
        .count()
        .map(|n: &u64| Traced::<u64, PipelineLatency>::new(*n))
        .stamp_precise::<pipeline_latency::received>()
        .map(|sample: &Traced<u64, PipelineLatency>| {
            Traced::with_latency(sample.payload * 10, sample.latency)
        })
        .stamp_precise::<pipeline_latency::normalized>()
        .map(|sample: &Traced<u64, PipelineLatency>| {
            Traced::with_latency(sample.payload >= 20, sample.latency)
        })
        .stamp_precise::<pipeline_latency::decided>();

    // The report sink observes every traced value and prints once, when the
    // bounded run tears down. The handle is also available for programmatic
    // snapshots, alerts, or windowed metrics.
    let (_report_sink, latency) = decisions.latency_report(ReportOutput::Stdout);

    let mut runner = g.build();
    runner.run(RunMode::HistoricalFrom(NanoTime::ZERO), RunFor::Cycles(5))?;

    println!("captured {} named hops", latency.hops().len());

    Ok(())
}
