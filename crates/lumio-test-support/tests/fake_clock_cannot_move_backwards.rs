use std::time::Duration;

use lumio_platform::{MonotonicClock, Ticks};
use lumio_test_support::{FakeClock, FakeClockError};

#[test]
fn fake_clock_cannot_move_backwards() {
    let clock = FakeClock::new(Ticks::from_nanos(10));
    assert_eq!(clock.now(), Ticks::from_nanos(10));

    assert_eq!(
        clock.set_forward(Ticks::from_nanos(5)),
        Err(FakeClockError::Backward)
    );
    assert_eq!(clock.now(), Ticks::from_nanos(10));

    assert_eq!(clock.set_forward(Ticks::from_nanos(20)), Ok(()));
    assert_eq!(clock.now(), Ticks::from_nanos(20));

    clock.advance(Duration::from_nanos(3));
    assert_eq!(clock.now(), Ticks::from_nanos(23));
}
