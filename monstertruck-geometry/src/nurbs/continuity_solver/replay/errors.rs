//! Typed failures for continuity-contract preparation and execution.

use super::*;

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
