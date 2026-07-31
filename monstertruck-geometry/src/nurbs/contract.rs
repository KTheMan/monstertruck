//! Persistent continuity contracts for parametric replay.
//!
//! A [`crate::nurbs::contract::ContinuityContract`] stores semantic
//! face-boundary references rather than generation-specific topology
//! identifiers. After a feature graph is replayed,
//! [`crate::nurbs::contract::ContinuityContract::resolve`] binds those
//! references to the current [`monstertruck_core::TrackingId`] values in a
//! [`monstertruck_core::TrackingSession`]. No solver state,
//! cached geometry, or control-point data enters the serialized contract.
//!
//! G0 is the established solver target. G1 through G3 currently have
//! procedural evidence and imported workflow execution, while independent
//! higher-order certification remains pending. G4 remains an experimental
//! target. Every order uses the same versioned contract representation so
//! later solver work does not require a schema migration.

pub use super::continuity::BoundaryAlignment;
use super::continuity::{ContinuityOrder, SurfaceBoundary};
use monstertruck_core::{
    SemanticTopologyRef, TopologyKind, TrackingError, TrackingId, TrackingSession,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

/// Current serialized continuity-contract schema.
pub const CONTINUITY_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Stable, non-empty identifier for a [`ContinuityContract`].
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ContractId(String);

impl ContractId {
    /// Creates a validated contract identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::InvalidId`] when `value` is empty or contains
    /// only whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(ContractError::InvalidId)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the identifier text.
    #[inline(always)]
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Display for ContractId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ContractId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Persistent reference to one boundary of a semantic surface face.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub struct SurfaceBoundaryRef {
    topology: SemanticTopologyRef,
    boundary: SurfaceBoundary,
}

impl SurfaceBoundaryRef {
    /// Creates a face-boundary reference.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::WrongTopologyKind`] when `topology` does not
    /// identify a face.
    pub fn new(
        topology: SemanticTopologyRef,
        boundary: SurfaceBoundary,
    ) -> Result<Self, ContractError> {
        if topology.kind() == TopologyKind::Face {
            Ok(Self { topology, boundary })
        } else {
            Err(ContractError::WrongTopologyKind {
                actual: topology.kind(),
            })
        }
    }

    /// Returns the semantic face reference.
    #[inline(always)]
    pub const fn topology(&self) -> &SemanticTopologyRef { &self.topology }

    /// Returns the selected parametric boundary.
    #[inline(always)]
    pub const fn boundary(&self) -> SurfaceBoundary { self.boundary }

    fn resolve(
        &self,
        session: &TrackingSession,
    ) -> Result<ResolvedSurfaceBoundaryRef, ContractError> {
        Ok(ResolvedSurfaceBoundaryRef {
            tracking_id: session.resolve(&self.topology)?.clone(),
            boundary: self.boundary,
        })
    }
}

impl<'de> Deserialize<'de> for SurfaceBoundaryRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        #[derive(Deserialize)]
        struct SerializedSurfaceBoundaryRef {
            topology: SemanticTopologyRef,
            boundary: SurfaceBoundary,
        }

        let value = SerializedSurfaceBoundaryRef::deserialize(deserializer)?;
        Self::new(value.topology, value.boundary).map_err(serde::de::Error::custom)
    }
}

/// Persistent high-order continuity request between two surface boundaries.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub struct ContinuityContract {
    schema_version: u16,
    id: ContractId,
    first: SurfaceBoundaryRef,
    second: SurfaceBoundaryRef,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
}

