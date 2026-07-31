use std::fmt::{Display, Formatter};

use crate::*;

/// Result type for controlled topology tracking operations.
pub use monstertruck_core::TrackingResult;

/// Summary of one deterministic tracking initialization pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackingReport {
    pub(super) all_ids: Vec<TrackingId>,
    pub(super) preserved_ids: Vec<TrackingId>,
    pub(super) generated: Vec<SemanticBinding>,
}

impl TrackingReport {
    /// Returns every distinct topology ID in deterministic traversal order.
    pub fn all_ids(&self) -> &[TrackingId] { &self.all_ids }

    /// Returns IDs preserved from an input topology.
    pub fn preserved_ids(&self) -> &[TrackingId] { &self.preserved_ids }

    /// Returns semantic bindings allocated for generated topology.
    pub fn generated(&self) -> &[SemanticBinding] { &self.generated }

    /// Returns generated IDs in deterministic traversal order.
    pub fn generated_ids(&self) -> impl Iterator<Item = &TrackingId> {
        self.generated.iter().map(SemanticBinding::tracking_id)
    }
}

/// A topology value that can be finalized by a tracking session.
///
/// Implementations visit shared vertices and edges once by pointer identity.
/// Existing current-generation IDs are preserved on their first identity.
/// Any copied duplicate on a different identity is a generated element and
/// receives a new ID.
pub trait TopologyTracking {
    /// Assigns current-generation IDs and semantic labels to generated
    /// topology in this operation result.
    ///
    /// `feature` owns generated labels such as `vertex.0000`,
    /// `edge.0000`, and `face.0000`. A fixed traversal and feature identifier
    /// therefore replay to the same generation-local serial sequence.
    ///
    /// # Errors
    ///
    /// Returns a [`TrackingError`] when an existing ID is stale or belongs to
    /// another session, or when a generated semantic label is already bound
    /// inconsistently.
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport>;

    /// Returns distinct assigned IDs in deterministic topology order.
    fn tracking_ids(&self) -> Vec<TrackingId>;

    /// Consumes a newly generated topology layer and removes copied IDs.
    ///
    /// This is an internal operation-construction hook. It does not mutate
    /// the source topology from which the layer was mapped.
    #[doc(hidden)]
    fn into_untracked(self) -> Self;
}

impl Display for TrackingReport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} topology IDs ({} preserved, {} generated)",
            self.all_ids.len(),
            self.preserved_ids.len(),
            self.generated.len(),
        )
    }
}
