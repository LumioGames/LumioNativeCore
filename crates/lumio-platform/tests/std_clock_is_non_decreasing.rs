use std::sync::Arc;
use std::thread;

use lumio_platform::{MonotonicClock, StdMonotonicClock, Ticks};

const SAMPLES: usize = 4096;

#[test]
fn std_clock_is_non_decreasing() {
    let clock = StdMonotonicClock::new();
    let single = sample_sequence(&clock, SAMPLES);
    assert_non_decreasing(&single);

    let clock = Arc::new(StdMonotonicClock::new());
    let left = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || sample_sequence(&clock, SAMPLES))
    };
    let right = {
        let clock = Arc::clone(&clock);
        thread::spawn(move || sample_sequence(&clock, SAMPLES))
    };

    let left = left.join().expect("left clock thread");
    let right = right.join().expect("right clock thread");
    assert_non_decreasing(&left);
    assert_non_decreasing(&right);
}

fn sample_sequence(clock: &StdMonotonicClock, n: usize) -> Vec<Ticks> {
    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        samples.push(clock.now());
    }
    samples
}

fn assert_non_decreasing(samples: &[Ticks]) {
    for pair in samples.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "clock went backwards: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}
