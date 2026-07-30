//! Transactional continuity-request preparation for parametric replay.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use monstertruck_core::{TrackingError, TrackingId, TrackingSession};

use super::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, BoundaryEndpoint, ContinuitySolveError,
    ContinuitySolveReport,
};
use crate::base::Vector4;
use crate::nurbs::NurbsSurface;
use crate::nurbs::contract::{ContinuityContract, ContractError, ContractId};

/// Current-generation IDs of surfaces available to continuity replay.
///
/// Build the registry from the replayed topology before preparing contracts.
/// Construction validates every [`TrackingId`] against the supplied
/// [`TrackingSession`] and rejects duplicate IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedSurfaceIdRegistry {
    ids: BTreeSet<TrackingId>,
}

impl TrackedSurfaceIdRegistry {
    /// Builds a validated surface-ID registry for the current generation.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuityReplayError::InvalidSurfaceId`] when an ID is not
    /// current and [`ContinuityReplayError::DuplicateSurfaceId`] when the
    /// topology supplies the same ID more than once.
    pub fn try_new(
        session: &TrackingSession,
        tracking_ids: impl IntoIterator<Item = TrackingId>,
    ) -> Result<Self, ContinuityReplayError> {
        let mut ids = tracking_ids
            .into_iter()
            .map(|tracking_id| {
                session
                    .validate_current(&tracking_id)
                    .map(|()| tracking_id.clone())
                    .map_err(|source| ContinuityReplayError::InvalidSurfaceId {
                        tracking_id,
                        source: Box::new(source),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        ids.sort();
        if let Some(tracking_id) = ids
            .windows(2)
            .find(|pair| pair[0] == pair[1])
            .map(|pair| pair[0].clone())
        {
            Err(ContinuityReplayError::DuplicateSurfaceId { tracking_id })
        } else {
            Ok(Self {
                ids: ids.into_iter().collect(),
            })
        }
    }

    /// Returns whether a current-generation surface ID is registered.
    #[inline(always)]
    pub fn contains(&self, tracking_id: &TrackingId) -> bool {
        self.ids.contains(tracking_id)
    }

    /// Returns the number of registered surface IDs.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns whether no surface IDs are registered.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Iterates over registered IDs in canonical tracking-ID order.
    #[inline(always)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &TrackingId> {
        self.ids.iter()
    }
}

/// One resolved contract ready to pair with two NURBS surfaces.
///
/// This runtime value deliberately retains generation-scoped [`TrackingId`]s
/// and therefore must not be serialized as a persistent contract.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ResolvedBoundaryContinuityRequest {
    contract_id: ContractId,
    first_surface: TrackingId,
    second_surface: TrackingId,
    request: BoundaryContinuityRequest,
}

impl ResolvedBoundaryContinuityRequest {
    /// Returns the persistent contract identifier.
    #[inline(always)]
    pub const fn contract_id(&self) -> &ContractId {
        &self.contract_id
    }

    /// Returns the current-generation fixed-surface ID.
    #[inline(always)]
    pub const fn first_surface(&self) -> &TrackingId {
        &self.first_surface
    }

    /// Returns the current-generation optimized-surface ID.
    #[inline(always)]
    pub const fn second_surface(&self) -> &TrackingId {
        &self.second_surface
    }

    /// Returns the surface-only solver request.
    #[inline(always)]
    pub const fn request(&self) -> BoundaryContinuityRequest {
        self.request
    }

    fn from_contract(
        contract: &ContinuityContract,
        session: &TrackingSession,
        surfaces: &TrackedSurfaceIdRegistry,
    ) -> Result<Self, ContinuityReplayError> {
        let resolved = contract.resolve(session).map_err(|source| {
            ContinuityReplayError::ContractResolution {
                contract_id: contract.id().clone(),
                source: Box::new(source),
            }
        })?;
        [
            (BoundaryEndpoint::First, resolved.first().tracking_id()),
            (BoundaryEndpoint::Second, resolved.second().tracking_id()),
        ]
        .into_iter()
        .try_for_each(|(endpoint, tracking_id)| {
            session.validate_current(tracking_id).map_err(|source| {
                ContinuityReplayError::ResolvedSurfaceIdInvalid {
                    contract_id: contract.id().clone(),
                    endpoint,
                    tracking_id: tracking_id.clone(),
                    source: Box::new(source),
                }
            })?;
            if surfaces.contains(tracking_id) {
                Ok(())
            } else {
                Err(ContinuityReplayError::SurfaceNotRegistered {
                    contract_id: contract.id().clone(),
                    endpoint,
                    tracking_id: tracking_id.clone(),
                })
            }
        })?;
        Ok(Self {
            contract_id: contract.id().clone(),
            first_surface: resolved.first().tracking_id().clone(),
            second_surface: resolved.second().tracking_id().clone(),
            request: BoundaryContinuityRequest::new(
                resolved.first().boundary(),
                resolved.second().boundary(),
                resolved.alignment(),
                resolved.order(),
            ),
        })
    }
}

/// Resolves and validates a complete continuity-contract batch.
///
/// Contracts are prepared in canonical [`ContractId`] order. The function is
/// transactional: it borrows all inputs immutably and returns no request batch
/// unless every contract resolves and both of its surface IDs occur in
/// `surfaces`.
///
/// # Errors
///
/// Returns a [`ContinuityReplayError`] for duplicate contract IDs, failed
/// semantic resolution, stale resolved IDs, or IDs absent from the replayed
/// surface registry.
pub fn prepare_boundary_continuity_requests(
    session: &TrackingSession,
    surfaces: &TrackedSurfaceIdRegistry,
    contracts: &[ContinuityContract],
) -> Result<Vec<ResolvedBoundaryContinuityRequest>, ContinuityReplayError> {
    let mut ordered = contracts.iter().collect::<Vec<_>>();
    ordered.sort_by(|first, second| first.id().cmp(second.id()));
    if let Some(contract_id) = ordered
        .windows(2)
        .find(|pair| pair[0].id() == pair[1].id())
        .map(|pair| pair[0].id().clone())
    {
        Err(ContinuityReplayError::DuplicateContractId { contract_id })
    } else {
        ordered
            .into_iter()
            .map(|contract| {
                ResolvedBoundaryContinuityRequest::from_contract(contract, session, surfaces)
            })
            .collect()
    }
}

/// Diagnostics for one contract executed during replay.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuityContractSolve {
    contract_id: ContractId,
    report: ContinuitySolveReport,
}

impl ContinuityContractSolve {
    /// Returns the persistent contract identifier.
    pub const fn contract_id(&self) -> &ContractId {
        &self.contract_id
    }

