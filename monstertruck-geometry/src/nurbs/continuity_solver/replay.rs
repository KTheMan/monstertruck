//! Transactional continuity-request preparation for parametric replay.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use monstertruck_core::{TrackingError, TrackingId, TrackingSession};

use super::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, BoundaryEndpoint, BoundaryTransition,
    ContinuitySolveError, ContinuitySolveReport,
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
    pub fn contains(&self, tracking_id: &TrackingId) -> bool { self.ids.contains(tracking_id) }

    /// Returns the number of registered surface IDs.
    #[inline(always)]
    pub fn len(&self) -> usize { self.ids.len() }

    /// Returns whether no surface IDs are registered.
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.ids.is_empty() }

    /// Iterates over registered IDs in canonical tracking-ID order.
    #[inline(always)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &TrackingId> { self.ids.iter() }
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
    pub const fn contract_id(&self) -> &ContractId { &self.contract_id }

    /// Returns the current-generation fixed-surface ID.
    #[inline(always)]
    pub const fn first_surface(&self) -> &TrackingId { &self.first_surface }

    /// Returns the current-generation optimized-surface ID.
    #[inline(always)]
    pub const fn second_surface(&self) -> &TrackingId { &self.second_surface }

    /// Returns the surface-only solver request.
    #[inline(always)]
    pub const fn request(&self) -> BoundaryContinuityRequest { self.request }

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
///
/// # Examples
///
/// ```
/// use monstertruck_core::{
///     FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingSession,
///     TrackingSessionId,
/// };
/// use monstertruck_geometry::nurbs::continuity::{
///     BoundaryAlignment, ContinuityOrder, SurfaceBoundary,
/// };
/// use monstertruck_geometry::nurbs::continuity_solver::{
///     TrackedSurfaceIdRegistry, prepare_boundary_continuity_requests,
/// };
/// use monstertruck_geometry::nurbs::contract::{
///     ContinuityContract, ContractId, SurfaceBoundaryRef,
/// };
///
/// let mut session = TrackingSession::new(
///     TrackingSessionId::new("replay").expect("the session identifier is valid"),
/// );
/// let feature = FeatureId::new("body").expect("the feature identifier is valid");
/// let boundary = |label, edge| {
///     let topology = SemanticTopologyRef::new(
///         feature.clone(),
///         TopologyKind::Face,
///         SemanticLabel::new(label).expect("the semantic label is valid"),
///     );
///     SurfaceBoundaryRef::new(topology, edge)
///         .expect("the semantic reference identifies a face")
/// };
/// let first = boundary("hood", SurfaceBoundary::UEnd);
/// let second = boundary("fender", SurfaceBoundary::UStart);
/// let contract = ContinuityContract::new(
///     ContractId::new("hood-to-fender").expect("the contract identifier is valid"),
///     first.clone(),
///     second.clone(),
///     BoundaryAlignment::Aligned,
///     ContinuityOrder::G1,
/// )?;
/// let first_id = session.allocate()?;
/// let second_id = session.allocate()?;
/// session.bind(first.topology().clone(), first_id.clone())?;
/// session.bind(second.topology().clone(), second_id.clone())?;
/// let surfaces = TrackedSurfaceIdRegistry::try_new(
///     &session,
///     [first_id.clone(), second_id.clone()],
/// )?;
/// let prepared = prepare_boundary_continuity_requests(&session, &surfaces, &[contract])?;
///
/// assert_eq!(prepared[0].first_surface(), &first_id);
/// assert_eq!(prepared[0].second_surface(), &second_id);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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

mod errors;
mod execution;

pub use errors::{ContinuityReplayError, ContinuityReplayExecutionError};
pub use execution::{
    ContinuityContractSolve, ContinuityReplaySolution, execute_boundary_continuity_contracts,
};

#[cfg(test)]
mod tests;
