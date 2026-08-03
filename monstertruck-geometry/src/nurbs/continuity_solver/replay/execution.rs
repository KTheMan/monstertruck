//! Transactional dependency ordering and contract execution.

use super::*;

/// Diagnostics for one contract executed during replay.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuityContractSolve {
    contract_id: ContractId,
    transition: BoundaryTransition,
    report: ContinuitySolveReport,
}

impl ContinuityContractSolve {
    /// Returns the persistent contract identifier.
    pub const fn contract_id(&self) -> &ContractId { &self.contract_id }

    /// Returns the accepted local [`BoundaryTransition`] for this contract.
    pub const fn transition(&self) -> &BoundaryTransition { &self.transition }

    /// Returns deterministic solver diagnostics.
    pub const fn report(&self) -> &ContinuitySolveReport { &self.report }
}

/// Owned result of a transactional continuity-contract replay.
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuityReplaySolution {
    surfaces: BTreeMap<TrackingId, NurbsSurface<Vector4>>,
    solves: Vec<ContinuityContractSolve>,
}

impl ContinuityReplaySolution {
    /// Returns the solved surfaces keyed by current-generation tracking ID.
    pub const fn surfaces(&self) -> &BTreeMap<TrackingId, NurbsSurface<Vector4>> { &self.surfaces }

    /// Returns contract diagnostics in deterministic dependency order.
    pub fn solves(&self) -> &[ContinuityContractSolve] { &self.solves }

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
            })?;
        let second = solved_surfaces
            .get(resolved.second_surface())
            .ok_or_else(|| ContinuityReplayExecutionError::SurfaceGeometryMissing {
                contract_id: resolved.contract_id().clone(),
                endpoint: BoundaryEndpoint::Second,
                tracking_id: resolved.second_surface().clone(),
            })?;
        let solution = solver
            .solve(first, second, resolved.request())
            .map_err(|source| ContinuityReplayExecutionError::Solve {
                contract_id: resolved.contract_id().clone(),
                source,
            })?;
        let (_, solved_second, transition, report) = solution.into_parts_with_transition();
        solved_surfaces.insert(resolved.second_surface().clone(), solved_second);
        solves.push(ContinuityContractSolve {
            contract_id: resolved.contract_id().clone(),
            transition,
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