    /// Returns deterministic solver diagnostics.
    pub const fn report(&self) -> &ContinuitySolveReport {
        &self.report
    }
}

/// Owned result of a transactional continuity-contract replay.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuityReplaySolution {
    surfaces: BTreeMap<TrackingId, NurbsSurface<Vector4>>,
    solves: Vec<ContinuityContractSolve>,
}

impl ContinuityReplaySolution {
    /// Returns the solved surfaces keyed by current-generation tracking ID.
    pub const fn surfaces(&self) -> &BTreeMap<TrackingId, NurbsSurface<Vector4>> {
        &self.surfaces
    }

    /// Returns contract diagnostics in deterministic dependency order.
    pub fn solves(&self) -> &[ContinuityContractSolve] {
        &self.solves
    }

    /// Consumes the replay result.
    pub fn into_parts(
        self,
    ) -> (
        BTreeMap<TrackingId, NurbsSurface<Vector4>>,
        Vec<ContinuityContractSolve>,
    ) {
        (self.surfaces, self.solves)
    }
}

/// Resolves and executes a complete boundary-continuity contract batch.
///
/// The input map is borrowed and cloned before solving. Contracts execute in a
/// deterministic topological order so a solved dependent surface becomes the
/// master for its downstream contracts. No result is returned unless every
/// contract succeeds. Multiple contracts that optimize the same surface,
/// same-surface contracts, and dependency cycles are rejected because they
/// require a future coupled multi-boundary solve.
///
/// # Errors
///
/// Returns a typed preparation, dependency-graph, or solver failure. The input
/// surfaces remain unchanged for every failure.
pub fn execute_boundary_continuity_contracts(
    solver: &BoundaryContinuitySolver,
    session: &TrackingSession,
    surfaces: &BTreeMap<TrackingId, NurbsSurface<Vector4>>,
    contracts: &[ContinuityContract],
) -> Result<ContinuityReplaySolution, ContinuityReplayExecutionError> {
    let registry = TrackedSurfaceIdRegistry::try_new(session, surfaces.keys().cloned())?;
    let prepared = prepare_boundary_continuity_requests(session, &registry, contracts)?;
    let execution_order = contract_execution_order(&prepared)?;
    let mut solved_surfaces = surfaces.clone();
    let mut solves = Vec::with_capacity(prepared.len());

    execution_order.into_iter().try_for_each(|index| {
        let resolved = &prepared[index];
        let first = solved_surfaces
            .get(resolved.first_surface())
            .ok_or_else(|| ContinuityReplayExecutionError::SurfaceGeometryMissing {
                contract_id: resolved.contract_id().clone(),
                endpoint: BoundaryEndpoint::First,
                tracking_id: resolved.first_surface().clone(),
            })?
            .clone();
        let second = solved_surfaces
            .get(resolved.second_surface())
            .ok_or_else(|| ContinuityReplayExecutionError::SurfaceGeometryMissing {
                contract_id: resolved.contract_id().clone(),
                endpoint: BoundaryEndpoint::Second,
                tracking_id: resolved.second_surface().clone(),
            })?
            .clone();
        let solution = solver
            .solve(&first, &second, resolved.request())
            .map_err(|source| ContinuityReplayExecutionError::Solve {
                contract_id: resolved.contract_id().clone(),
                source,
            })?;
        let (_, solved_second, report) = solution.into_parts();
        solved_surfaces.insert(resolved.second_surface().clone(), solved_second);
        solves.push(ContinuityContractSolve {
            contract_id: resolved.contract_id().clone(),
            report,
        });
        Ok::<(), ContinuityReplayExecutionError>(())
    })?;

    Ok(ContinuityReplaySolution {
        surfaces: solved_surfaces,
        solves,
    })
}

