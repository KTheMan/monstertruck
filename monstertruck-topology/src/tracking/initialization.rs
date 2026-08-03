use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

use crate::*;

use super::interface::{TrackingReplacement, TrackingReport, TrackingResult};

pub(super) struct TrackingState<'a> {
    session: &'a mut TrackingSession,
    feature: FeatureId,
    used: HashSet<TrackingId>,
    next_vertex: usize,
    next_edge: usize,
    next_face: usize,
    all_ids: Vec<TrackingId>,
    preserved_ids: Vec<TrackingId>,
    generated: Vec<SemanticBinding>,
    replacements: Vec<TrackingReplacement>,
}

impl<'a> TrackingState<'a> {
    pub(super) fn new(session: &'a mut TrackingSession, feature: FeatureId) -> Self {
        Self {
            session,
            feature,
            used: HashSet::default(),
            next_vertex: 0,
            next_edge: 0,
            next_face: 0,
            all_ids: Vec::new(),
            preserved_ids: Vec::new(),
            generated: Vec::new(),
            replacements: Vec::new(),
        }
    }

    fn next_ordinal(&mut self, kind: TopologyKind) -> usize {
        let counter = match kind {
            TopologyKind::Vertex => &mut self.next_vertex,
            TopologyKind::Edge => &mut self.next_edge,
            TopologyKind::Face => &mut self.next_face,
        };
        let ordinal = *counter;
        *counter += 1;
        ordinal
    }

    fn assign(
        &mut self,
        kind: TopologyKind,
        existing: Option<&TrackingId>,
    ) -> TrackingResult<TrackingId> {
        let ordinal = self.next_ordinal(kind);
        if let Some(existing) = existing {
            if let Some(binding) = self.session.binding_for_tracking_id(existing)?
                && binding.reference().kind() != kind
            {
                return Err(TrackingError::TopologyKindMismatch {
                    expected: kind,
                    actual: binding.reference().kind(),
                });
            }
            if self.used.insert(existing.clone()) {
                self.all_ids.push(existing.clone());
                self.preserved_ids.push(existing.clone());
                return Ok(existing.clone());
            }
        }

        let tracking_id = self.session.allocate()?;
        let label = SemanticLabel::new(format!(
            "{}.{ordinal:04}",
            match kind {
                TopologyKind::Vertex => "vertex",
                TopologyKind::Edge => "edge",
                TopologyKind::Face => "face",
            },
        ))?;
        let reference = SemanticTopologyRef::new(self.feature.clone(), kind, label);
        let binding = self.session.bind(reference, tracking_id.clone())?.clone();
        self.used.insert(tracking_id.clone());
        self.all_ids.push(tracking_id.clone());
        self.generated.push(binding);
        if let Some(original) = existing {
            self.replacements.push(TrackingReplacement {
                original: original.clone(),
                replacement: tracking_id.clone(),
            });
        }
        Ok(tracking_id)
    }

    pub(super) fn finish(self) -> TrackingReport {
        TrackingReport {
            all_ids: self.all_ids,
            preserved_ids: self.preserved_ids,
            generated: self.generated,
            replacements: self.replacements,
        }
    }
}

pub(super) fn track_vertex<P>(
    vertex: &mut Vertex<P>,
    state: &mut TrackingState<'_>,
    vertices: &mut HashMap<VertexId<P>, TrackingId>,
) -> TrackingResult<()> {
    if let Some(tracking_id) = vertices.get(&vertex.id()) {
        vertex.set_tracking_id(Some(tracking_id.clone()));
        return Ok(());
    }
    let tracking_id = state.assign(TopologyKind::Vertex, vertex.tracking_id())?;
    vertices.insert(vertex.id(), tracking_id.clone());
    vertex.set_tracking_id(Some(tracking_id));
    Ok(())
}

pub(super) fn track_edge<P, C>(
    edge: &mut Edge<P, C>,
    state: &mut TrackingState<'_>,
    vertices: &mut HashMap<VertexId<P>, TrackingId>,
    edges: &mut HashMap<EdgeId<C>, TrackingId>,
) -> TrackingResult<()> {
    track_vertex(&mut edge.vertices.0, state, vertices)?;
    track_vertex(&mut edge.vertices.1, state, vertices)?;
    if let Some(tracking_id) = edges.get(&edge.id()) {
        edge.set_tracking_id(Some(tracking_id.clone()));
        return Ok(());
    }
    let tracking_id = state.assign(TopologyKind::Edge, edge.tracking_id())?;
    edges.insert(edge.id(), tracking_id.clone());
    edge.set_tracking_id(Some(tracking_id));
    Ok(())
}

pub(super) fn track_face<P, C, S>(
    face: &mut Face<P, C, S>,
    state: &mut TrackingState<'_>,
    vertices: &mut HashMap<VertexId<P>, TrackingId>,
    edges: &mut HashMap<EdgeId<C>, TrackingId>,
) -> TrackingResult<()> {
    face.boundaries
        .iter_mut()
        .flat_map(Wire::edge_iter_mut)
        .try_for_each(|edge| track_edge(edge, state, vertices, edges))?;
    let tracking_id = state.assign(TopologyKind::Face, face.tracking_id())?;
    face.set_tracking_id(Some(tracking_id));
    Ok(())
}

pub(super) fn initialize_faces<'a, P: 'a, C: 'a, S: 'a>(
    faces: impl IntoIterator<Item = &'a mut Face<P, C, S>>,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> TrackingResult<TrackingReport> {
    let mut state = TrackingState::new(session, feature);
    let mut vertices = HashMap::default();
    let mut edges = HashMap::default();
    faces
        .into_iter()
        .try_for_each(|face| track_face(face, &mut state, &mut vertices, &mut edges))?;
    Ok(state.finish())
}

pub(super) fn unique_ids(ids: impl IntoIterator<Item = TrackingId>) -> Vec<TrackingId> {
    let mut seen = HashSet::default();
    ids.into_iter()
        .filter(|tracking_id| seen.insert(tracking_id.clone()))
        .collect()
}

pub(super) fn initialize_transactionally<T: Clone>(
    topology: &mut T,
    session: &mut TrackingSession,
    feature: FeatureId,
    initialize: impl FnOnce(&mut T, &mut TrackingSession, FeatureId) -> TrackingResult<TrackingReport>,
) -> TrackingResult<TrackingReport> {
    let mut staged_topology = topology.clone();
    let mut staged_session = session.clone();
    let report = initialize(&mut staged_topology, &mut staged_session, feature)?;
    *topology = staged_topology;
    *session = staged_session;
    Ok(report)
}
