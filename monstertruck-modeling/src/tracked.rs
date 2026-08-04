//! Contextual modeling operations with session-scoped topology lineage.
//!
//! Raw mapping and sweep traits remain useful for geometry construction. The
//! wrappers in this module add the operation context needed to validate
//! preserved identities or initialize fresh generated identities.
//!
//! The following tracked extrusion preserves the source face identities,
//! assigns fresh identities to generated topology, and records lineage in the
//! [`TrackingSession`].
//!
//! ```
//! use monstertruck_core::{Point3, TrackingSession, TrackingSessionId, Vector3};
//! use monstertruck_modeling::{builder, tracked, Edge, Face, Solid};
//! use monstertruck_topology::{FeatureId, TopologyTracking};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let vertex = builder::vertex(Point3::new(0.0, 0.0, 0.0));
//!     let edge: Edge = builder::extrude(&vertex, Vector3::unit_x());
//!     let mut face: Face = builder::extrude(&edge, Vector3::unit_y());
//!     let mut session = TrackingSession::new(TrackingSessionId::new("modeling-session")?);
//!     face.initialize_tracking(&mut session, FeatureId::new("profile")?)?;
//!
//!     let solid: Solid = tracked::extrude(
//!         &face,
//!         Vector3::unit_z(),
//!         &mut session,
//!         FeatureId::new("pad")?,
//!     )?;
//!
//!     assert!(!solid.tracking_ids().is_empty());
//!     assert_eq!(session.lineage().len(), face.tracking_ids().len());
//!     Ok(())
//! }
//! ```

#[cfg(feature = "solid")]
use std::collections::BTreeSet;

use monstertruck_core::{Matrix4, Point3, Rad, Vector3};
#[cfg(feature = "solid")]
use monstertruck_topology::TrackingReport;
use monstertruck_topology::{
    FeatureId, LineageRelation, OperationKind, TopologyTracking, TrackingError, TrackingId,
    TrackingSession,
};
use thiserror::Error;

#[cfg(feature = "fillet")]
use crate::Edge;
use crate::builder::{self, SweepAngle};
use crate::geom_impls::{ArcConnector, ExtrudeConnector, LineConnector, RevoluteConnector};
use crate::topo_traits::{ClosedSweep, Mapped, Sweep};
#[cfg(feature = "solid")]
use crate::{Curve, Solid, Surface};
use crate::{Shell, Wire};

/// Result type for tracked modeling operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Failure produced while applying a tracked modeling operation.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum Error {
    /// A session-level tracking invariant failed.
    #[error(transparent)]
    Tracking(#[from] TrackingError),
    /// The operation source contains no tracked topology.
    #[error("the modeling operation source contains no tracked topology")]
    UntrackedSource,
    /// A topology-preserving map changed the ordered tracking identity set.
    #[error("the mapped topology did not preserve its ordered tracking identity set")]
    IdentityMismatch,
    /// A Boolean shape operation failed.
    #[cfg(feature = "solid")]
    #[error("Boolean shape operation failed: {0}")]
    ShapeOperation(String),
    /// An edge-fillet operation failed.
    #[cfg(feature = "fillet")]
    #[error("edge-fillet operation failed: {0}")]
    Fillet(String),
    /// Plane-cut section faces could not be matched to the tracked solid.
    #[cfg(feature = "solid")]
    #[error("plane-cut section topology did not match the tracked result solid")]
    SectionTrackingMismatch,
}

fn current_ids<T: TopologyTracking>(
    topology: &T,
    session: &TrackingSession,
) -> Result<Vec<TrackingId>> {
    let ids = topology.tracking_ids();
    if ids.is_empty() {
        Err(Error::UntrackedSource)
    } else {
        ids.iter()
            .try_for_each(|tracking_id| session.validate_current(tracking_id))?;
        Ok(ids)
    }
}