impl ContinuityContract {
    /// Creates a version-one continuity contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use monstertruck_core::{
    ///     FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind,
    /// };
    /// use monstertruck_geometry::nurbs::continuity::{
    ///     BoundaryAlignment, ContinuityOrder, SurfaceBoundary,
    /// };
    /// use monstertruck_geometry::nurbs::contract::{
    ///     ContinuityContract, ContractId, SurfaceBoundaryRef,
    /// };
    ///
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
    /// let contract = ContinuityContract::new(
    ///     ContractId::new("hood-to-fender").expect("the contract identifier is valid"),
    ///     boundary("hood", SurfaceBoundary::UEnd),
    ///     boundary("fender", SurfaceBoundary::UStart),
    ///     BoundaryAlignment::Aligned,
    ///     ContinuityOrder::G2,
    /// )?;
    ///
    /// assert_eq!(contract.order(), ContinuityOrder::G2);
    /// # Ok::<(), monstertruck_geometry::nurbs::contract::ContractError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::IdenticalEndpoints`] when both endpoint
    /// references select the same semantic face boundary.
    pub fn new(
        id: ContractId,
        first: SurfaceBoundaryRef,
        second: SurfaceBoundaryRef,
        alignment: BoundaryAlignment,
        order: ContinuityOrder,
    ) -> Result<Self, ContractError> {
        if first == second {
            Err(ContractError::IdenticalEndpoints)
        } else {
            Ok(Self {
                schema_version: CONTINUITY_CONTRACT_SCHEMA_VERSION,
                id,
                first,
                second,
                alignment,
                order,
            })
        }
    }

    /// Returns the serialized schema version.
    #[inline(always)]
    pub const fn schema_version(&self) -> u16 { self.schema_version }

    /// Returns the stable contract identifier.
    #[inline(always)]
    pub const fn id(&self) -> &ContractId { &self.id }

    /// Returns the first persistent boundary reference.
    #[inline(always)]
    pub const fn first(&self) -> &SurfaceBoundaryRef { &self.first }

    /// Returns the second persistent boundary reference.
    #[inline(always)]
    pub const fn second(&self) -> &SurfaceBoundaryRef { &self.second }

    /// Returns the endpoint orientation relationship.
    #[inline(always)]
    pub const fn alignment(&self) -> BoundaryAlignment { self.alignment }

    /// Returns the requested geometric-continuity order.
    #[inline(always)]
    pub const fn order(&self) -> ContinuityOrder { self.order }

    /// Resolves both semantic endpoints in the current tracking generation.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError::Tracking`] when either semantic endpoint is
    /// unavailable in `session`.
    pub fn resolve(
        &self,
        session: &TrackingSession,
    ) -> Result<ResolvedContinuityContract, ContractError> {
        Ok(ResolvedContinuityContract {
            id: self.id.clone(),
            first: self.first.resolve(session)?,
            second: self.second.resolve(session)?,
            alignment: self.alignment,
            order: self.order,
        })
    }
}

impl<'de> Deserialize<'de> for ContinuityContract {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        #[derive(Deserialize)]
        struct SerializedContinuityContract {
            schema_version: u16,
            id: ContractId,
            first: SurfaceBoundaryRef,
            second: SurfaceBoundaryRef,
            alignment: BoundaryAlignment,
            order: ContinuityOrder,
        }

        let value = SerializedContinuityContract::deserialize(deserializer)?;
        if value.schema_version != CONTINUITY_CONTRACT_SCHEMA_VERSION {
            Err(serde::de::Error::custom(
                ContractError::UnsupportedSchemaVersion(value.schema_version),
            ))
        } else {
            Self::new(
                value.id,
                value.first,
                value.second,
                value.alignment,
                value.order,
            )
            .map_err(serde::de::Error::custom)
        }
    }
}

/// Current-generation resolution of one surface boundary.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ResolvedSurfaceBoundaryRef {
    tracking_id: TrackingId,
    boundary: SurfaceBoundary,
}

impl ResolvedSurfaceBoundaryRef {
    /// Returns the current generation-specific topology identifier.
    #[inline(always)]
    pub const fn tracking_id(&self) -> &TrackingId { &self.tracking_id }

    /// Returns the selected parametric boundary.
    #[inline(always)]
    pub const fn boundary(&self) -> SurfaceBoundary { self.boundary }
}

