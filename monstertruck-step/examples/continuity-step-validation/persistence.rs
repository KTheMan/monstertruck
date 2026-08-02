//! Topology and geometric-bound persistence signatures.

use super::classify::{ImportedShell, to_nurbs};
use super::errors::ValidationError;
use monstertruck_geometry::prelude::{
    BoundedSurface, InnerSpace, MetricSpace, ParametricSurface, Vector3,
};
use monstertruck_step::load::step_geometry::StepParameterCurve;
use monstertruck_topology::compress::CompressedEdgeUse;
use serde::Serialize;

const BOUNDING_BOX_INTERVALS: usize = 32;
const TOPOLOGY_VERTEX_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct TopologySignature {
    vertex_count: usize,
    edge_count: usize,
    face_count: usize,
    euler_characteristic: i64,
    vertex_valences: Vec<usize>,
    edge_face_use_counts: Vec<usize>,
    faces: Vec<FaceSignature>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct FaceSignature {
    orientation: bool,
    boundary_edge_counts: Vec<usize>,
}

impl TopologySignature {
    pub(super) fn from_shell(shell: &ImportedShell) -> Self {
        let mut vertex_valences = (0..shell.vertices.len())
            .map(|vertex| {
                shell
                    .edges
                    .iter()
                    .filter(|edge| edge.vertices.0 == vertex || edge.vertices.1 == vertex)
                    .count()
            })
            .collect::<Vec<_>>();
        vertex_valences.sort_unstable();
        let mut edge_face_use_counts = (0..shell.edges.len())
            .map(|edge| {
                shell
                    .faces
                    .iter()
                    .flat_map(|face| face.boundaries.iter().flatten())
                    .filter(|edge_use| edge_use.index == edge)
                    .count()
            })
            .collect::<Vec<_>>();
        edge_face_use_counts.sort_unstable();
        let mut faces = shell
            .faces
            .iter()
            .map(|face| {
                let mut boundary_edge_counts =
                    face.boundaries.iter().map(Vec::len).collect::<Vec<_>>();
                boundary_edge_counts.sort_unstable();
                FaceSignature {
                    orientation: face.orientation,
                    boundary_edge_counts,
                }
            })
            .collect::<Vec<_>>();
        faces.sort();
        Self {
            vertex_count: shell.vertices.len(),
            edge_count: shell.edges.len(),
            face_count: shell.faces.len(),
            euler_characteristic: shell.vertices.len() as i64 - shell.edges.len() as i64
                + shell.faces.len() as i64,
            vertex_valences,
            edge_face_use_counts,
            faces,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(super) struct BoundingBoxSignature {
    minimum: [f64; 3],
    maximum: [f64; 3],
    diagonal: f64,
    samples: usize,
}

impl BoundingBoxSignature {
    pub(super) fn from_shell(shell: &ImportedShell) -> Result<Self, ValidationError> {
        let surfaces = shell
            .faces
            .iter()
            .filter_map(|face| to_nurbs(&face.surface))
            .collect::<Vec<_>>();
        let evaluated = surfaces.iter().flat_map(|surface| {
            let ((u_start, u_end), (v_start, v_end)) = surface.range_tuple();
            (0..=BOUNDING_BOX_INTERVALS).flat_map(move |u_sample| {
                (0..=BOUNDING_BOX_INTERVALS).map(move |v_sample| {
                    let u = normalized_parameter(u_start, u_end, u_sample);
                    let v = normalized_parameter(v_start, v_end, v_sample);
                    surface.evaluate(u, v)
                })
            })
        });
        let points = shell.vertices.iter().copied().chain(evaluated);
        let (bounds, samples) = points.fold(
            (
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                0_usize,
            ),
            |(mut bounds, samples), point| {
                bounds[0] = bounds[0].min(point.x);
                bounds[1] = bounds[1].min(point.y);
                bounds[2] = bounds[2].min(point.z);
                bounds[3] = bounds[3].max(point.x);
                bounds[4] = bounds[4].max(point.y);
                bounds[5] = bounds[5].max(point.z);
                (bounds, samples + 1)
            },
        );
        let minimum = [bounds[0], bounds[1], bounds[2]];
        let maximum = [bounds[3], bounds[4], bounds[5]];
        let diagonal = Vector3::new(
            maximum[0] - minimum[0],
            maximum[1] - minimum[1],
            maximum[2] - minimum[2],
        )
        .magnitude();
        if samples == 0
            || minimum
                .into_iter()
                .chain(maximum)
                .any(|value| !value.is_finite())
            || !diagonal.is_finite()
            || diagonal <= f64::EPSILON
        {
            Err(ValidationError::InvalidBoundingBox)
        } else {
            Ok(Self {
                minimum,
                maximum,
                diagonal,
                samples,
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(super) struct PersistenceEvidence {
    topology_before_export: TopologySignature,
    topology_after_reimport: TopologySignature,
    topology_equal: bool,
    bounding_box_before_export: BoundingBoxSignature,
    bounding_box_after_reimport: BoundingBoxSignature,
    bounding_box_normalized_maximum_drift: f64,
    bounding_box_normalized_tolerance: f64,
}

impl PersistenceEvidence {
    pub(super) const fn bounding_box_normalized_maximum_drift(&self) -> f64 {
        self.bounding_box_normalized_maximum_drift
    }

    pub(super) fn compare(
        before: &ImportedShell,
        after: &ImportedShell,
        bounding_box_tolerance: f64,
    ) -> Result<Self, ValidationError> {
        let topology_before_export = TopologySignature::from_shell(before);
        let topology_after_reimport = TopologySignature::from_shell(after);
        let bounding_box_before_export = BoundingBoxSignature::from_shell(before)?;
        let bounding_box_after_reimport = BoundingBoxSignature::from_shell(after)?;
        let scale = bounding_box_before_export
            .diagonal
            .max(bounding_box_after_reimport.diagonal);
        let topology_equal = topology_before_export == topology_after_reimport
            && topology_corresponds(before, after, TOPOLOGY_VERTEX_TOLERANCE * scale);
        if !topology_equal {
            Err(ValidationError::TopologyPersistenceMismatch {
                before: format!("{topology_before_export:?}"),
                after: format!("{topology_after_reimport:?}"),
            })
        } else {
            let maximum_drift = bounding_box_before_export
                .minimum
                .into_iter()
                .chain(bounding_box_before_export.maximum)
                .zip(
                    bounding_box_after_reimport
                        .minimum
                        .into_iter()
                        .chain(bounding_box_after_reimport.maximum),
                )
                .map(|(before, after)| (before - after).abs())
                .fold(0.0, f64::max);
            let normalized_drift = maximum_drift / scale;
            if !normalized_drift.is_finite() || normalized_drift > bounding_box_tolerance {
                Err(ValidationError::BoundingBoxPersistenceFailed {
                    maximum: normalized_drift,
                    tolerance: bounding_box_tolerance,
                })
            } else {
                Ok(Self {
                    topology_before_export,
                    topology_after_reimport,
                    topology_equal,
                    bounding_box_before_export,
                    bounding_box_after_reimport,
                    bounding_box_normalized_maximum_drift: normalized_drift,
                    bounding_box_normalized_tolerance: bounding_box_tolerance,
                })
            }
        }
    }
}

fn topology_corresponds(before: &ImportedShell, after: &ImportedShell, tolerance: f64) -> bool {
    vertex_correspondence(before, after, tolerance).is_some_and(|correspondence| {
        let identity = identity(after.vertices.len());
        canonical_edges(before, &correspondence) == canonical_edges(after, &identity)
            && match (
                canonical_faces(before, &correspondence),
                canonical_faces(after, &identity),
            ) {
                (Some(before), Some(after)) => before == after,
                _ => false,
            }
    })
}

fn vertex_correspondence(
    before: &ImportedShell,
    after: &ImportedShell,
    tolerance: f64,
) -> Option<Vec<usize>> {
    (before.vertices.len() == after.vertices.len()).then_some(())?;
    before
        .vertices
        .iter()
        .try_fold(
            (
                Vec::with_capacity(before.vertices.len()),
                vec![false; after.vertices.len()],
            ),
            |(mut correspondence, mut used), point| {
                let candidate = after
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !used[*index])
                    .map(|(index, candidate)| (index, point.distance2(*candidate)))
                    .filter(|(_, distance)| {
                        distance.is_finite() && *distance <= tolerance * tolerance
                    })
                    .min_by(|left, right| left.1.total_cmp(&right.1))?;
                used[candidate.0] = true;
                correspondence.push(candidate.0);
                Some((correspondence, used))
            },
        )
        .map(|(correspondence, _)| correspondence)
}

fn canonical_edges(shell: &ImportedShell, vertices: &[usize]) -> Vec<[usize; 2]> {
    let mut edges = shell
        .edges
        .iter()
        .map(|edge| {
            let mut endpoints = [vertices[edge.vertices.0], vertices[edge.vertices.1]];
            endpoints.sort_unstable();
            endpoints
        })
        .collect::<Vec<_>>();
    edges.sort_unstable();
    edges
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CanonicalFace {
    orientation: bool,
    boundaries: Vec<Vec<usize>>,
}

fn canonical_faces(shell: &ImportedShell, vertices: &[usize]) -> Option<Vec<CanonicalFace>> {
    let mut faces = shell
        .faces
        .iter()
        .map(|face| {
            let mut boundaries = face
                .boundaries
                .iter()
                .map(|boundary| canonical_boundary(shell, boundary, vertices))
                .collect::<Option<Vec<_>>>()?;
            boundaries.sort();
            Some(CanonicalFace {
                orientation: face.orientation,
                boundaries,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    faces.sort();
    Some(faces)
}

fn canonical_boundary(
    shell: &ImportedShell,
    boundary: &[CompressedEdgeUse<StepParameterCurve>],
    vertices: &[usize],
) -> Option<Vec<usize>> {
    let directed = boundary
        .iter()
        .map(|edge_use| {
            let edge = shell.edges.get(edge_use.index)?;
            let endpoints = if edge_use.orientation {
                edge.vertices
            } else {
                (edge.vertices.1, edge.vertices.0)
            };
            Some((vertices[endpoints.0], vertices[endpoints.1]))
        })
        .collect::<Option<Vec<_>>>()?;
    let connected = directed
        .iter()
        .zip(directed.iter().cycle().skip(1))
        .take(directed.len())
        .all(|(current, next)| current.1 == next.0);
    connected.then(|| canonical_rotation(&directed.iter().map(|edge| edge.0).collect::<Vec<_>>()))
}

fn canonical_rotation(cycle: &[usize]) -> Vec<usize> {
    (0..cycle.len())
        .map(|offset| {
            cycle[offset..]
                .iter()
                .chain(&cycle[..offset])
                .copied()
                .collect::<Vec<_>>()
        })
        .min()
        .unwrap_or_default()
}

fn identity(len: usize) -> Vec<usize> { (0..len).collect() }

fn normalized_parameter(start: f64, end: f64, sample: usize) -> f64 {
    start + (end - start) * sample as f64 / BOUNDING_BOX_INTERVALS as f64
}
