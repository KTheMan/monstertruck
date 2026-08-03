use super::transition::monotone_seam_map;
use super::*;

#[test]
fn transition_layout_partitions_every_order_without_overlap() {
    (0..=4).for_each(|order| {
        let start = 7;
        let field_count = 4;
        let layout = TransitionLayout::try_new(order, field_count, start)
            .expect("small transition layouts are representable");
        let variables = (0..start + layout.variable_count())
            .map(|index| Dual::constant(index as f64))
            .collect::<Vec<_>>();

        assert_eq!(layout.seam_map(&variables).len(), field_count - 1);
        assert_eq!(layout.seam_map(&variables)[0].value(), start as f64);
        if order > 0 {
            assert_eq!(
                layout.alpha(&variables, 1)[0].value(),
                (start + field_count - 1) as f64
            );
            assert_eq!(
                layout.log_beta(&variables)[0].value(),
                (start + field_count - 1 + order * field_count) as f64
            );
            if order > 1 {
                assert_eq!(
                    layout.beta(&variables, order)[field_count - 1].value(),
                    (start + layout.variable_count() - 1) as f64
                );
            }
        }
    });
}

#[test]
fn seam_map_is_endpoint_preserving_and_strictly_monotone() {
    let logs = [
        Dual::constant(-1.0),
        Dual::constant(0.5),
        Dual::constant(-0.25),
    ];
    let values = [0.0, 0.2, 0.5, 0.8, 1.0].map(|parameter| {
        let jet = monotone_seam_map(&logs, TaylorJet::coordinate_r(1, Dual::constant(parameter)));
        (
            jet.coefficient(0, 0)
                .expect("the value coefficient is active")
                .value(),
            jet.coefficient(0, 1)
                .expect("the seam derivative is active")
                .value(),
        )
    });

    assert!(values[0].0.abs() < 1.0e-15);
    assert!((values[4].0 - 1.0).abs() < 1.0e-15);
    assert!(values.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert!(values.iter().all(|(_, derivative)| *derivative > 0.0));
}