/// Current-generation resolution of a [`ContinuityContract`].
///
/// This type intentionally does not implement [`Serialize`]. Persist the
/// source [`ContinuityContract`] so replay never stores stale tracking IDs.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ResolvedContinuityContract {
    id: ContractId,
    first: ResolvedSurfaceBoundaryRef,
    second: ResolvedSurfaceBoundaryRef,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
}

impl ResolvedContinuityContract {
    /// Returns the stable contract identifier.
    #[inline(always)]
    pub const fn id(&self) -> &ContractId { &self.id }

    /// Returns the first current-generation boundary.
    #[inline(always)]
    pub const fn first(&self) -> &ResolvedSurfaceBoundaryRef { &self.first }

    /// Returns the second current-generation boundary.
    #[inline(always)]
    pub const fn second(&self) -> &ResolvedSurfaceBoundaryRef { &self.second }

    /// Returns the endpoint orientation relationship.
    #[inline(always)]
    pub const fn alignment(&self) -> BoundaryAlignment { self.alignment }

    /// Returns the requested geometric-continuity order.
    #[inline(always)]
    pub const fn order(&self) -> ContinuityOrder { self.order }
}

/// Failure to create, deserialize, or resolve a continuity contract.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContractError {
    /// A contract identifier was empty or contained only whitespace.
    #[error("continuity contract id must not be empty")]
    InvalidId,
    /// A boundary reference identified topology other than a face.
    #[error("surface boundary reference requires Face topology, got {actual:?}")]
    WrongTopologyKind {
        /// Actual topology kind supplied by the caller.
        actual: TopologyKind,
    },
    /// Both contract endpoints selected the same semantic face boundary.
    #[error("continuity contract endpoints must be distinct")]
    IdenticalEndpoints,
    /// The serialized contract uses an unsupported schema.
    #[error("unsupported continuity contract schema version {0}")]
    UnsupportedSchemaVersion(u16),
    /// A persistent semantic reference could not be resolved.
    #[error(transparent)]
    Tracking(#[from] TrackingError),
}

#[cfg(test)]
mod tests {
    use super::super::continuity::ContinuityMaturity;
    use super::*;
    use monstertruck_core::{
        FeatureId, SemanticLabel, SemanticTopologyRef, TrackingGeneration, TrackingSessionId,
    };

    fn semantic(kind: TopologyKind, label: &str) -> SemanticTopologyRef {
        SemanticTopologyRef::new(
            FeatureId::new("blend-feature").expect("the test feature identifier is valid"),
            kind,
            SemanticLabel::new(label).expect("the test semantic label is valid"),
        )
    }

    fn boundary(label: &str, boundary: SurfaceBoundary) -> SurfaceBoundaryRef {
        SurfaceBoundaryRef::new(semantic(TopologyKind::Face, label), boundary)
            .expect("the test semantic reference identifies a face")
    }

    fn contract(order: ContinuityOrder) -> ContinuityContract {
        ContinuityContract::new(
            ContractId::new("hood-to-fender").expect("the test contract identifier is valid"),
            boundary("hood", SurfaceBoundary::UEnd),
            boundary("fender", SurfaceBoundary::UStart),
            BoundaryAlignment::Reversed,
            order,
        )
        .expect("the test endpoints are distinct")
    }

    fn session() -> TrackingSession {
        TrackingSession::new(
            TrackingSessionId::new("styling-session")
                .expect("the test session identifier is valid"),
        )
    }

    fn bind_contract_endpoints(
        session: &mut TrackingSession,
        contract: &ContinuityContract,
    ) -> (TrackingId, TrackingId) {
        let first = session.allocate().expect("the serial range is available");
        let second = session.allocate().expect("the serial range is available");
        session
            .bind(contract.first().topology().clone(), first.clone())
            .expect("the first semantic binding is valid");
        session
            .bind(contract.second().topology().clone(), second.clone())
            .expect("the second semantic binding is valid");
        (first, second)
    }

