#![allow(missing_docs, unused_qualifications)]

/// re-export [`ruststep`](https://docs.rs/ruststep/latest/ruststep/)
pub use ruststep;

use monstertruck_assembly::assy::*;
use ruststep::{
    ast::{DataSection, EntityInstance, Name, Parameter, SubSuperRecord},
    tables::{EntityTable, IntoOwned, PlaceHolder},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::result::Result;

pub mod convert;
/// Geometry parsed from STEP that can be handled by monstertruck.
pub mod step_geometry;
/// STEP type definitions: structs, enums, and their From/TryFrom impls.
mod step_types;

use step_geometry::*;
pub use step_types::*;

/// the exchange structure corresponds to a graph in STEP file
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    // representation
    pub representation: HashMap<u64, RepresentationHolder>,
    pub representation_item: HashMap<u64, RepresentationItemHolder>,
    pub representation_context: HashMap<u64, RepresentationContextHolder>,

    // primitives
    pub cartesian_point: HashMap<u64, CartesianPointHolder>,
    pub direction: HashMap<u64, DirectionHolder>,
    pub vector: HashMap<u64, VectorHolder>,
    pub placement: HashMap<u64, PlacementHolder>,
    pub axis1_placement: HashMap<u64, Axis1PlacementHolder>,
    pub axis2_placement_2d: HashMap<u64, Axis2Placement2dHolder>,
    pub axis2_placement_3d: HashMap<u64, Axis2Placement3dHolder>,

    // curve
    pub line: HashMap<u64, LineHolder>,
    pub polyline: HashMap<u64, PolylineHolder>,
    pub b_spline_curve_with_knots: HashMap<u64, BsplineCurveWithKnotsHolder>,
    pub bezier_curve: HashMap<u64, BezierCurveHolder>,
    pub quasi_uniform_curve: HashMap<u64, QuasiUniformCurveHolder>,
    pub uniform_curve: HashMap<u64, UniformCurveHolder>,
    pub rational_b_spline_curve: HashMap<u64, RationalBsplineCurveHolder>,
    pub circle: HashMap<u64, CircleHolder>,
    pub ellipse: HashMap<u64, EllipseHolder>,
    pub hyperbola: HashMap<u64, HyperbolaHolder>,
    pub parabola: HashMap<u64, ParabolaHolder>,
    pub pcurve: HashMap<u64, PcurveHolder>,
    pub surface_curve: HashMap<u64, SurfaceCurveHolder>,

    // surface
    pub plane: HashMap<u64, PlaneHolder>,
    pub spherical_surface: HashMap<u64, SphericalSurfaceHolder>,
    pub cylindrical_surface: HashMap<u64, CylindricalSurfaceHolder>,
    pub toroidal_surface: HashMap<u64, ToroidalSurfaceHolder>,
    pub conical_surface: HashMap<u64, ConicalSurfaceHolder>,
    pub b_spline_surface_with_knots: HashMap<u64, BsplineSurfaceWithKnotsHolder>,
    pub uniform_surface: HashMap<u64, UniformSurfaceHolder>,
    pub quasi_uniform_surface: HashMap<u64, QuasiUniformSurfaceHolder>,
    pub bezier_surface: HashMap<u64, BezierSurfaceHolder>,
    pub rational_b_spline_surface: HashMap<u64, RationalBsplineSurfaceHolder>,
    pub surface_of_linear_extrusion: HashMap<u64, SurfaceOfLinearExtrusionHolder>,
    pub surface_of_revolution: HashMap<u64, SurfaceOfRevolutionHolder>,

    // topology
    pub vertex_point: HashMap<u64, VertexPointHolder>,
    pub edge_curve: HashMap<u64, EdgeCurveHolder>,
    pub oriented_edge: HashMap<u64, OrientedEdgeHolder>,
    pub edge_loop: HashMap<u64, EdgeLoopHolder>,
    pub face_bound: HashMap<u64, FaceBoundHolder>,
    pub face_surface: HashMap<u64, FaceSurfaceHolder>,
    pub oriented_face: HashMap<u64, OrientedFaceHolder>,
    pub shell: HashMap<u64, ShellHolder>,
    pub oriented_shell: HashMap<u64, OrientedShellHolder>,
    pub shell_based_surface_model: HashMap<u64, ShellBasedSurfaceModelHolder>,
    pub manifold_solid_brep: HashMap<u64, ManifoldSolidBrepHolder>,

    // assembly
    pub application_context: HashMap<u64, ApplicationContextHolder>,
    pub product_context: HashMap<u64, ProductContextHolder>,
    pub product: HashMap<u64, ProductHolder>,
    pub product_definition_formation: HashMap<u64, ProductDefinitionFormationHolder>,
    pub product_definition_context: HashMap<u64, ProductDefinitionContextHolder>,
    pub product_definition: HashMap<u64, ProductDefinitionHolder>,
    pub product_definition_shape: HashMap<u64, ProductDefinitionShapeHolder>,
    pub shape_definition_representation: HashMap<u64, ShapeDefinitionRepresentationHolder>,
    pub shape_representation: HashMap<u64, ShapeRepresentationHolder>,
    pub context_dependent_shape_representation:
        HashMap<u64, ContextDependentShapeRepresentationHolder>,
    pub shape_representation_relationship: HashMap<u64, ShapeRepresentationRelationshipHolder>,
    pub shape_representation_relationship_with_transformation:
        HashMap<u64, ShapeRepresentationRelationshipWithTransformationHolder>,
    pub next_assembly_usage_occurrence: HashMap<u64, NextAssemblyUsageOccurrenceHolder>,
    pub item_defined_transformation: HashMap<u64, ItemDefinedTransformationHolder>,

    // geometric sets
    pub geometric_curve_set: HashMap<u64, GeometricCurveSetHolder>,

    // others
    pub definitional_representation: HashMap<u64, DefinitionalRepresentationHolder>,

    // dummy
    pub dummy: HashMap<u64, DummyHolder>,
}

