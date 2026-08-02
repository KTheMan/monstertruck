//! Finite, nondegenerate, and consistently oriented triangle validation.

use super::classify::ImportedShell;
use super::errors::ValidationError;
use monstertruck_geometry::prelude::{InnerSpace, Point3, Vector3};
use monstertruck_meshing::prelude::{PolygonMesh, RobustMeshableShape, StandardVertex};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub(super) struct MeshValidationConfig {
    pub(super) tessellation_tolerance: f64,
    pub(super) normalized_double_area_tolerance: f64,
    pub(super) minimum_normal_alignment: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(super) struct MeshEvidence {
    face_count: usize,
    triangle_count: usize,
    triangles_only: bool,
    finite: bool,
    nondegenerate: bool,
    consistently_oriented: bool,
    minimum_normalized_double_area: f64,
    normalized_double_area_tolerance: f64,
    minimum_normal_alignment: f64,
    normal_alignment_tolerance: f64,
}

impl MeshEvidence {
    pub(super) const fn triangle_count(&self) -> usize { self.triangle_count }

    pub(super) const fn minimum_normalized_double_area(&self) -> f64 {
        self.minimum_normalized_double_area
    }

    pub(super) const fn minimum_normal_alignment(&self) -> f64 { self.minimum_normal_alignment }
}

pub(super) fn validate_mesh(
    shell: &ImportedShell,
    config: MeshValidationConfig,
) -> Result<MeshEvidence, ValidationError> {
    let meshed = shell.robust_triangulation(config.tessellation_tolerance);
    let meshes = meshed
        .faces
        .iter()
        .enumerate()
        .map(|(face, face_mesh)| {
            face_mesh
                .surface
                .as_ref()
                .ok_or(ValidationError::MissingFaceMesh { face: face + 1 })
                .map(|mesh| (face, mesh))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bounds = finite_bounds(&meshes)?;
    let scale = Vector3::new(
        bounds[3] - bounds[0],
        bounds[4] - bounds[1],
        bounds[5] - bounds[2],
    )
    .magnitude();
    if !scale.is_finite() || scale <= f64::EPSILON {
        Err(ValidationError::InvalidMeshScale)
    } else {
        meshes
            .iter()
            .try_fold(
                MeshEvidence {
                    face_count: meshes.len(),
                    triangle_count: 0,
                    triangles_only: true,
                    finite: true,
                    nondegenerate: true,
                    consistently_oriented: true,
                    minimum_normalized_double_area: f64::INFINITY,
                    normalized_double_area_tolerance: config.normalized_double_area_tolerance,
                    minimum_normal_alignment: f64::INFINITY,
                    normal_alignment_tolerance: config.minimum_normal_alignment,
                },
                |evidence, (face, mesh)| validate_face(evidence, *face, mesh, scale, config),
            )
            .and_then(|evidence| {
                if evidence.triangle_count == 0 {
                    Err(ValidationError::EmptyMesh)
                } else {
                    Ok(evidence)
                }
            })
    }
}

fn finite_bounds(meshes: &[(usize, &PolygonMesh)]) -> Result<[f64; 6], ValidationError> {
    meshes.iter().try_fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |bounds, (face, mesh)| {
            mesh.positions()
                .iter()
                .enumerate()
                .try_fold(bounds, |mut bounds, (position, point)| {
                    if finite_point(*point) {
                        bounds[0] = bounds[0].min(point.x);
                        bounds[1] = bounds[1].min(point.y);
                        bounds[2] = bounds[2].min(point.z);
                        bounds[3] = bounds[3].max(point.x);
                        bounds[4] = bounds[4].max(point.y);
                        bounds[5] = bounds[5].max(point.z);
                        Ok(bounds)
                    } else {
                        Err(ValidationError::NonFiniteMeshPosition {
                            face: *face + 1,
                            position,
                        })
                    }
                })
        },
    )
}

fn validate_face(
    evidence: MeshEvidence,
    face: usize,
    mesh: &PolygonMesh,
    scale: f64,
    config: MeshValidationConfig,
) -> Result<MeshEvidence, ValidationError> {
    if !mesh.quad_faces().is_empty() || !mesh.other_faces().is_empty() {
        Err(ValidationError::NonTriangularFaceMesh { face: face + 1 })
    } else {
        mesh.tri_faces().iter().enumerate().try_fold(
            evidence,
            |mut evidence, (triangle, vertices)| {
                let positions = triangle_positions(mesh, *vertices, face, triangle)?;
                let geometric_normal =
                    (positions[1] - positions[0]).cross(positions[2] - positions[0]);
                let double_area = geometric_normal.magnitude();
                let normalized_double_area = double_area / scale.powi(2);
                if !normalized_double_area.is_finite()
                    || normalized_double_area <= config.normalized_double_area_tolerance
                {
                    Err(ValidationError::DegenerateTriangle {
                        face: face + 1,
                        triangle: triangle + 1,
                        normalized_double_area,
                        tolerance: config.normalized_double_area_tolerance,
                    })
                } else {
                    let minimum_alignment =
                        vertices.iter().try_fold(f64::INFINITY, |minimum, vertex| {
                            let normal = triangle_normal(mesh, *vertex, face, triangle)?;
                            let magnitude = normal.magnitude();
                            let alignment =
                                geometric_normal.dot(normal) / (double_area * magnitude);
                            if !alignment.is_finite() || alignment < config.minimum_normal_alignment
                            {
                                Err(ValidationError::InconsistentTriangleOrientation {
                                    face: face + 1,
                                    triangle: triangle + 1,
                                    alignment,
                                    tolerance: config.minimum_normal_alignment,
                                })
                            } else {
                                Ok(minimum.min(alignment))
                            }
                        })?;
                    evidence.triangle_count += 1;
                    evidence.minimum_normalized_double_area = evidence
                        .minimum_normalized_double_area
                        .min(normalized_double_area);
                    evidence.minimum_normal_alignment =
                        evidence.minimum_normal_alignment.min(minimum_alignment);
                    Ok(evidence)
                }
            },
        )
    }
}

fn triangle_positions(
    mesh: &PolygonMesh,
    triangle: [StandardVertex; 3],
    face: usize,
    triangle_index: usize,
) -> Result<[Point3; 3], ValidationError> {
    let position = |vertex: StandardVertex| {
        mesh.positions()
            .get(vertex.pos)
            .copied()
            .ok_or(ValidationError::InvalidTriangleVertex {
                face: face + 1,
                triangle: triangle_index + 1,
            })
    };
    Ok([
        position(triangle[0])?,
        position(triangle[1])?,
        position(triangle[2])?,
    ])
}

fn triangle_normal(
    mesh: &PolygonMesh,
    vertex: StandardVertex,
    face: usize,
    triangle: usize,
) -> Result<Vector3, ValidationError> {
    let normal = vertex
        .nor
        .and_then(|index| mesh.normals().get(index))
        .copied()
        .ok_or(ValidationError::MissingTriangleNormal {
            face: face + 1,
            triangle: triangle + 1,
        })?;
    if finite_vector(normal) && normal.magnitude() > f64::EPSILON {
        Ok(normal)
    } else {
        Err(ValidationError::NonFiniteTriangleNormal {
            face: face + 1,
            triangle: triangle + 1,
        })
    }
}

const fn finite_point(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

const fn finite_vector(vector: Vector3) -> bool {
    vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
}
