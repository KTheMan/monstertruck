//! Controlled assignment of immutable, session-scoped topology identities.
//!
//! [`StableId`] remains the serialized, solid-local compatibility key.
//! [`TrackingId`] adds the session and replay generation needed by a
//! parametric feature graph. Tracking IDs have no public setter: callers hand
//! a newly built operation result to [`TopologyTracking::initialize_tracking`]
//! before publishing it.
//!
//! Operation policy is contextual:
//!
//! | Operation | Identity policy | Lineage |
//! | --- | --- | --- |
//! | map / rotate | preserve the one-to-one ID | `Preserved` |
//! | sweep / extrude / revolve | preserve embedded source topology and assign fresh IDs to generated topology | `Generated` |
//! | cut / split | assign distinct fresh IDs to ordered children | `Split` |
//!
//! A sweep internally maps its source to construct another layer. The
//! initializer detects copied IDs on distinct pointer identities and reseeds
//! those generated copies. This avoids giving the base and ceiling the same
//! identity while still allowing a standalone map to preserve identity.

use crate::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::fmt::{Display, Formatter};

/// Result type for controlled topology tracking operations.
pub type TrackingResult<T> = std::result::Result<T, TrackingError>;

/// Summary of one deterministic tracking initialization pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackingReport {
    all_ids: Vec<TrackingId>,
    preserved_ids: Vec<TrackingId>,
    generated: Vec<SemanticBinding>,
}

impl TrackingReport {
    /// Returns every distinct topology ID in deterministic traversal order.
    pub fn all_ids(&self) -> &[TrackingId] {
        &self.all_ids
    }

    /// Returns IDs preserved from an input topology.
    pub fn preserved_ids(&self) -> &[TrackingId] {
        &self.preserved_ids
    }

    /// Returns semantic bindings allocated for generated topology.
    pub fn generated(&self) -> &[SemanticBinding] {
        &self.generated
    }

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

struct TrackingState<'a> {
    session: &'a mut TrackingSession,
    feature: FeatureId,
    used: HashSet<TrackingId>,
    next_vertex: usize,
    next_edge: usize,
    next_face: usize,
    all_ids: Vec<TrackingId>,
    preserved_ids: Vec<TrackingId>,
    generated: Vec<SemanticBinding>,
}

impl<'a> TrackingState<'a> {
    fn new(session: &'a mut TrackingSession, feature: FeatureId) -> Self {
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
            self.session.validate_current(existing)?;
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
        Ok(tracking_id)
    }

    fn finish(self) -> TrackingReport {
        TrackingReport {
            all_ids: self.all_ids,
            preserved_ids: self.preserved_ids,
            generated: self.generated,
        }
    }
}