fn contract_execution_order(
    prepared: &[ResolvedBoundaryContinuityRequest],
) -> Result<Vec<usize>, ContinuityReplayExecutionError> {
    let mut optimized_surfaces = BTreeMap::new();
    prepared
        .iter()
        .enumerate()
        .try_for_each(|(index, request)| {
            if request.first_surface() == request.second_surface() {
                return Err(ContinuityReplayExecutionError::SameSurfaceContract {
                    contract_id: request.contract_id().clone(),
                    tracking_id: request.first_surface().clone(),
                });
            }
            if let Some(previous) =
                optimized_surfaces.insert(request.second_surface().clone(), index)
            {
                Err(ContinuityReplayExecutionError::CoupledOptimizedSurface {
                    tracking_id: request.second_surface().clone(),
                    first_contract: prepared[previous].contract_id().clone(),
                    second_contract: request.contract_id().clone(),
                })
            } else {
                Ok(())
            }
        })?;

    let mut outgoing = vec![Vec::new(); prepared.len()];
    let mut incoming = vec![0_usize; prepared.len()];
    prepared
        .iter()
        .enumerate()
        .for_each(|(downstream, request)| {
            if let Some(&upstream) = optimized_surfaces.get(request.first_surface()) {
                outgoing[upstream].push(downstream);
                incoming[downstream] += 1;
            }
        });
    outgoing.iter_mut().for_each(|indices| {
        indices.sort_by(|&first, &second| {
            prepared[first]
                .contract_id()
                .cmp(prepared[second].contract_id())
        });
    });

    let mut ready = prepared
        .iter()
        .enumerate()
        .filter(|(index, _)| incoming[*index] == 0)
        .map(|(index, request)| (request.contract_id().clone(), index))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(prepared.len());
    while let Some((_, index)) = ready.pop_first() {
        order.push(index);
        outgoing[index].iter().copied().for_each(|downstream| {
            incoming[downstream] -= 1;
            if incoming[downstream] == 0 {
                ready.insert((prepared[downstream].contract_id().clone(), downstream));
            }
        });
    }
    if order.len() == prepared.len() {
        Ok(order)
    } else {
        Err(ContinuityReplayExecutionError::DependencyCycle {
            contracts: prepared
                .iter()
                .enumerate()
                .filter(|(index, _)| incoming[*index] > 0)
                .map(|(_, request)| request.contract_id().clone())
                .collect(),
        })
    }
}

