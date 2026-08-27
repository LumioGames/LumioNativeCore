use core::time::Duration;
use lumio_platform::{Deadline, Ticks};

#[test]
fn ticks_checked_add_does_not_wrap() {
    assert_eq!(
        Ticks::from_nanos(u64::MAX).checked_add(Duration::from_nanos(1)),
        None
    );
    assert_eq!(
        Ticks::from_nanos(u64::MAX - 5).checked_add(Duration::from_nanos(5)),
        Some(Ticks::from_nanos(u64::MAX))
    );
    assert_eq!(
        Ticks::from_nanos(u64::MAX).saturating_add(Duration::from_nanos(1)),
        Ticks::from_nanos(u64::MAX)
    );

    let t = Ticks::from_nanos(7);
    assert!(Deadline::at(t).is_expired(t));
    assert!(!Deadline::NONE.is_expired(t));
    assert!(!Deadline::NONE.is_expired(Ticks::ZERO));
    assert!(!Deadline::NONE.is_expired(Ticks::from_nanos(u64::MAX)));
}
