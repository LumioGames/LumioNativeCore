//! T-job-02 / R-00105: CancellationSource/View is monotonic; views share the flag.

use lumio_job::CancellationSource;

#[test]
fn cancel_is_monotonic() {
    let source = CancellationSource::new();
    let view = source.view();
    let view_clone = view.clone();
    let other_view = source.view();

    assert!(!source.is_cancelled());
    assert!(!view.is_cancelled());
    assert!(!view_clone.is_cancelled());
    assert!(!other_view.is_cancelled());

    source.cancel();
    assert!(source.is_cancelled());
    assert!(view.is_cancelled());
    assert!(view_clone.is_cancelled());
    assert!(other_view.is_cancelled());

    source.cancel();
    assert!(source.is_cancelled());
    assert!(view.is_cancelled());
    assert!(view_clone.is_cancelled());
    assert!(other_view.is_cancelled());
}