fn record_preserved<T: TopologyTracking>(
    source: &[TrackingId],
    output: &T,
    session: &mut TrackingSession,
    operation: OperationKind,
) -> Result<()> {
    let output = current_ids(output, session)?;
    if source != output {
        Err(Error::IdentityMismatch)
    } else {
        source.iter().try_for_each(|tracking_id| {
            session
                .record_lineage(
                    operation,
                    LineageRelation::Preserved,
                    tracking_id.clone(),
                    [tracking_id.clone()],
                )
                .map(|_| ())
        })?;
        Ok(())
    }
}

fn record_generated(
    source: &[TrackingId],
    output: &[TrackingId],
    session: &mut TrackingSession,
    operation: OperationKind,
) -> Result<()> {
    source.iter().try_for_each(|tracking_id| {
        session
            .record_lineage(
                operation,
                LineageRelation::Generated,
                tracking_id.clone(),
                output.iter().cloned(),
            )
            .map(|_| ())
    })?;
    Ok(())
}

#[cfg(feature = "solid")]
fn record_finalized(
    source: &[TrackingId],
    report: &TrackingReport,
    session: &mut TrackingSession,
    operation: OperationKind,
) -> Result<()> {
    source.iter().try_for_each(|tracking_id| {
        let preserved = report
            .all_ids()
            .contains(tracking_id)
            .then(|| tracking_id.clone());
        let replacements = report
            .replacements()
            .iter()
            .filter(|replacement| replacement.original() == tracking_id)
            .map(|replacement| replacement.replacement().clone());
        let children: Vec<_> = preserved.into_iter().chain(replacements).collect();
        let relation = match children.len() {
            0 => LineageRelation::Deleted,
            1 => LineageRelation::Preserved,
            _ => LineageRelation::Split,
        };
        session
            .record_lineage(operation, relation, tracking_id.clone(), children)
            .map(|_| ())
    })?;
    Ok(())
}

#[cfg(feature = "solid")]
fn finalize_output<T: TopologyTracking>(
    output: &mut T,
    source: &[TrackingId],
    session: &mut TrackingSession,
    feature: FeatureId,
    operation: OperationKind,
) -> Result<()> {
    let report = output.initialize_tracking(session, feature)?;
    record_finalized(source, &report, session, operation)
}

fn commit_session<T>(
    session: &mut TrackingSession,
    operation: impl FnOnce(&mut TrackingSession) -> Result<T>,
) -> Result<T> {
    let mut staged_session = session.clone();
    let output = operation(&mut staged_session)?;
    *session = staged_session;
    Ok(output)
}

/// Transforms topology while preserving its current tracking identities.
///
/// The wrapper rejects untracked sources and mapped results whose ordered
/// identity set differs from the source.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not current,
/// the mapping changes topology identity, or lineage recording fails.
pub fn transformed<T>(topology: &T, matrix: Matrix4, session: &mut TrackingSession) -> Result<T>
where T: Mapped<Matrix4> + TopologyTracking {
    let source = current_ids(topology, session)?;
    let output = builder::transformed(topology, matrix);
    commit_session(session, |session| {
        record_preserved(&source, &output, session, OperationKind::Map)
    })?;
    Ok(output)
}

/// Clones topology while preserving its current tracking identities.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not
/// current, cloning changes topology identity, or lineage recording fails.
pub fn cloned<T>(topology: &T, session: &mut TrackingSession) -> Result<T>
where T: Mapped<()> + TopologyTracking {
    let source = current_ids(topology, session)?;
    let output = builder::clone(topology);
    commit_session(session, |session| {
        record_preserved(&source, &output, session, OperationKind::Clone)
    })?;
    Ok(output)
}

/// Translates topology while preserving its current tracking identities.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not
/// current, translation changes topology identity, or lineage recording fails.
pub fn translated<T>(topology: &T, vector: Vector3, session: &mut TrackingSession) -> Result<T>
where T: Mapped<Matrix4> + TopologyTracking {
    let source = current_ids(topology, session)?;
    let output = builder::translated(topology, vector);
    commit_session(session, |session| {
        record_preserved(&source, &output, session, OperationKind::Translate)
    })?;
    Ok(output)
}