    #[test]
    fn g3_contract_round_trip_persists_only_semantic_references() {
        let contract = contract(ContinuityOrder::G3);
        let json = serde_json::to_string(&contract).expect("contract serialization succeeds");
        let restored: ContinuityContract =
            serde_json::from_str(&json).expect("contract deserialization succeeds");

        assert_eq!(restored, contract);
        assert_eq!(
            restored.schema_version(),
            CONTINUITY_CONTRACT_SCHEMA_VERSION,
        );
        assert!(!json.contains("tracking_id"));
        assert!(!json.contains("solver"));
        assert!(!json.contains("cache"));
    }

    #[test]
    fn g4_contract_round_trip_and_resolution_remain_experimental() {
        let contract = contract(ContinuityOrder::G4);
        let json = serde_json::to_string(&contract).expect("contract serialization succeeds");
        let restored: ContinuityContract =
            serde_json::from_str(&json).expect("contract deserialization succeeds");
        let mut session = session();
        let (first, second) = bind_contract_endpoints(&mut session, &restored);
        let resolved = restored
            .resolve(&session)
            .expect("both semantic endpoints are bound");

        assert_eq!(
            restored.order().maturity(),
            ContinuityMaturity::Experimental
        );
        assert_eq!(resolved.first().tracking_id(), &first);
        assert_eq!(resolved.second().tracking_id(), &second);
        assert_eq!(resolved.order(), ContinuityOrder::G4);
    }

    #[test]
    fn construction_rejects_invalid_ids_wrong_kinds_and_identical_endpoints() {
        assert_eq!(ContractId::new(" \t "), Err(ContractError::InvalidId));
        assert_eq!(
            SurfaceBoundaryRef::new(
                semantic(TopologyKind::Edge, "seam"),
                SurfaceBoundary::VStart,
            ),
            Err(ContractError::WrongTopologyKind {
                actual: TopologyKind::Edge,
            }),
        );

        let endpoint = boundary("shared", SurfaceBoundary::VEnd);
        assert_eq!(
            ContinuityContract::new(
                ContractId::new("invalid-pair").expect("the test contract identifier is valid"),
                endpoint.clone(),
                endpoint,
                BoundaryAlignment::Aligned,
                ContinuityOrder::G3,
            ),
            Err(ContractError::IdenticalEndpoints),
        );
    }

    #[test]
    fn resolution_preserves_typed_tracking_failures() {
        let error = contract(ContinuityOrder::G3)
            .resolve(&session())
            .expect_err("unbound semantic endpoints must not resolve");

        assert!(matches!(
            error,
            ContractError::Tracking(TrackingError::UnknownSemanticReference(_)),
        ));
    }

    #[test]
    fn replay_resolves_semantic_endpoints_to_the_new_generation() {
        let contract = contract(ContinuityOrder::G3);
        let mut session = session();
        let (old_first, old_second) = bind_contract_endpoints(&mut session, &contract);
        let old_resolved = contract
            .resolve(&session)
            .expect("the initial semantic bindings resolve");

        session
            .advance_generation()
            .expect("the generation range is available");
        let (new_first, new_second) = bind_contract_endpoints(&mut session, &contract);
        let replayed = contract
            .resolve(&session)
            .expect("the replayed semantic bindings resolve");

        assert_eq!(
            old_resolved.first().tracking_id().generation(),
            TrackingGeneration::INITIAL,
        );
        assert_eq!(
            replayed.first().tracking_id().generation(),
            TrackingGeneration::new(1),
        );
        assert_eq!(old_first.serial(), new_first.serial());
        assert_eq!(old_second.serial(), new_second.serial());
        assert_ne!(old_first, new_first);
        assert_ne!(old_second, new_second);
        assert_eq!(replayed.first().tracking_id(), &new_first);
        assert_eq!(replayed.second().tracking_id(), &new_second);
    }
}
