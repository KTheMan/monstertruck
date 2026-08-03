//! The deprecated prefix-form `u_*` / `v_*` accessors must keep answering, and
//! must answer IDENTICALLY to the postfix names that replaced them.
//!
//! `u_period` -> `period_u`, `v_period` -> `period_v` (`ParametricSurface`) and
//! `Plane::u_axis` -> `axis_u`, `Plane::v_axis` -> `axis_v` were renamed for
//! `<property>_<direction>` consistency with `derivative_u`, `knot_vector_u` and
//! `cut_u`. The old names remain as `#[deprecated]` forwarders so downstream
//! code keeps compiling; these rows are what stops a forwarder from silently
//! rotting into a different answer than the method it delegates to.
//!
//! A forwarder that returned the wrong value would be worse than a hard rename:
//! callers would get a plausible number with no diagnostic at all.

use monstertruck_geometry::prelude::*;

/// Periodicity through both spellings, on surfaces that are periodic in u only,
/// in v only, and in both -- so a forwarder wired to the wrong direction cannot
/// pass by symmetry.
#[test]
#[allow(deprecated)]
fn deprecated_period_aliases_match_their_replacements() {
    // Torus: periodic in BOTH u and v (2*pi each). Catches a wrong value.
    let torus = Torus::new(Point3::origin(), 3.0, 1.0);
    assert_eq!(torus.u_period(), torus.period_u());
    assert_eq!(torus.v_period(), torus.period_v());
    assert_eq!(torus.period_u(), Some(2.0 * std::f64::consts::PI));
    assert_eq!(torus.period_v(), Some(2.0 * std::f64::consts::PI));

    // Sphere: periodic in v, NOT in u. Catches a forwarder crossed between
    // the two directions, which the torus rows cannot see.
    let sphere = Sphere::new(Point3::origin(), 2.0);
    assert_eq!(sphere.u_period(), sphere.period_u());
    assert_eq!(sphere.v_period(), sphere.period_v());
    assert_eq!(sphere.period_u(), None, "sphere is not periodic in u");
    assert_eq!(sphere.period_v(), Some(2.0 * std::f64::consts::PI));

    // Plane: periodic in neither, i.e. the trait's default arm.
    let plane = Plane::new(
        Point3::origin(),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(plane.u_period(), plane.period_u());
    assert_eq!(plane.v_period(), plane.period_v());
    assert_eq!(plane.period_u(), None);
    assert_eq!(plane.period_v(), None);
}

/// `Plane`'s axis accessors return distinct, non-parallel vectors, so a
/// forwarder pointing at the wrong one is visible rather than coincidental.
#[test]
#[allow(deprecated)]
fn deprecated_plane_axis_aliases_match_their_replacements() {
    let plane = Plane::new(
        Point3::new(1.0, 2.0, 3.0),
        Point3::new(5.0, 2.0, 3.0),
        Point3::new(1.0, 9.0, 3.0),
    );
    assert_eq!(plane.u_axis(), plane.axis_u());
    assert_eq!(plane.v_axis(), plane.axis_v());
    assert_eq!(plane.axis_u(), Vector3::new(4.0, 0.0, 0.0));
    assert_eq!(plane.axis_v(), Vector3::new(0.0, 7.0, 0.0));
    assert_ne!(
        plane.axis_u(),
        plane.axis_v(),
        "the two axes must not be the same vector, or the rows above prove nothing"
    );
}