/// Rotates topology while preserving its current tracking identities.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not current,
/// rotation changes topology identity, or lineage recording fails.
pub fn rotated<T>(
    topology: &T,
    origin: Point3,
    axis: Vector3,
    angle: Rad<f64>,
    session: &mut TrackingSession,
) -> Result<T>
where
    T: Mapped<Matrix4> + TopologyTracking,
{
    let source = current_ids(topology, session)?;
    let output = builder::rotated(topology, origin, axis, angle);
    commit_session(session, |session| {
        record_preserved(&source, &output, session, OperationKind::Rotate)
    })?;
    Ok(output)
}

/// Scales topology while preserving its current tracking identities.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not
/// current, scaling changes topology identity, or lineage recording fails.
pub fn scaled<T>(
    topology: &T,
    origin: Point3,
    scalars: Vector3,
    session: &mut TrackingSession,
) -> Result<T>
where
    T: Mapped<Matrix4> + TopologyTracking,
{
    let source = current_ids(topology, session)?;
    let output = builder::scaled(topology, origin, scalars);
    commit_session(session, |session| {
        record_preserved(&source, &output, session, OperationKind::Scale)
    })?;
    Ok(output)
}

/// Sweeps topology and initializes fresh tracking identities on the result.
///
/// Every tracked source element records generated lineage to the complete,
/// deterministic set of fresh identities in the sweep result.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not current,
/// result initialization fails, or lineage recording fails.
pub fn sweep<T, Mapping, PointConnector, CurveConnector, Swept>(
    topology: &T,
    mapping: Mapping,
    point_connector: PointConnector,
    curve_connector: CurveConnector,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Swept>
where
    T: Sweep<Mapping, PointConnector, CurveConnector, Swept> + TopologyTracking,
    Swept: TopologyTracking,
{
    let source = current_ids(topology, session)?;
    let mut output = topology.sweep(mapping, point_connector, curve_connector);
    commit_session(session, |session| {
        let report = output.initialize_tracking(session, feature)?;
        let generated: Vec<_> = report.generated_ids().cloned().collect();
        record_generated(&source, &generated, session, OperationKind::Sweep)
    })?;
    Ok(output)
}

/// Extrudes topology and initializes fresh tracking identities on the result.
///
/// Every tracked source element records generated lineage to the complete,
/// deterministic set of fresh identities in the extrusion result.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not current,
/// result initialization fails, or lineage recording fails.
pub fn extrude<T, Swept>(
    topology: &T,
    vector: Vector3,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Swept>
where
    T: Sweep<Matrix4, LineConnector, ExtrudeConnector, Swept> + TopologyTracking,
    Swept: TopologyTracking,
{
    let source = current_ids(topology, session)?;
    let mut output = builder::extrude(topology, vector);
    commit_session(session, |session| {
        let report = output.initialize_tracking(session, feature)?;
        let generated: Vec<_> = report.generated_ids().cloned().collect();
        record_generated(&source, &generated, session, OperationKind::Extrude)
    })?;
    Ok(output)
}

/// Revolves topology and initializes fresh tracking identities on the result.
///
/// Every tracked source element records generated lineage to the complete,
/// deterministic set of fresh identities in the revolved result.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not current,
/// result initialization fails, or lineage recording fails.
pub fn revolve<T, Swept>(
    topology: &T,
    origin: Point3,
    axis: Vector3,
    sweep: SweepAngle,
    division: usize,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Swept>
where
    T: ClosedSweep<Matrix4, ArcConnector, RevoluteConnector, Swept> + TopologyTracking,
    Swept: TopologyTracking,
{
    let source = current_ids(topology, session)?;
    let mut output = builder::revolve(topology, origin, axis, sweep, division);
    commit_session(session, |session| {
        let report = output.initialize_tracking(session, feature)?;
        let generated: Vec<_> = report.generated_ids().cloned().collect();
        record_generated(&source, &generated, session, OperationKind::Revolve)
    })?;
    Ok(output)
}

/// Revolves a wire while collapsing degenerate on-axis edges and tracking all
/// generated topology.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not
/// current, result initialization fails, or lineage recording fails.
pub fn revolve_wire(
    wire: &Wire,
    origin: Point3,
    axis: Vector3,
    sweep: SweepAngle,
    division: usize,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Shell> {
    let source = current_ids(wire, session)?;
    let mut output = builder::revolve_wire(wire, origin, axis, sweep, division);
    commit_session(session, |session| {
        let report = output.initialize_tracking(session, feature)?;
        let generated: Vec<_> = report.generated_ids().cloned().collect();
        record_generated(&source, &generated, session, OperationKind::Revolve)
    })?;
    Ok(output)
}

/// Revolves a wire around an axis through its first vertex and tracks all
/// generated topology.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, an identity is not
/// current, result initialization fails, or lineage recording fails.
#[deprecated(note = "Use `revolve_wire` instead, which takes an explicit origin parameter.")]
pub fn cone(
    wire: &Wire,
    axis: Vector3,
    sweep: SweepAngle,
    division: usize,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Shell> {
    let origin = wire
        .front_vertex()
        .map_or(Point3::new(0.0, 0.0, 0.0), |vertex| vertex.point());
    revolve_wire(wire, origin, axis, sweep, division, session, feature)
}

#[cfg(feature = "solid")]
fn boolean_operation(
    first: &Solid,
    second: &Solid,
    session: &mut TrackingSession,
    feature: FeatureId,
    operation: OperationKind,
    boolean: impl FnOnce(
        &Solid,
        &Solid,
    ) -> std::result::Result<Solid, monstertruck_solid::ShapeOpsError>,
) -> Result<Solid> {
    let source: Vec<_> = current_ids(first, session)?
        .into_iter()
        .chain(current_ids(second, session)?)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut output =
        boolean(first, second).map_err(|error| Error::ShapeOperation(error.to_string()))?;
    commit_session(session, |session| {
        finalize_output(&mut output, &source, session, feature, operation)
    })?;
    Ok(output)
}

/// Intersects two solids and records preserved, split, and deleted topology.
///
/// # Errors
///
/// Returns [`enum@Error`] when either source is untracked, the Boolean
/// operation fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn and(
    first: &Solid,
    second: &Solid,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    boolean_operation(
        first,
        second,
        session,
        feature,
        OperationKind::BooleanIntersection,
        |first, second| monstertruck_solid::and(first, second, tolerance),
    )
}

/// Intersects two solids using known shell-orientation hints.
///
/// # Errors
///
/// Returns [`enum@Error`] when either source is untracked, the Boolean
/// operation fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn and_with_orientation_hints(
    first: &Solid,
    second: &Solid,
    orientation_hints: monstertruck_solid::ShellOrientationHints,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    boolean_operation(
        first,
        second,
        session,
        feature,
        OperationKind::BooleanIntersection,
        |first, second| {
            monstertruck_solid::and_with_orientation_hints(
                first,
                second,
                orientation_hints,
                tolerance,
            )
        },
    )
}

/// Unites two solids and records preserved, split, and deleted topology.
///
/// # Errors
///
/// Returns [`enum@Error`] when either source is untracked, the Boolean
/// operation fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn or(
    first: &Solid,
    second: &Solid,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    boolean_operation(
        first,
        second,
        session,
        feature,
        OperationKind::BooleanUnion,
        |first, second| monstertruck_solid::or(first, second, tolerance),
    )
}

/// Subtracts `second` from `first` and records topology lineage.
///
/// # Errors
///
/// Returns [`enum@Error`] when either source is untracked, the Boolean
/// operation fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn difference(
    first: &Solid,
    second: &Solid,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    boolean_operation(
        first,
        second,
        session,
        feature,
        OperationKind::BooleanDifference,
        |first, second| monstertruck_solid::difference(first, second, tolerance),
    )
}

/// Computes the symmetric difference and records topology lineage.
///
/// # Errors
///
/// Returns [`enum@Error`] when either source is untracked, the Boolean
/// operation fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn symmetric_difference(
    first: &Solid,
    second: &Solid,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    boolean_operation(
        first,
        second,
        session,
        feature,
        OperationKind::BooleanSymmetricDifference,
        |first, second| monstertruck_solid::symmetric_difference(first, second, tolerance),
    )
}

