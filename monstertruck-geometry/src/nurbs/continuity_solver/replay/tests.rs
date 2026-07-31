use super::*;
use crate::base::Vector4;
use crate::nurbs::continuity::{ContinuityOrder, SurfaceBoundary};
use crate::nurbs::contract::{BoundaryAlignment, SurfaceBoundaryRef};
use crate::nurbs::{BsplineSurface, KnotVector, NurbsSurface};
use monstertruck_core::{
    FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingSessionId,
};

fn semantic(feature: &str, label: &str) -> SemanticTopologyRef {
    SemanticTopologyRef::new(
        FeatureId::new(feature).expect("the test feature ID is valid"),
        TopologyKind::Face,
        SemanticLabel::new(label).expect("the test semantic label is valid"),
    )
}

fn contract(
    id: &str,
    first: SemanticTopologyRef,
    second: SemanticTopologyRef,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
) -> ContinuityContract {
    ContinuityContract::new(
        ContractId::new(id).expect("the test contract ID is valid"),
        SurfaceBoundaryRef::new(first, SurfaceBoundary::UEnd)
            .expect("the first endpoint identifies a face"),
        SurfaceBoundaryRef::new(second, SurfaceBoundary::VStart)
            .expect("the second endpoint identifies a face"),
        alignment,
        order,
    )
    .expect("the test contract endpoints are distinct")
}

fn session() -> TrackingSession {
    TrackingSession::new(
        TrackingSessionId::new("continuity-replay").expect("the test tracking-session ID is valid"),
    )
}

fn bind(session: &mut TrackingSession, reference: SemanticTopologyRef) -> TrackingId {
    let tracking_id = session
        .allocate()
        .expect("the test tracking serial is available");
    session
        .bind(reference, tracking_id.clone())
        .expect("the test semantic binding is unique");
    tracking_id
}

fn replay_surfaces() -> (NurbsSurface<Vector4>, NurbsSurface<Vector4>) {
    let knots = KnotVector::bezier_knot(1);
    let first = NurbsSurface::new(BsplineSurface::new(
        (knots.clone(), knots.clone()),
        vec![
            vec![
                Vector4::new(0.0, 0.0, 0.0, 1.0),
                Vector4::new(0.0, 1.0, 0.0, 1.0),
            ],
            vec![
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                Vector4::new(1.0, 1.0, 0.0, 1.0),
            ],
        ],
    ));
    let second = NurbsSurface::new(BsplineSurface::new(
        (knots.clone(), knots),
        vec![
            vec![
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                Vector4::new(2.0, 0.0, 0.0, 1.0),
            ],
            vec![
                Vector4::new(1.0, 1.0, 0.0, 1.0),
                Vector4::new(2.0, 1.0, 0.0, 1.0),
            ],
        ],
    ));
    (first, second)
}