/// Failure to execute a continuity-contract batch transactionally.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ContinuityReplayExecutionError {
    /// Contract preparation failed before geometry was touched.
    #[error(transparent)]
    Preparation(#[from] ContinuityReplayError),
    /// One contract resolves both endpoints to the same surface.
    #[error("continuity contract `{contract_id}` targets surface `{tracking_id}` twice")]
    SameSurfaceContract {
        /// Persistent contract identifier.
        contract_id: ContractId,
        /// Repeated current-generation surface.
        tracking_id: TrackingId,
    },
    /// Independent solves would optimize the same surface more than once.
    #[error(
        "surface `{tracking_id}` is optimized by both `{first_contract}` and `{second_contract}`"
    )]
    CoupledOptimizedSurface {
        /// Shared optimized surface.
        tracking_id: TrackingId,
        /// First persistent contract.
        first_contract: ContractId,
        /// Second persistent contract.
        second_contract: ContractId,
    },
    /// A prepared endpoint was absent from the cloned geometry map.
    #[error(
        "{endpoint:?} surface `{tracking_id}` for continuity contract \
         `{contract_id}` has no replay geometry"
    )]
    SurfaceGeometryMissing {
        /// Persistent contract identifier.
        contract_id: ContractId,
        /// Missing contract endpoint.
        endpoint: BoundaryEndpoint,
        /// Missing current-generation surface.
        tracking_id: TrackingId,
    },
    /// The directed master-to-optimized surface graph contains a cycle.
    #[error("continuity contract dependency graph contains a cycle")]
    DependencyCycle {
        /// Contracts participating in the remaining cyclic graph.
        contracts: Vec<ContractId>,
    },
    /// A prepared contract failed its geometric solve.
    #[error("continuity contract `{contract_id}` failed to solve")]
    Solve {
        /// Persistent contract identifier.
        contract_id: ContractId,
        /// Typed geometric solver failure.
        source: ContinuitySolveError,
    },
}

/// Failure to prepare continuity solver requests from replayed contracts.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContinuityReplayError {
    /// A topology surface ID is not current in the supplied session.
    #[error("surface tracking ID `{tracking_id}` is invalid for the current replay generation")]
    InvalidSurfaceId {
        /// Invalid topology ID.
        tracking_id: TrackingId,
        /// Typed tracking validation failure.
        source: Box<TrackingError>,
    },
    /// A topology traversal supplied one surface ID more than once.
    #[error("surface tracking ID `{tracking_id}` occurs more than once in the replayed topology")]
    DuplicateSurfaceId {
        /// Repeated topology ID.
        tracking_id: TrackingId,
    },
    /// A request batch contains duplicate persistent contract IDs.
    #[error("continuity contract ID `{contract_id}` occurs more than once")]
    DuplicateContractId {
        /// Repeated persistent contract ID.
        contract_id: ContractId,
    },
    /// A persistent contract could not resolve in the current generation.
    #[error("continuity contract `{contract_id}` could not be resolved")]
    ContractResolution {
        /// Persistent contract ID.
        contract_id: ContractId,
        /// Typed contract-resolution failure.
        source: Box<ContractError>,
    },
    /// A resolved contract endpoint is not current in the supplied session.
    #[error(
        "{endpoint:?} surface `{tracking_id}` for continuity contract \
         `{contract_id}` is invalid for the current replay generation"
    )]
    ResolvedSurfaceIdInvalid {
        /// Persistent contract ID.
        contract_id: ContractId,
        /// Contract endpoint with the invalid ID.
        endpoint: BoundaryEndpoint,
        /// Invalid resolved topology ID.
        tracking_id: TrackingId,
        /// Typed tracking validation failure.
        source: Box<TrackingError>,
    },
    /// A resolved contract endpoint is absent from the replayed surfaces.
    #[error(
        "{endpoint:?} surface `{tracking_id}` for continuity contract \
         `{contract_id}` is absent from the replayed topology"
    )]
    SurfaceNotRegistered {
        /// Persistent contract ID.
        contract_id: ContractId,
        /// Contract endpoint absent from the registry.
        endpoint: BoundaryEndpoint,
        /// Missing current-generation topology ID.
        tracking_id: TrackingId,
    },
}

#[cfg(test)]
mod tests {
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
            TrackingSessionId::new("continuity-replay")
                .expect("the test tracking-session ID is valid"),
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

        let replay =
            execute_boundary_continuity_contracts(&solver, &session, &surfaces, &[contract])
                .expect("the exact tracked boundary contract executes");

        assert_eq!(surfaces.get(&first_id), Some(&first));
        assert_eq!(surfaces.get(&second_id), Some(&second));
        assert_eq!(replay.surfaces(), &surfaces);
        assert_eq!(replay.solves().len(), 1);
        assert_eq!(replay.solves()[0].contract_id().as_str(), "g0-replay");
        assert_eq!(
            replay.solves()[0].report().termination(),
            super::super::ContinuityTermination::Converged
        );
    }
}
