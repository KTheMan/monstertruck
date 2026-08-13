use super::*;

#[test]
fn local_transition_evaluates_nonidentity_cross_fields() {
    let transition = BoundaryTransition::new(
        ContinuityOrder::G2,
        BoundaryAlignment::Aligned,
        vec![0.0],
        vec![vec![2.0, 2.0], vec![6.0, 6.0]],
        vec![2.0_f64.ln(), 2.0_f64.ln()],
        vec![vec![6.0, 6.0]],
    );

    let (seam, cross) = transition
        .mapped_coordinates(0.5, 0.1)
        .expect("the finite local transition evaluates");

    assert!((seam - 0.73).abs() < 1.0e-14);
    assert!((cross - 0.23).abs() < 1.0e-14);
}
