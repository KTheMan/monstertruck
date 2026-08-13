use super::*;

#[test]
fn nodes_are_symmetric_and_ordered() {
    let nodes = gauss_legendre_nodes(5).expect("a positive order has nodes");
    assert!(nodes.windows(2).all(|pair| pair[0] < pair[1]));
    nodes
        .iter()
        .zip(nodes.iter().rev())
        .for_each(|(first, second)| assert!((first + second).abs() < 1.0e-15));
}

#[test]
fn samples_follow_nonzero_knot_spans() {
    let knots = KnotVector::from(vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
    let samples = seam_samples(&knots, 2, 4, 2).expect("the domain has two spans");

    assert_eq!(samples.len(), 7);
    assert_eq!(samples[0], 0.0);
    assert_eq!(samples[3], 0.5);
    assert_eq!(samples[6], 1.0);
}

#[test]
fn validation_samples_use_an_independent_midpoint_grid() {
    let knots = KnotVector::from(vec![0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0]);
    let samples = seam_validation_samples(&knots, 2, 4, 4).expect("the domain has two spans");

    assert_eq!(samples.len(), 8);
    assert_eq!(samples[0], 0.0625);
    assert_eq!(samples[7], 0.9375);
}
