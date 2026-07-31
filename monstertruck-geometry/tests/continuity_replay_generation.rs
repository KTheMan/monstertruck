use std::collections::BTreeMap;

use monstertruck_core::{
    FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingError, TrackingId,
    TrackingSession, TrackingSessionId,
};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{ContinuityOrder, SurfaceBoundary};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, BoundaryEndpoint, ContinuityReplayError,
    ContinuityReplayExecutionError, ContinuitySolveError, ContinuitySolverConfig,
    TrackedSurfaceIdRegistry, execute_boundary_continuity_contracts,
    prepare_boundary_continuity_requests,
};
use monstertruck_geometry::nurbs::contract::{
    BoundaryAlignment, ContinuityContract, ContractId, SurfaceBoundaryRef,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

#[test]
fn replay_resolves_edited_generation_and_rejects_stale_geometry_atomically() {
    let references = [
        semantic("edited-loft", "master"),
        semantic("edited-loft", "middle"),
        semantic("edited-loft", "dependent"),
    ];
    let upstream = contract(
        "z-master-to-middle",
        references[0].clone(),
        references[1].clone(),
    );
    let downstream = contract(
        "a-middle-to-dependent",
        references[1].clone(),
        references[2].clone(),
    );
    let contracts = [downstream.clone(), upstream.clone()];
    let mut session = TrackingSession::new(
        TrackingSessionId::new("generation-edit").expect("the test session ID is valid"),
    );
    let stale_ids = references
        .iter()
        .map(|reference| bind(&mut session, reference))
        .collect::<Vec<_>>();
    let stale_generation = session.generation();

    session
        .advance_generation()
        .expect("the replay generation can advance");
    let current_ids = references
        .iter()
        .map(|reference| bind(&mut session, reference))
        .collect::<Vec<_>>();

    stale_ids
        .iter()
        .zip(&current_ids)
        .for_each(|(stale, current)| {
            assert_eq!(stale.serial(), current.serial());
            assert_eq!(stale.generation(), stale_generation);
            assert_eq!(current.generation(), session.generation());
            assert_ne!(stale, current);
            assert!(matches!(
                session.validate_current(stale),
                Err(TrackingError::StaleGeneration { .. })
            ));
        });

    let resolved = upstream
        .resolve(&session)
        .expect("persistent endpoints resolve after the upstream edit");
    assert_eq!(resolved.first().tracking_id(), &current_ids[0]);
    assert_eq!(resolved.second().tracking_id(), &current_ids[1]);

    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())
        .expect("the default solver configuration is valid");
    let stale_surfaces = BTreeMap::from([
        (stale_ids[0].clone(), plane(0.0, 1.0)),
        (stale_ids[1].clone(), plane(1.0, 1.0)),
        (stale_ids[2].clone(), plane(2.0, 1.0)),
    ]);
    let stale_error = execute_boundary_continuity_contracts(
        &solver,
        &session,
        &stale_surfaces,
        std::slice::from_ref(&upstream),
    )
    .expect_err("stale-generation surface keys cannot enter replay");
    assert!(matches!(
        stale_error,
        ContinuityReplayExecutionError::Preparation(
            ContinuityReplayError::InvalidSurfaceId { source, .. }
        ) if matches!(*source, TrackingError::StaleGeneration { .. })
    ));

    let mut invalid_dependent = plane(2.0, 1.75);
    invalid_dependent.control_point_mut(0, 0).w = 0.0;
    let surfaces = BTreeMap::from([
        (current_ids[0].clone(), plane(0.0, 1.75)),
        (current_ids[1].clone(), plane(1.0, 1.5)),
        (current_ids[2].clone(), invalid_dependent),
    ]);
    let original_surfaces = surfaces.clone();
    let registry = TrackedSurfaceIdRegistry::try_new(&session, surfaces.keys().cloned())
        .expect("all edited surface IDs are current and unique");
    let prepared =
        prepare_boundary_continuity_requests(&session, &registry, std::slice::from_ref(&upstream))
            .expect("the persistent contract prepares against current geometry");
    assert_eq!(prepared[0].first_surface(), &current_ids[0]);
    assert_eq!(prepared[0].second_surface(), &current_ids[1]);

    let successful = execute_boundary_continuity_contracts(
        &solver,
        &session,
        &surfaces,
        std::slice::from_ref(&upstream),
    )
    .expect("the upstream repair replays in the edited generation");
    assert_ne!(successful.surfaces(), &surfaces);
    assert_ne!(
        successful.surfaces()[&current_ids[1]],
        surfaces[&current_ids[1]],
    );
    assert_eq!(
        successful.surfaces()[&current_ids[2]],
        surfaces[&current_ids[2]],
    );
    assert_ne!(
        successful.surfaces()[&current_ids[0]],
        stale_surfaces[&stale_ids[0]],
    );
    assert_eq!(surfaces, original_surfaces);

    let first_error =
        execute_boundary_continuity_contracts(&solver, &session, &surfaces, &contracts)
            .expect_err("the downstream invalid weight rejects the complete batch");
    let second_error =
        execute_boundary_continuity_contracts(&solver, &session, &surfaces, &contracts)
            .expect_err("the same failing batch is deterministic");

    assert_eq!(first_error, second_error);
    assert!(matches!(
        first_error,
        ContinuityReplayExecutionError::Solve {
            contract_id,
            source:
                ContinuitySolveError::NonPositiveWeight {
                    endpoint: BoundaryEndpoint::Second,
                    row: 0,
                    column: 0,
                    weight: 0.0,
                },
        } if contract_id.as_str() == "a-middle-to-dependent"
    ));
    assert_eq!(surfaces, original_surfaces);
}

fn semantic(feature: &str, label: &str) -> SemanticTopologyRef {
    SemanticTopologyRef::new(
        FeatureId::new(feature).expect("the test feature ID is valid"),
        TopologyKind::Face,
        SemanticLabel::new(label).expect("the test semantic label is valid"),
    )
}

fn bind(session: &mut TrackingSession, reference: &SemanticTopologyRef) -> TrackingId {
    let tracking_id = session
        .allocate()
        .expect("the test tracking serial is available");
    session
        .bind(reference.clone(), tracking_id.clone())
        .expect("the semantic reference is unique in this generation");
    tracking_id
}

fn contract(
    id: &str,
    first: SemanticTopologyRef,
    second: SemanticTopologyRef,
) -> ContinuityContract {
    ContinuityContract::new(
        ContractId::new(id).expect("the test contract ID is valid"),
        SurfaceBoundaryRef::new(first, SurfaceBoundary::UEnd)
            .expect("the first endpoint identifies a face"),
        SurfaceBoundaryRef::new(second, SurfaceBoundary::UStart)
            .expect("the second endpoint identifies a face"),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G0,
    )
    .expect("the contract endpoints are distinct")
}

fn plane(x_start: f64, height: f64) -> NurbsSurface<Vector4> {
    let knots = KnotVector::bezier_knot(1);
    NurbsSurface::new(BsplineSurface::new(
        (knots.clone(), knots),
        vec![
            vec![
                Vector4::new(x_start, 0.0, 0.0, 1.0),
                Vector4::new(x_start, height, 0.0, 1.0),
            ],
            vec![
                Vector4::new(x_start + 1.0, 0.0, 0.0, 1.0),
                Vector4::new(x_start + 1.0, height, 0.0, 1.0),
            ],
        ],
    ))
}