/// Clips a solid against a canonical clip-space half-space.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, clipping fails,
/// result tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn clip_half_space_z(
    solid: &Solid,
    world_to_clip: Matrix4,
    keep_positive_z: bool,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<Solid> {
    let source = current_ids(solid, session)?;
    let mut output =
        monstertruck_solid::clip_half_space_z(solid, world_to_clip, keep_positive_z, tolerance)
            .map_err(|error| Error::ShapeOperation(error.to_string()))?;
    commit_session(session, |session| {
        finalize_output(&mut output, &source, session, feature, OperationKind::Cut)
    })?;
    Ok(output)
}

/// Cuts a solid by a plane and tracks both the solid and section faces.
///
/// # Errors
///
/// Returns [`enum@Error`] when the source is untracked, cutting fails, result
/// tracking fails, or lineage recording fails.
#[cfg(feature = "solid")]
pub fn plane_cut(
    solid: &Solid,
    world_to_clip: Matrix4,
    keep_positive_z: bool,
    tolerance: f64,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<monstertruck_solid::PlaneCut<Curve, Surface>> {
    let source = current_ids(solid, session)?;
    let mut output =
        monstertruck_solid::plane_cut(solid, world_to_clip, keep_positive_z, tolerance)
            .map_err(|error| Error::ShapeOperation(error.to_string()))?;
    let section = commit_session(session, |session| {
        finalize_output(
            &mut output.solid,
            &source,
            session,
            feature,
            OperationKind::Cut,
        )?;
        output
            .section
            .iter()
            .map(|section_face| {
                output
                    .solid
                    .face_iter()
                    .find(|solid_face| solid_face.is_same(section_face))
                    .cloned()
                    .ok_or(Error::SectionTrackingMismatch)
            })
            .collect::<Result<Vec<_>>>()
    })?;
    output.section = section;
    Ok(output)
}

/// Fillets selected shell edges and records replacement topology.
///
/// The mutation is transactional: both the shell and tracking session remain
/// unchanged when filleting or tracking fails.
///
/// # Errors
///
/// Returns [`enum@Error`] when the shell or selected edges are untracked,
/// filleting fails, result tracking fails, or lineage recording fails.
#[cfg(feature = "fillet")]
pub fn fillet_edges(
    shell: &mut Shell,
    edges: &[Edge],
    options: Option<&monstertruck_fillet::FilletOptions>,
    session: &mut TrackingSession,
    feature: FeatureId,
) -> Result<()> {
    let source = current_ids(shell, session)?;
    edges
        .iter()
        .try_for_each(|edge| current_ids(edge, session).map(|_| ()))?;
    let mut output = shell.clone();
    monstertruck_fillet::fillet_edges_generic(&mut output, edges, options)
        .map_err(|error| Error::Fillet(error.to_string()))?;
    commit_session(session, |session| {
        finalize_output(
            &mut output,
            &source,
            session,
            feature,
            OperationKind::Fillet,
        )
    })?;
    *shell = output;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Edge, Face, Solid, Vertex, Wire};
    use monstertruck_core::{Deg, TrackingSessionId};
    use monstertruck_topology::{Edge as RawEdge, Vertex as RawVertex};
    use std::collections::BTreeSet;
    use std::fmt::Debug;

    fn feature(value: &str) -> FeatureId {
        FeatureId::new(value).expect("the test feature identifier is valid")
    }

    fn session(value: &str) -> TrackingSession {
        TrackingSession::new(
            TrackingSessionId::new(value).expect("the test session identifier is valid"),
        )
    }

    fn tracked_square(session: &mut TrackingSession) -> Face {
        let vertex = builder::vertex(Point3::new(0.0, 0.0, 0.0));
        let edge: Edge = builder::extrude(&vertex, Vector3::unit_x());
        let mut face: Face = builder::extrude(&edge, Vector3::unit_y());
        face.initialize_tracking(session, feature("square"))
            .expect("tracking initialization succeeds");
        face
    }

    fn ids<T: TopologyTracking>(topology: &T) -> Vec<TrackingId> { topology.tracking_ids() }

    fn entry(
        kind: &str,
        identity: impl Debug,
        tracking_id: Option<&TrackingId>,
    ) -> (String, TrackingId) {
        (
            format!("{kind}:{identity:?}"),
            tracking_id
                .expect("tracking initialization assigns every topology element")
                .clone(),
        )
    }

    fn assert_injective(entries: &[(String, TrackingId)]) {
        (0..entries.len())
            .flat_map(|first| ((first + 1)..entries.len()).map(move |second| (first, second)))
            .for_each(|(first, second)| {
                let (first_identity, first_tracking) = &entries[first];
                let (second_identity, second_tracking) = &entries[second];
                if first_identity == second_identity {
                    assert_eq!(first_tracking, second_tracking);
                } else {
                    assert_ne!(first_tracking, second_tracking);
                }
            });
    }

    fn assert_solid_tracking_is_injective(solid: &Solid) {
        let entries: Vec<_> = solid
            .face_iter()
            .map(|face| entry("face", face.id(), face.tracking_id()))
            .chain(solid.edge_iter().flat_map(|edge| {
                let (front, back) = edge.absolute_ends();
                [
                    entry("edge", edge.id(), edge.tracking_id()),
                    entry("vertex", front.id(), front.tracking_id()),
                    entry("vertex", back.id(), back.tracking_id()),
                ]
            }))
            .collect();
        assert_injective(&entries);
    }

    fn assert_wire_tracking_is_injective(wire: &Wire) {
        let entries: Vec<_> = wire
            .edge_iter()
            .flat_map(|edge| {
                let (front, back) = edge.absolute_ends();
                [
                    entry("edge", edge.id(), edge.tracking_id()),
                    entry("vertex", front.id(), front.tracking_id()),
                    entry("vertex", back.id(), back.tracking_id()),
                ]
            })
            .collect();
        assert_injective(&entries);
    }

    #[test]
    fn rotation_preserves_tracking_identity_and_records_lineage() {
        let mut session = session("rotation");
        let square = tracked_square(&mut session);
        let before = ids(&square);

        let rotated = rotated(
            &square,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            Deg(90.0).into(),
            &mut session,
        )
        .expect("tracked rotation succeeds");

        assert_eq!(ids(&rotated), before);
        assert_eq!(session.lineage().len(), before.len());
        assert!(session.lineage().iter().all(|event| {
            event.operation() == OperationKind::Rotate
                && event.relation() == LineageRelation::Preserved
                && event.children() == [event.parent().clone()]
        }));
    }

    #[test]
    fn transformation_preserves_tracking_identity_and_records_lineage() {
        let mut session = session("transformation");
        let square = tracked_square(&mut session);
        let before = ids(&square);

        let transformed = transformed(
            &square,
            Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0)),
            &mut session,
        )
        .expect("tracked transformation succeeds");

        assert_eq!(ids(&transformed), before);
        assert_eq!(session.lineage().len(), before.len());
        assert!(session.lineage().iter().all(|event| {
            event.operation() == OperationKind::Map
                && event.relation() == LineageRelation::Preserved
                && event.children() == [event.parent().clone()]
        }));
    }

    #[test]
    fn generic_sweep_assigns_fresh_ids_and_generated_lineage() {
        let mut session = session("generic-sweep");
        let mut vertex = RawVertex::new(1_usize);
        vertex
            .initialize_tracking(&mut session, feature("source"))
            .expect("tracking initialization succeeds");
        let source = ids(&vertex);
        let mapping: fn(&usize) -> usize = |value| *value + 1;
        let point_connector: fn(&usize, &usize) -> isize =
            |first, second| (first * 10 + second) as isize;
        let curve_connector: fn(&isize, &isize) = |_, _| ();

        let edge: RawEdge<usize, isize> = sweep(
            &vertex,
            mapping,
            point_connector,
            curve_connector,
            &mut session,
            feature("sweep"),
        )
        .expect("tracked generic sweep succeeds");
        let output = ids(&edge);
        let output_set: BTreeSet<_> = output.iter().cloned().collect();
        let source_set: BTreeSet<_> = source.iter().cloned().collect();
        let generated = session.lineage()[0].children();
        let generated_set: BTreeSet<_> = generated.iter().cloned().collect();
        let expected: BTreeSet<_> = output_set.difference(&source_set).cloned().collect();
        let (front, back) = edge.absolute_ends();

        assert_eq!(generated_set, expected);
        assert_eq!(session.lineage().len(), 1);
        assert_eq!(session.lineage()[0].operation(), OperationKind::Sweep);
        assert_eq!(session.lineage()[0].relation(), LineageRelation::Generated);
        assert_injective(&[
            entry("edge", edge.id(), edge.tracking_id()),
            entry("vertex", front.id(), front.tracking_id()),
            entry("vertex", back.id(), back.tracking_id()),
        ]);
    }

    #[test]
    fn extrusion_assigns_fresh_unique_ids_and_generated_lineage() {
        let mut session = session("extrusion");
        let square = tracked_square(&mut session);
        let source = ids(&square);

        let solid: Solid = extrude(&square, Vector3::unit_z(), &mut session, feature("cube"))
            .expect("tracked extrusion succeeds");
        let output = ids(&solid);
        let output_set: BTreeSet<_> = output.iter().cloned().collect();
        let source_set: BTreeSet<_> = source.iter().cloned().collect();
        let generated = session.lineage()[0].children();
        let unique: BTreeSet<_> = generated.iter().cloned().collect();
        let expected: BTreeSet<_> = output_set.difference(&source_set).cloned().collect();

        assert_eq!(unique.len(), generated.len());
        assert!(source.iter().all(|id| output_set.contains(id)));
        assert!(source.iter().all(|id| !unique.contains(id)));
        assert!(generated.iter().all(|id| output_set.contains(id)));
        assert_eq!(unique, expected);
        assert_eq!(session.lineage().len(), source.len());
        assert!(session.lineage().iter().all(|event| {
            event.operation() == OperationKind::Extrude
                && event.relation() == LineageRelation::Generated
                && event.children() == generated
        }));
        assert_solid_tracking_is_injective(&solid);
    }

    #[test]
    fn revolve_replaces_propagated_layer_ids_with_fresh_unique_ids() {
        let mut session = session("revolve");
        let mut vertex: Vertex = builder::vertex(Point3::new(2.0, 0.0, 0.0));
        vertex
            .initialize_tracking(&mut session, feature("profile"))
            .expect("tracking initialization succeeds");
        let source = ids(&vertex);

        let wire: Wire = revolve(
            &vertex,
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
            SweepAngle::Closed,
            4,
            &mut session,
            feature("circle"),
        )
        .expect("tracked revolve succeeds");
        let output = ids(&wire);
        let output_set: BTreeSet<_> = output.iter().cloned().collect();
        let source_set: BTreeSet<_> = source.iter().cloned().collect();
        let generated = session.lineage()[0].children();
        let unique: BTreeSet<_> = generated.iter().cloned().collect();
        let expected: BTreeSet<_> = output_set.difference(&source_set).cloned().collect();

        assert_eq!(unique.len(), generated.len());
        assert!(source.iter().all(|id| output_set.contains(id)));
        assert!(source.iter().all(|id| !unique.contains(id)));
        assert!(generated.iter().all(|id| output_set.contains(id)));
        assert_eq!(unique, expected);
        assert_eq!(session.lineage().len(), source.len());
        assert!(session.lineage().iter().all(|event| {
            event.operation() == OperationKind::Revolve
                && event.relation() == LineageRelation::Generated
                && event.children() == generated
        }));
        assert_wire_tracking_is_injective(&wire);
    }
}