impl Table {
    pub fn push_instance(&mut self, instance: &EntityInstance) -> ruststep::error::Result<()> {
        match instance {
            EntityInstance::Simple { id, record } => match record.name.as_str() {
                "CARTESIAN_POINT" => {
                    self.cartesian_point
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "DIRECTION" => {
                    self.direction
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "VECTOR" => {
                    self.vector.insert(*id, Deserialize::deserialize(record)?);
                }
                "PLACEMENT" => {
                    self.placement
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "AXIS1_PLACEMENT" => {
                    self.axis1_placement
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "AXIS2_PLACEMENT_2D" => {
                    self.axis2_placement_2d
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "AXIS2_PLACEMENT_3D" => {
                    self.axis2_placement_3d
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "LINE" => {
                    self.line
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "POLYLINE" => {
                    self.polyline.insert(*id, Deserialize::deserialize(record)?);
                }
                "B_SPLINE_CURVE_WITH_KNOTS" => {
                    self.b_spline_curve_with_knots
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "BEZIER_CURVE" => {
                    self.bezier_curve
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "QUASI_UNIFORM_CURVE" => {
                    self.quasi_uniform_curve
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "UNIFORM_CURVE" => {
                    self.uniform_curve
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "CIRCLE" => {
                    self.circle.insert(*id, Deserialize::deserialize(record)?);
                }
                "ELLIPSE" => {
                    self.ellipse.insert(*id, Deserialize::deserialize(record)?);
                }
                "HYPERBOLA" => {
                    self.hyperbola
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "PARABOLA" => {
                    self.parabola.insert(*id, Deserialize::deserialize(record)?);
                }
                "PCURVE" => {
                    self.pcurve.insert(*id, Deserialize::deserialize(record)?);
                }
                "SURFACE_CURVE" => {
                    let curve: SurfaceCurveParams = Deserialize::deserialize(&record.parameter)?;
                    self.surface_curve.insert(
                        *id,
                        curve.into_holder(step_types::SurfaceCurveKind::Surface),
                    );
                }
                "SEAM_CURVE" => {
                    let curve: SurfaceCurveParams = Deserialize::deserialize(&record.parameter)?;
                    self.surface_curve
                        .insert(*id, curve.into_holder(step_types::SurfaceCurveKind::Seam));
                }
                "INTERSECTION_CURVE" => {
                    let curve: SurfaceCurveParams = Deserialize::deserialize(&record.parameter)?;
                    self.surface_curve.insert(
                        *id,
                        curve.into_holder(step_types::SurfaceCurveKind::Intersection),
                    );
                }
                "PLANE" => {
                    self.plane.insert(*id, Deserialize::deserialize(record)?);
                }
                "SPHERICAL_SURFACE" => {
                    self.spherical_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "CYLINDRICAL_SURFACE" => {
                    self.cylindrical_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "TOROIDAL_SURFACE" => {
                    self.toroidal_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "CONICAL_SURFACE" => {
                    self.conical_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "B_SPLINE_SURFACE_WITH_KNOTS" => {
                    self.b_spline_surface_with_knots
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "UNIFORM_SURFACE" => {
                    self.uniform_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "QUASI_UNIFORM_SURFACE" => {
                    self.quasi_uniform_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "BEZIER_SURFACE" => {
                    self.bezier_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "SURFACE_OF_LINEAR_EXTRUSION" => {
                    self.surface_of_linear_extrusion
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "SURFACE_OF_REVOLUTION" => {
                    self.surface_of_revolution
                        .insert(*id, Deserialize::deserialize(record)?);
                }

                "VERTEX_POINT" => {
                    self.vertex_point
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "EDGE_CURVE" => {
                    self.edge_curve
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "ORIENTED_EDGE" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 5
                    {
                        self.oriented_edge.insert(
                            *id,
                            OrientedEdgeHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                edge_element: Deserialize::deserialize(&params[3])?,
                                orientation: Deserialize::deserialize(&params[4])?,
                            },
                        );
                    }
                }
                "EDGE_LOOP" => {
                    self.edge_loop
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "FACE_BOUND" => {
                    self.face_bound
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "FACE_OUTER_BOUND" => {
                    self.face_bound
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "FACE_SURFACE" => {
                    self.face_surface
                        .insert(*id, Deserialize::deserialize(record)?);
                }
                "ADVANCED_FACE" => {
                    self.face_surface
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "ORIENTED_FACE" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 4
                    {
                        self.oriented_face.insert(
                            *id,
                            OrientedFaceHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                face_element: Deserialize::deserialize(&params[2])?,
                                orientation: Deserialize::deserialize(&params[3])?,
                            },
                        );
                    }
                }
                "OPEN_SHELL" => {
                    self.shell
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "CLOSED_SHELL" => {
                    self.shell
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "ORIENTED_OPEN_SHELL" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 4
                    {
                        self.oriented_shell.insert(
                            *id,
                            OrientedShellHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                shell_element: Deserialize::deserialize(&params[2])?,
                                orientation: Deserialize::deserialize(&params[3])?,
                            },
                        );
                    }
                }
                "ORIENTED_CLOSED_SHELL" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 4
                    {
                        self.oriented_shell.insert(
                            *id,
                            OrientedShellHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                shell_element: Deserialize::deserialize(&params[2])?,
                                orientation: Deserialize::deserialize(&params[3])?,
                            },
                        );
                    }
                }
                "SHELL_BASED_SURFACE_MODEL" => {
                    self.shell_based_surface_model
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "MANIFOLD_SOLID_BREP" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 2
                    {
                        self.manifold_solid_brep.insert(
                            *id,
                            ManifoldSolidBrepHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                outer: Deserialize::deserialize(&params[1])?,
                                voids: Vec::new(),
                            },
                        );
                    }
                }
                "BREP_WITH_VOIDS" => {
                    self.manifold_solid_brep
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "GEOMETRIC_CURVE_SET" => {
                    self.geometric_curve_set
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "DEFINITIONAL_REPRESENTATION" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() == 3
                    {
                        self.definitional_representation.insert(
                            *id,
                            DefinitionalRepresentationHolder {
                                label: Deserialize::deserialize(&params[0])?,
                                representation_item: Deserialize::deserialize(&params[1])?,
                                context_of_items: match &params[2] {
                                    Parameter::Ref(x) => PlaceHolder::Ref(x.clone()),
                                    _ => PlaceHolder::Owned(DummyHolder {
                                        record: format!("{:?}", params[2]),
                                        is_simple: true,
                                    }),
                                },
                            },
                        );
                    }
                }
                "APPLICATION_CONTEXT" => {
                    self.application_context
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT_CONTEXT" => {
                    self.product_context
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT" => {
                    self.product
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT_DEFINITION_FORMATION" => {
                    self.product_definition_formation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE" => {
                    if let Parameter::List(params) = &record.parameter
                        && params.len() >= 3
                    {
                        self.product_definition_formation.insert(
                            *id,
                            ProductDefinitionFormationHolder {
                                id: Deserialize::deserialize(&params[0])?,
                                description: Deserialize::deserialize(&params[1])?,
                                of_product: Deserialize::deserialize(&params[2])?,
                            },
                        );
                    }
                }
                "PRODUCT_DEFINITION_CONTEXT" => {
                    self.product_definition_context
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT_DEFINITION" => {
                    self.product_definition
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "PRODUCT_DEFINITION_SHAPE" => {
                    self.product_definition_shape
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "SHAPE_DEFINITION_REPRESENTATION" => {
                    self.shape_definition_representation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "SHAPE_REPRESENTATION" => {
                    self.shape_representation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "ADVANCED_BREP_SHAPE_REPRESENTATION" => {
                    self.shape_representation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => {
                    self.context_dependent_shape_representation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "SHAPE_REPRESENTATION_RELATIONSHIP" => {
                    self.shape_representation_relationship
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                    self.next_assembly_usage_occurrence
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                "ITEM_DEFINED_TRANSFORMATION" => {
                    self.item_defined_transformation
                        .insert(*id, Deserialize::deserialize(&record.parameter)?);
                }
                _ => {
                    self.dummy.insert(
                        *id,
                        DummyHolder {
                            record: format!("{record:?}"),
                            is_simple: true,
                        },
                    );
                }
            },
            EntityInstance::Complex {
                id,
                subsuper: SubSuperRecord(records),
            } => {
                use NonRationalBsplineCurveHolder as NRBC;
                use NonRationalBsplineSurfaceHolder as NRBS;
                if records.len() == 7 {
                    match (
                        records[0].name.as_str(),
                        &records[0].parameter,
                        records[1].name.as_str(),
                        &records[1].parameter,
                        records[2].name.as_str(),
                        &records[2].parameter,
                        records[3].name.as_str(),
                        &records[3].parameter,
                        records[4].name.as_str(),
                        &records[4].parameter,
                        records[5].name.as_str(),
                        &records[5].parameter,
                        records[6].name.as_str(),
                        &records[6].parameter,
                    ) {
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "B_SPLINE_CURVE_WITH_KNOTS",
                            Parameter::List(knots_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            params.extend(knots_params.clone());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBsplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::BsplineCurveWithKnots(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BEZIER_CURVE",
                            _,
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBsplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::BezierCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "QUASI_UNIFORM_CURVE",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = vec![label[0].clone()];
                            params.extend(bsp_params.iter().cloned());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBsplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::QuasiUniformCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "UNIFORM_CURVE",
                            _,
                        ) => {
                            let mut params = vec![label[0].clone()];
                            params.extend(bsp_params.iter().cloned());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBsplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::UniformCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "B_SPLINE_SURFACE_WITH_KNOTS",
                            Parameter::List(knots_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            params.extend(knots_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBsplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::BsplineSurfaceWithKnots(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BEZIER_SURFACE",
                            _,
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBsplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::BezierSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "QUASI_UNIFORM_SURFACE",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBsplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::QuasiUniformSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                            "UNIFORM_SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBsplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::UniformSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        _ => {
                            self.dummy.insert(
                                *id,
                                DummyHolder {
                                    record: format!("{records:?}"),
                                    is_simple: false,
                                },
                            );
                        }
                    }
                } else if records.len() == 3 {
                    match (
                        records[0].name.as_str(),
                        &records[0].parameter,
                        records[1].name.as_str(),
                        &records[1].parameter,
                        records[2].name.as_str(),
                        &records[2].parameter,
                    ) {
                        (
                            "REPRESENTATION_RELATIONSHIP",
                            Parameter::List(rr_parameter),
                            "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                            Parameter::List(transformation),
                            "SHAPE_REPRESENTATION_RELATIONSHIP",
                            _,
                        ) => {
                            let entity = ShapeRepresentationRelationshipWithTransformationHolder {
                                name: Deserialize::deserialize(&rr_parameter[0])?,
                                description: Deserialize::deserialize(&rr_parameter[1])?,
                                rep_1: Deserialize::deserialize(&rr_parameter[2])?,
                                rep_2: Deserialize::deserialize(&rr_parameter[3])?,
                                transformation_operator: Deserialize::deserialize(
                                    &transformation[0],
                                )?,
                            };
                            self.shape_representation_relationship_with_transformation
                                .insert(*id, entity);
                        }
                        _ => {
                            self.dummy.insert(
                                *id,
                                DummyHolder {
                                    record: format!("{records:?}"),
                                    is_simple: false,
                                },
                            );
                        }
                    }
                } else {
                    self.dummy.insert(
                        *id,
                        DummyHolder {
                            record: format!("{records:?}"),
                            is_simple: false,
                        },
                    );
                }
            }
        }
        Ok(())
    }
    #[inline(always)]
    pub fn from_data_section(data_section: &DataSection) -> Table {
        Table::from_iter(&data_section.entities)
    }
    #[inline(always)]
    pub fn from_step(step_str: &str) -> Option<Table> {
        let exchange = ruststep::parser::parse(step_str).ok()?;
        Some(Table::from_data_section(&exchange.data[0]))
    }
}

impl<'a> FromIterator<&'a EntityInstance> for Table {
    fn from_iter<I: IntoIterator<Item = &'a EntityInstance>>(iter: I) -> Table {
        let mut res = Table::default();
        iter.into_iter().for_each(|instance| {
            res.push_instance(instance)
                .unwrap_or_else(|e| eprintln!("{e}"))
        });
        res
    }
}
