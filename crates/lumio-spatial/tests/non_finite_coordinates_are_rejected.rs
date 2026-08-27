//! T-spatial-01 / R-00107: Point3/Aabb3 constructors reject non-finite coordinates.

use lumio_kernel::error::{ErrorCategory, KernelError};
use lumio_spatial::{Aabb3, Point3};

fn assert_invalid_argument<T: std::fmt::Debug>(result: Result<T, KernelError>, what: &str) {
    let err = result.expect_err(what);
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

#[test]
fn non_finite_coordinates_are_rejected() {
    let origin = Point3::new(0.0, 0.0, 0.0).expect("finite origin");
    let extent = Point3::new(1.0, 2.0, 3.0).expect("finite point");
    assert_eq!(
        origin,
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0
        }
    );
    assert_eq!(extent.x, 1.0);
    assert_eq!(extent.y, 2.0);
    assert_eq!(extent.z, 3.0);

    let box_ok = Aabb3::new(origin, extent).expect("finite AABB");
    assert_eq!(box_ok.min, origin);
    assert_eq!(box_ok.max, extent);

    let degenerate = Aabb3::new(extent, extent).expect("min==max is a valid finite AABB");
    assert_eq!(degenerate.min, extent);
    assert_eq!(degenerate.max, extent);

    assert_invalid_argument(Point3::new(f32::NAN, 0.0, 0.0), "NaN x");
    assert_invalid_argument(Point3::new(0.0, f32::NAN, 0.0), "NaN y");
    assert_invalid_argument(Point3::new(0.0, 0.0, f32::NAN), "NaN z");

    assert_invalid_argument(Point3::new(f32::INFINITY, 0.0, 0.0), "+Inf x");
    assert_invalid_argument(Point3::new(0.0, f32::INFINITY, 0.0), "+Inf y");
    assert_invalid_argument(Point3::new(0.0, 0.0, f32::INFINITY), "+Inf z");
    assert_invalid_argument(Point3::new(f32::NEG_INFINITY, 0.0, 0.0), "-Inf x");
    assert_invalid_argument(Point3::new(0.0, f32::NEG_INFINITY, 0.0), "-Inf y");
    assert_invalid_argument(Point3::new(0.0, 0.0, f32::NEG_INFINITY), "-Inf z");

    let inverted_min = Point3::new(1.0, 0.0, 0.0).expect("finite inverted min");
    let inverted_max = Point3::new(0.0, 1.0, 1.0).expect("finite inverted max");
    assert_invalid_argument(
        Aabb3::new(inverted_min, inverted_max),
        "min.x > max.x must fail",
    );

    assert_invalid_argument(
        Aabb3::new(
            Point3 {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
            },
            extent,
        ),
        "AABB NaN min.x",
    );
    assert_invalid_argument(
        Aabb3::new(
            origin,
            Point3 {
                x: 1.0,
                y: f32::INFINITY,
                z: 1.0,
            },
        ),
        "AABB +Inf max.y",
    );
}
