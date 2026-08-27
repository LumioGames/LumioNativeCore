//! StdMonotonicClock `now()` delta-distribution smoke.
//!
//! `criterion` / `[[bench]]` are not approved (`EXTERNAL_ALLOWLIST` is empty).
//! This file is compiled as a `cfg(test)` module from `lib.rs`.

use crate::{MonotonicClock, StdMonotonicClock};

const SAMPLES: usize = 1024;

struct ClockDeltaDistribution {
    count: usize,
    min_nanos: u128,
    max_nanos: u128,
}

fn sample_delta_distribution(clock: &StdMonotonicClock, n: usize) -> ClockDeltaDistribution {
    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        samples.push(clock.now());
    }

    let mut min_nanos = u128::MAX;
    let mut max_nanos = u128::MIN;
    let mut count = 0;
    for pair in samples.windows(2) {
        let nanos = pair[1].saturating_duration_since(pair[0]).as_nanos();
        min_nanos = min_nanos.min(nanos);
        max_nanos = max_nanos.max(nanos);
        count += 1;
    }

    ClockDeltaDistribution {
        count,
        min_nanos,
        max_nanos,
    }
}

#[test]
fn clock_benchmark_reports_distribution() {
    let clock = StdMonotonicClock::new();
    let stats = sample_delta_distribution(&clock, SAMPLES);
    assert_eq!(stats.count, SAMPLES - 1);
    assert!(
        stats.min_nanos <= stats.max_nanos,
        "undefined distribution: min_nanos={} max_nanos={}",
        stats.min_nanos,
        stats.max_nanos,
    );
}