fn track_vertex<P>(
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

fn track_edge<P, C>(
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

fn track_face<P, C, S>(
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

fn initialize_faces<'a, P: 'a, C: 'a, S: 'a>(
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

fn unique_ids(ids: impl IntoIterator<Item = TrackingId>) -> Vec<TrackingId> {
    let mut seen = HashSet::default();
    ids.into_iter()
        .filter(|tracking_id| seen.insert(tracking_id.clone()))
        .collect()
}

impl<P> TopologyTracking for Vertex<P> {
    fn initialize_tracking(
        &mut self,
        session: &mut TrackingSession,
        feature: FeatureId,
    ) -> TrackingResult<TrackingReport> {
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        track_vertex(self, &mut state, &mut vertices)?;
        Ok(state.finish())
    }

    fn tracking_ids(&self) -> Vec<TrackingId> {
        self.tracking_id().cloned().into_iter().collect()
    }

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
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_edge(self, &mut state, &mut vertices, &mut edges)?;
        Ok(state.finish())
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
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        self.edge_iter_mut()
            .try_for_each(|edge| track_edge(edge, &mut state, &mut vertices, &mut edges))?;
        Ok(state.finish())
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
        initialize_faces([self], session, feature)
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
        initialize_faces(self.face_iter_mut(), session, feature)
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
        initialize_faces(
            self.boundaries.iter_mut().flat_map(Shell::face_iter_mut),
            session,
            feature,
        )
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
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_edge(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_edge(&mut second, &mut state, &mut vertices, &mut edges)?;
        let first_id = first.tracking_id().cloned().expect("the child was tracked");
        let second_id = second
            .tracking_id()
            .cloned()
            .expect("the child was tracked");
        session.record_lineage(
            OperationKind::Cut,
            LineageRelation::Split,
            parent,
            [first_id, second_id],
        )?;
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
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_edge(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_edge(&mut second, &mut state, &mut vertices, &mut edges)?;
        let children = [
            first.tracking_id().cloned().expect("the child was tracked"),
            second
                .tracking_id()
                .cloned()
                .expect("the child was tracked"),
        ];
        session.record_lineage(
            OperationKind::Split,
            LineageRelation::Split,
            parent,
            children,
        )?;
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
        let mut state = TrackingState::new(session, feature);
        let mut vertices = HashMap::default();
        let mut edges = HashMap::default();
        track_face(&mut first, &mut state, &mut vertices, &mut edges)?;
        track_face(&mut second, &mut state, &mut vertices, &mut edges)?;
        let children = [
            first.tracking_id().cloned().expect("the child was tracked"),
            second
                .tracking_id()
                .cloned()
                .expect("the child was tracked"),
        ];
        session.record_lineage(OperationKind::Cut, LineageRelation::Split, parent, children)?;
        Ok(Some((first, second)))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(value: &str) -> FeatureId {
        FeatureId::new(value).expect("the test feature identifier is valid")
    }

    fn session() -> TrackingSession {
        TrackingSession::new(
            TrackingSessionId::new("topology-tests").expect("the test session identifier is valid"),
        )
    }

    fn triangle() -> Face<(), (), ()> {
        let vertices = Vertex::from_points([(), (), ()]);
        Face::new(
            vec![Wire::from([
                Edge::new(&vertices[0], &vertices[1], ()),
                Edge::new(&vertices[1], &vertices[2], ()),
                Edge::new(&vertices[2], &vertices[0], ()),
            ])],
            (),
        )
    }

    fn square() -> (Face<(), (), ()>, Vec<Vertex<()>>) {
        let vertices = Vertex::from_points([(), (), (), ()]);
        let face = Face::new(
            vec![Wire::from([
                Edge::new(&vertices[0], &vertices[1], ()),
                Edge::new(&vertices[1], &vertices[2], ()),
                Edge::new(&vertices[2], &vertices[3], ()),
                Edge::new(&vertices[3], &vertices[0], ()),
            ])],
            (),
        );
        (face, vertices)
    }

    #[test]
    fn initialization_is_unique_and_shared_uses_agree() {
        let mut face = triangle();
        let mut session = session();
        let report = face
            .initialize_tracking(&mut session, feature("triangle"))
            .expect("tracking initialization succeeds");

        assert_eq!(report.all_ids().len(), 7);
        assert_eq!(report.generated().len(), 7);
        assert_eq!(
            report.all_ids().iter().collect::<HashSet<_>>().len(),
            report.all_ids().len(),
        );
        assert_eq!(
            face.boundaries()[0][0].back().tracking_id(),
            face.boundaries()[0][1].front().tracking_id(),
        );
    }

    #[test]
    fn mapped_face_preserves_one_to_one_tracking() {
        let mut face = triangle();
        let mut session = session();
        face.initialize_tracking(&mut session, feature("source"))
            .expect("source tracking succeeds");
        let source_ids = face.tracking_ids();
        let mapped = face.mapped(|_| (), |_| (), |_| ());

        assert_eq!(mapped.tracking_ids(), source_ids);
    }

    #[test]
    fn one_session_allocates_unique_ids_across_multiple_results() {
        let mut first = Shell::from([triangle()]);
        let mut second = Shell::from([triangle()]);
        let mut session = session();
        first
            .initialize_tracking(&mut session, feature("first"))
            .expect("first tracking initialization succeeds");
        second
            .initialize_tracking(&mut session, feature("second"))
            .expect("second tracking initialization succeeds");
        let first_ids: HashSet<_> = first.tracking_ids().into_iter().collect();
        let second_ids: HashSet<_> = second.tracking_ids().into_iter().collect();

        assert!(first_ids.is_disjoint(&second_ids));
    }

    #[test]
    fn edge_split_assigns_distinct_children_and_ordered_lineage() {
        #[derive(Clone)]
        struct Segment;

        impl ParametricCurve for Segment {
            type Point = f64;
            type Vector = f64;
            fn evaluate(&self, parameter: f64) -> f64 {
                parameter
            }
            fn derivative(&self, _: f64) -> f64 {
                1.0
            }
            fn derivative_2(&self, _: f64) -> f64 {
                0.0
            }
            fn derivative_n(&self, order: usize, parameter: f64) -> f64 {
                match order {
                    0 => self.evaluate(parameter),
                    1 => 1.0,
                    _ => 0.0,
                }
            }
            fn parameter_range(&self) -> ParameterRange {
                (
                    std::ops::Bound::Included(0.0),
                    std::ops::Bound::Included(1.0),
                )
            }
        }

        impl BoundedCurve for Segment {}

        impl Cut for Segment {
            fn cut(&mut self, _: f64) -> Self {
                Self
            }
        }

        let vertices = Vertex::from_points([0.0, 1.0]);
        let mut edge = Edge::new(&vertices[0], &vertices[1], Segment);
        let mut session = session();
        edge.initialize_tracking(&mut session, feature("source"))
            .expect("source tracking succeeds");
        let parent = edge.tracking_id().cloned().expect("the parent is tracked");
        let split_vertex = Vertex::new(0.5);
        let (first, second) = edge
            .cut_with_parameter_tracked(&split_vertex, 0.5, &mut session, feature("split"))
            .expect("tracking succeeds")
            .expect("the parameter is interior");
        let first_id = first.tracking_id().expect("the first child is tracked");
        let second_id = second.tracking_id().expect("the second child is tracked");

        assert_ne!(first_id, second_id);
        assert_ne!(first_id, &parent);
        assert_ne!(second_id, &parent);
        let event = session.lineage().last().expect("lineage was recorded");
        assert_eq!(event.parent(), &parent);
        assert_eq!(event.children(), &[first_id.clone(), second_id.clone()]);
        assert_eq!(event.relation(), LineageRelation::Split);
    }

    #[test]
    fn face_cut_assigns_two_fresh_faces_and_split_lineage() {
        let (mut face, vertices) = square();
        let mut session = session();
        face.initialize_tracking(&mut session, feature("source-face"))
            .expect("source tracking succeeds");
        let parent = face.tracking_id().cloned().expect("the parent is tracked");
        let diagonal = Edge::new(&vertices[0], &vertices[2], ());
        let (first, second) = face
            .cut_by_edge_tracked(diagonal, &mut session, feature("face-cut"))
            .expect("tracking succeeds")
            .expect("the diagonal splits the square");
        let first_id = first.tracking_id().expect("the first face is tracked");
        let second_id = second.tracking_id().expect("the second face is tracked");

        assert_ne!(first_id, second_id);
        assert_ne!(first_id, &parent);
        assert_ne!(second_id, &parent);
        let event = session.lineage().last().expect("lineage was recorded");
        assert_eq!(event.parent(), &parent);
        assert_eq!(event.children(), &[first_id.clone(), second_id.clone()]);
        assert_eq!(event.relation(), LineageRelation::Split);
    }

    #[test]
    fn tracking_ids_survive_compressed_topology_roundtrip() {
        let mut shell = Shell::from([triangle()]);
        let mut session = session();
        shell
            .initialize_tracking(&mut session, feature("serialized"))
            .expect("tracking initialization succeeds");
        let expected = shell.tracking_ids();
        let extracted =
            Shell::extract_tracked(shell.compress_tracked()).expect("compressed shell is valid");

        assert_eq!(extracted.tracking_ids(), expected);
    }

    #[test]
    fn tracked_shell_serde_roundtrip_uses_tracking_wrapper() {
        let mut shell = Shell::from([triangle()]);
        let mut session = session();
        shell
            .initialize_tracking(&mut session, feature("serde"))
            .expect("tracking initialization succeeds");
        let expected = shell.tracking_ids();
        let json = serde_json::to_string(&shell).expect("tracked shell serializes");
        let extracted: Shell<(), (), ()> =
            serde_json::from_str(&json).expect("tracked shell deserializes");

        assert!(json.contains("\"tracking\""));
        assert_eq!(extracted.tracking_ids(), expected);
    }
}
