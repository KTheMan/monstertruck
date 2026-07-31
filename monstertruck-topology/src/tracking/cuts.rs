use rustc_hash::FxHashMap as HashMap;

use crate::*;

use super::initialization::{TrackingState, track_edge, track_face};
use super::interface::TrackingResult;

impl<P, C> Edge<P, C> {
    /// Cuts this edge and assigns fresh ordered child identities.
    ///
    /// The children are recorded in oriented parameter-start then
    /// parameter-end order. The parent identity is never copied onto either
    /// child.
    ///
    /// # Errors
    ///
    /// Returns a [`TrackingError`] when the parent is untracked or stale, or
    /// when generated identities cannot be allocated or bound.
    pub fn cut_tracked(
        &self,
        vertex: &Vertex<P>,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<Option<(Self, Self)>>
    where
        P: Clone,
        C: Cut<Point = P> + SearchParameter<CurveParameter, Point = P>,
    {
        let parent = self
            .tracking_id()
            .cloned()
            .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?;
        session.validate_current(&parent)?;
        let Some((mut first, mut second)) = self.cut(vertex) else {
            return Ok(None);
        };
        let mut staged_session = session.clone();
        let mut state = TrackingState::new(&mut staged_session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_edge(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_edge(&mut second, &mut state, &mut vertices, &mut edges)?;
        let first_id = first
            .tracking_id()
            .cloned()
            .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?;
        let second_id = second
            .tracking_id()
            .cloned()
            .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?;
        staged_session.record_lineage(
            OperationKind::Cut,
            LineageRelation::Split,
            parent,
            [first_id, second_id],
        )?;
        *session = staged_session;
        Ok(Some((first, second)))
    }

    /// Cuts this edge at a known parameter and assigns fresh child identities.
    ///
    /// # Errors
    ///
    /// Returns a [`TrackingError`] under the same conditions as
    /// [`cut_tracked`](Self::cut_tracked).
    pub fn cut_with_parameter_tracked(
        &self,
        vertex: &Vertex<P>,
        parameter: f64,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<Option<(Self, Self)>>
    where
        P: Clone + Tolerance,
        C: Cut<Point = P>,
    {
        let parent = self
            .tracking_id()
            .cloned()
            .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?;
        session.validate_current(&parent)?;
        let Some((mut first, mut second)) = self.cut_with_parameter(vertex, parameter) else {
            return Ok(None);
        };
        let mut staged_session = session.clone();
        let mut state = TrackingState::new(&mut staged_session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_edge(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_edge(&mut second, &mut state, &mut vertices, &mut edges)?;
        let children = [
            first
                .tracking_id()
                .cloned()
                .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?,
            second
                .tracking_id()
                .cloned()
                .ok_or(TrackingError::UntrackedTopology(TopologyKind::Edge))?,
        ];
        staged_session.record_lineage(
            OperationKind::Split,
            LineageRelation::Split,
            parent,
            children,
        )?;
        *session = staged_session;
        Ok(Some((first, second)))
    }
}

impl<P, C, S> Face<P, C, S> {
    /// Splits this face by an edge and assigns fresh ordered child identities.
    ///
    /// # Errors
    ///
    /// Returns a [`TrackingError`] when the parent is untracked or stale, or
    /// when generated identities cannot be allocated or bound.
    pub fn cut_by_edge_tracked(
        &self,
        edge: Edge<P, C>,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<Option<(Self, Self)>>
    where
        S: Clone,
    {
        let parent = self
            .tracking_id()
            .cloned()
            .ok_or(TrackingError::UntrackedTopology(TopologyKind::Face))?;
        session.validate_current(&parent)?;
        let Some((mut first, mut second)) = self.cut_by_edge(edge) else {
            return Ok(None);
        };
        let mut staged_session = session.clone();
        let mut state = TrackingState::new(&mut staged_session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_face(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_face(&mut second, &mut state, &mut vertices, &mut edges)?;
        let children = [
            first
                .tracking_id()
                .cloned()
                .ok_or(TrackingError::UntrackedTopology(TopologyKind::Face))?,
            second
                .tracking_id()
                .cloned()
                .ok_or(TrackingError::UntrackedTopology(TopologyKind::Face))?,
        ];
        staged_session.record_lineage(
            OperationKind::Cut,
            LineageRelation::Split,
            parent,
            children,
        )?;
        *session = staged_session;
        Ok(Some((first, second)))
    }
}