#[test]
fn batch_maps_contracts_in_canonical_id_order() {
    let first_a = semantic("loft", "first-a");
    let second_a = semantic("loft", "second-a");
    let first_b = semantic("blend", "first-b");
    let second_b = semantic("blend", "second-b");
    let contract_b = contract(
        "b-contract",
        first_b.clone(),
        second_b.clone(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G2,
    );
    let contract_a = contract(
        "a-contract",
        first_a.clone(),
        second_a.clone(),
        BoundaryAlignment::Reversed,
        ContinuityOrder::G3,
    );
    let mut session = session();
    let ids = [
        bind(&mut session, first_a),
        bind(&mut session, second_a),
        bind(&mut session, first_b),
        bind(&mut session, second_b),
    ];
    let surfaces = TrackedSurfaceIdRegistry::try_new(&session, ids.clone())
        .expect("all surface IDs are current and unique");

    let prepared =
        prepare_boundary_continuity_requests(&session, &surfaces, &[contract_b, contract_a])
            .expect("every contract endpoint is registered");

    assert_eq!(prepared[0].contract_id().as_str(), "a-contract");
    assert_eq!(
        prepared[0].request(),
        BoundaryContinuityRequest::new(
            SurfaceBoundary::UEnd,
            SurfaceBoundary::VStart,
            BoundaryAlignment::Reversed,
            ContinuityOrder::G3,
        ),
    );
    assert_eq!(prepared[0].first_surface(), &ids[0]);
    assert_eq!(prepared[0].second_surface(), &ids[1]);
    assert_eq!(prepared[1].contract_id().as_str(), "b-contract");
}

#[test]
fn missing_surface_rejects_the_entire_batch() {
    let first = semantic("loft", "first");
    let second = semantic("loft", "second");
    let contract = contract(
        "missing-second",
        first.clone(),
        second.clone(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
    );
    let mut session = session();
    let first_id = bind(&mut session, first);
    let second_id = bind(&mut session, second);
    let surfaces = TrackedSurfaceIdRegistry::try_new(&session, [first_id])
        .expect("the registered surface ID is current");

    let error = prepare_boundary_continuity_requests(&session, &surfaces, &[contract])
        .expect_err("an absent endpoint must reject the batch");

    assert!(matches!(
        error,
        ContinuityReplayError::SurfaceNotRegistered {
            endpoint: BoundaryEndpoint::Second,
            tracking_id,
            ..
        } if tracking_id == second_id
    ));
}

#[test]
fn registry_rejects_duplicate_and_stale_surface_ids() {
    let mut session = session();
    let tracking_id = session
        .allocate()
        .expect("the test tracking serial is available");
    assert_eq!(
        TrackedSurfaceIdRegistry::try_new(&session, [tracking_id.clone(), tracking_id.clone()],),
        Err(ContinuityReplayError::DuplicateSurfaceId {
            tracking_id: tracking_id.clone(),
        }),
    );

    session
        .advance_generation()
        .expect("the test tracking generation is available");
    assert!(matches!(
        TrackedSurfaceIdRegistry::try_new(&session, [tracking_id]),
        Err(ContinuityReplayError::InvalidSurfaceId {
            source,
            ..
        }) if matches!(*source, TrackingError::StaleGeneration { .. }),
    ));
}

#[test]
fn duplicate_contract_ids_are_rejected_before_resolution() {
    let first = semantic("loft", "first");
    let second = semantic("loft", "second");
    let first_contract = contract(
        "duplicate",
        first.clone(),
        second.clone(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G1,
    );
    let second_contract = contract(
        "duplicate",
        first,
        second,
        BoundaryAlignment::Reversed,
        ContinuityOrder::G3,
    );
    let session = session();
    let surfaces =
        TrackedSurfaceIdRegistry::try_new(&session, []).expect("an empty registry is valid");

    assert_eq!(
        prepare_boundary_continuity_requests(
            &session,
            &surfaces,
            &[first_contract, second_contract],
        ),
        Err(ContinuityReplayError::DuplicateContractId {
            contract_id: ContractId::new("duplicate").expect("the test contract ID is valid"),
        }),
    );
}

#[test]
fn replay_executes_contracts_transactionally() {
    let first_ref = semantic("loft", "master");
    let second_ref = semantic("loft", "dependent");
    let contract = contract(
        "g0-replay",
        first_ref.clone(),
        second_ref.clone(),
        BoundaryAlignment::Aligned,
        ContinuityOrder::G0,
    );
    let mut session = session();
    let first_id = bind(&mut session, first_ref);
    let second_id = bind(&mut session, second_ref);
    let (first, second) = replay_surfaces();
    let surfaces = BTreeMap::from([
        (first_id.clone(), first.clone()),
        (second_id.clone(), second.clone()),
    ]);
    let solver = BoundaryContinuitySolver::new(super::super::ContinuitySolverConfig::default())
        .expect("the default solver configuration is valid");

    let replay = execute_boundary_continuity_contracts(&solver, &session, &surfaces, &[contract])
        .expect("the exact tracked boundary contract executes");

    assert_eq!(surfaces.get(&first_id), Some(&first));
    assert_eq!(surfaces.get(&second_id), Some(&second));
    assert_eq!(replay.surfaces(), &surfaces);
    assert_eq!(replay.solves().len(), 1);
    assert_eq!(replay.solves()[0].contract_id().as_str(), "g0-replay");
    assert_eq!(replay.solves()[0].transition().order(), ContinuityOrder::G0,);
    assert_eq!(
        replay.solves()[0]
            .transition()
            .mapped_coordinates(0.25, 0.0),
        Some((0.25, 0.0)),
    );
    assert_eq!(
        replay.solves()[0].report().termination(),
        super::super::ContinuityTermination::Converged
    );
}
