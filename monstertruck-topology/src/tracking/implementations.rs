use rustc_hash::FxHashMap as HashMap;

use crate::*;

use super::initialization::{
    TrackingState, initialize_faces, initialize_transactionally, track_edge, track_vertex,
    unique_ids,
};
use super::interface::{TopologyTracking, TrackingReport, TrackingResult};

impl<P> TopologyTracking for Vertex<P> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        initialize_transactionally(self, session, feature, |vertex, session, feature| {
            let mut state = TrackingState::new(session, feature);
            let mut vertices = HashMap::default();
            track_vertex(vertex, &mut state, &mut vertices)?;
            Ok(state.finish())
        })
    }

    fn tracking_ids(&self) -> Vec<TrackingId> { self.tracking_id().cloned().into_iter().collect() }

    fn into_untracked(mut self) -> Self {
        self.set_tracking_id(None);
        self
    }
}

impl<P, C> TopologyTracking for Edge<P, C> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        initialize_transactionally(self, session, feature, |edge, session, feature| {
            let mut state = TrackingState::new(session, feature);
            let mut vertices = HashMap::default();
            let mut edges = HashMap::default();
            track_edge(edge, &mut state, &mut vertices, &mut edges)?;
            Ok(state.finish())
        })
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        let (front, back) = self.absolute_ends();
        unique_ids(
            [front, back]
                .into_iter()
                .filter_map(|vertex| vertex.tracking_id().cloned())
                .chain(self.tracking_id().cloned()),
        )
    }

    fn into_untracked(mut self) -> Self {
        self.vertices.0 = self.vertices.0.into_untracked();
        self.vertices.1 = self.vertices.1.into_untracked();
        self.set_tracking_id(None);
        self
    }
}

impl<P, C> TopologyTracking for Wire<P, C> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        initialize_transactionally(self, session, feature, |wire, session, feature| {
            let mut state = TrackingState::new(session, feature);
            let mut vertices = HashMap::default();
            let mut edges = HashMap::default();
            wire.edge_iter_mut()
                .try_for_each(|edge| track_edge(edge, &mut state, &mut vertices, &mut edges))?;
            Ok(state.finish())
        })
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        unique_ids(self.edge_iter().flat_map(TopologyTracking::tracking_ids))
    }

    fn into_untracked(mut self) -> Self {
        self.edge_iter_mut().for_each(|edge| {
            edge.set_tracking_id(None);
            edge.vertices.0.set_tracking_id(None);
            edge.vertices.1.set_tracking_id(None);
        });
        self
    }
}

impl<P, C, S> TopologyTracking for Face<P, C, S> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        initialize_transactionally(self, session, feature, |face, session, feature| {
            initialize_faces([face], session, feature)
        })
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        unique_ids(
            self.boundaries()
                .iter()
                .flat_map(Wire::edge_iter)
                .flat_map(|edge| {
                    let (front, back) = edge.absolute_ends();
                    [front, back]
                        .into_iter()
                        .filter_map(|vertex| vertex.tracking_id().cloned())
                        .chain(edge.tracking_id().cloned())
                })
                .chain(self.tracking_id().cloned()),
        )
    }

    fn into_untracked(mut self) -> Self {
        self.boundaries.iter_mut().for_each(|wire| {
            wire.edge_iter_mut().for_each(|edge| {
                edge.set_tracking_id(None);
                edge.vertices.0.set_tracking_id(None);
                edge.vertices.1.set_tracking_id(None);
            });
        });
        self.set_tracking_id(None);
        self
    }
}

impl<P, C, S> TopologyTracking for Shell<P, C, S> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        initialize_transactionally(self, session, feature, |shell, session, feature| {
            initialize_faces(shell.face_iter_mut(), session, feature)
        })
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        unique_ids(self.face_iter().flat_map(TopologyTracking::tracking_ids))
    }

    fn into_untracked(mut self) -> Self {
        self.face_iter_mut().for_each(|face| {
            face.set_tracking_id(None);
            face.boundaries.iter_mut().for_each(|wire| {
                wire.edge_iter_mut().for_each(|edge| {
                    edge.set_tracking_id(None);
                    edge.vertices.0.set_tracking_id(None);
                    edge.vertices.1.set_tracking_id(None);
                });
            });
        });
        self
    }
}

impl<P, C, S> TopologyTracking for Solid<P, C, S> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        let mut staged_topology = Solid {
            boundaries: self.boundaries.clone(),
            id_allocator: self.id_allocator.clone(),
            attributes: self.attributes.clone(),
        };
        let mut staged_session = session.clone();
        let report = initialize_faces(
            staged_topology
                .boundaries
                .iter_mut()
                .flat_map(Shell::face_iter_mut),
            &mut staged_session,
            feature,
        )?;
        *self = staged_topology;
        *session = staged_session;
        Ok(report)
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        unique_ids(self.face_iter().flat_map(TopologyTracking::tracking_ids))
    }

    fn into_untracked(mut self) -> Self {
        self.boundaries.iter_mut().for_each(|shell| {
            shell.face_iter_mut().for_each(|face| {
                face.set_tracking_id(None);
                face.boundaries.iter_mut().for_each(|wire| {
                    wire.edge_iter_mut().for_each(|edge| {
                        edge.set_tracking_id(None);
                        edge.vertices.0.set_tracking_id(None);
                        edge.vertices.1.set_tracking_id(None);
                    });
                });
            });
        });
        self
    }
}
