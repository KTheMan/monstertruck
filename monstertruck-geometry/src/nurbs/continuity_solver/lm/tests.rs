use super::*;
use crate::nurbs::continuity::{BoundaryAlignment, BoundarySide, ContinuityOrder};

#[test]
fn independent_validation_can_veto_collocation_convergence() {
    let config = ContinuitySolverConfig::default();
    let collocation = [OrderResidual::new(
        ContinuityOrder::G0,
        1.0e-12,
        1.0e-12,
        0,
        false,
        0,
        0,
    )];
    let validation = [OrderResidual::new(
        ContinuityOrder::G0,
        2.0e-8,
        2.0e-8,
        4,
        false,
        0,
        0,
    )];
    let merged = merge_residuals(&collocation, &validation);

    assert!(tolerances_met(
        &collocation,
        BoundaryContinuityRequest::new(
            BoundarySide::MaxU,
            BoundarySide::MinU,
            BoundaryAlignment::Aligned,
            ContinuityOrder::G0,
        ),
        &config,
    ));
    assert!(!tolerances_met(
        &merged,
        BoundaryContinuityRequest::new(
            BoundarySide::MaxU,
            BoundarySide::MinU,
            BoundaryAlignment::Aligned,
            ContinuityOrder::G0,
        ),
        &config,
    ));
    assert!(merged[0].is_validation_sample());
    assert_eq!(merged[0].worst_sample(), 4);
}
