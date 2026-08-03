//! Imported solve, topology replacement, meshing, and STEP round trip.

use super::Args;
use super::certify::{Certificate, CertificationConfig, certify};
use super::classify::{ImportedShell, select_full_nurbs_seam, to_nurbs};
use super::errors::ValidationError;
use super::mesh_validation::{MeshEvidence, MeshValidationConfig, validate_mesh};
use super::persistence::PersistenceEvidence;
use anyhow::{Context, Result, anyhow};
use monstertruck_geometry::nurbs::continuity::{
    BoundaryAlignment, ContinuityOrder, SurfaceBoundary,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, BoundaryContinuitySolver, BoundaryTransition, ContinuitySolveReport,
    ContinuitySolverConfig, ContinuityTermination, OrderResidual,
};
use monstertruck_geometry::prelude::{NurbsSurface, Vector4};
use monstertruck_step::load::step_geometry::Surface;
use monstertruck_step::load::{Table, step_geometry};
use monstertruck_step::save::{CompleteStepDisplay, StepHeaderDescriptor, StepModel};
use serde::Serialize;
use std::fs;

pub(super) fn execute(args: &Args) -> Result<()> {
    validate_args(args)?;
    let source = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let table = Table::from_step(&source).context("failed to parse imported STEP")?;
    let imported = load_shell(&table, args.shell)?;
    let selection = select_full_nurbs_seam(&imported, args.classification_tolerance)?;
    let mut solved_shell = imported.clone();
    let first = to_nurbs(&imported.faces[selection.first_face].surface)
        .ok_or(ValidationError::InsufficientNurbsFaces(0))?;
    let original_second = to_nurbs(&imported.faces[selection.second_face].surface)
        .ok_or(ValidationError::InsufficientNurbsFaces(0))?;
    let perturbed_second = perturb_boundary_strip(
        &original_second,
        selection.second_boundary,
        args.perturbation,
    )?;
    let order = continuity_order(args.order)?;
    let config = ContinuitySolverConfig::default()
        .with_anchor_weight(0.0)
        .with_fairness_weight(0.0)
        .with_max_iterations(80);
    let request = BoundaryContinuityRequest::new(
        selection.first_boundary,
        selection.second_boundary,
        selection.alignment,
        order,
    );
    let solution = BoundaryContinuitySolver::new(config)
        .context("failed to construct continuity solver")?
        .solve(&first, &perturbed_second, request)
        .context("continuity solver rejected or failed the imported fixture")?;
    let maximum_residual_by_order = maximum_residual_by_order(args);
    let certificate = certify(
        solution.first(),
        solution.second(),
        solution.transition(),
        selection,
        order,
        CertificationConfig {
            intervals: args.certification_intervals,
            normalized_step: args.certification_step,
            stencil_radius: args.certification_stencil_radius,
            maximum_residual_by_order: &maximum_residual_by_order,
            maximum_normal_angle: args.tangent_tolerance,
        },
    )?;
    solved_shell.faces[selection.second_face].surface =
        Surface::NurbsSurface(solution.second().clone());
    let mesh = validate_mesh(&solved_shell, mesh_validation_config(args))?;
    let output_step = export_step(&solved_shell);
    let reimport = validate_reimport(
        &output_step,
        args,
        order,
        solution.transition(),
        &solved_shell,
        &maximum_residual_by_order,
    )?;
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&args.output, &output_step)
        .with_context(|| format!("failed to write {}", args.output.display()))?;
    write_receipt(
        args,
        selection,
        &certificate,
        solution.report(),
        &mesh,
        &reimport,
    )?;
    println!(
        "status=validated input={} shell={} faces={}/{} boundaries={:?}/{:?} \
         alignment={:?} order=G{} classification_maximum={:.6e} \
         certificate_samples={} normalized_residuals={:?} normal_angle={:.6e} \
         post_reimport_residuals={:?} bounding_box_drift={:.6e} \
         triangles={} minimum_normalized_double_area={:.6e} \
         minimum_normal_alignment={:.6e} post_reimport_triangles={} output={}",
        args.input.display(),
        args.shell,
        selection.first_face + 1,
        selection.second_face + 1,
        selection.first_boundary,
        selection.second_boundary,
        selection.alignment,
        args.order,
        selection.classification_maximum,
        certificate.samples,
        certificate.maximum_normalized_residual_by_order,
        certificate.maximum_normal_angle,
        reimport.certificate.maximum_normalized_residual_by_order,
        reimport.persistence.bounding_box_normalized_maximum_drift(),
        mesh.triangle_count(),
        mesh.minimum_normalized_double_area(),
        mesh.minimum_normal_alignment(),
        reimport.mesh.triangle_count(),
        args.output.display(),
    );
    Ok(())
}

fn validate_args(args: &Args) -> Result<(), ValidationError> {
    [
        ("perturbation", args.perturbation),
        ("classification_tolerance", args.classification_tolerance),
        ("position_tolerance", args.position_tolerance),
        (
            "first_derivative_tolerance",
            args.first_derivative_tolerance,
        ),
        (
            "second_derivative_tolerance",
            args.second_derivative_tolerance,
        ),
        (
            "third_derivative_tolerance",
            args.third_derivative_tolerance,
        ),
        ("tangent_tolerance", args.tangent_tolerance),
        ("certification_step", args.certification_step),
        ("mesh_tolerance", args.mesh_tolerance),
        ("triangle_area_tolerance", args.triangle_area_tolerance),
        ("bounding_box_tolerance", args.bounding_box_tolerance),
    ]
    .into_iter()
    .find(|(_, value)| !value.is_finite() || *value <= 0.0)
    .map_or(Ok(()), |(name, _)| {
        Err(ValidationError::InvalidTolerance { name })
    })?;
    if !args.minimum_triangle_normal_alignment.is_finite()
        || !(0.0..=1.0).contains(&args.minimum_triangle_normal_alignment)
        || args.minimum_triangle_normal_alignment == 0.0
    {
        Err(ValidationError::InvalidTolerance {
            name: "minimum_triangle_normal_alignment",
        })
    } else {
        Ok(())
    }
}

#[derive(Serialize)]
struct ValidationReceipt<'a> {
    schema_version: u32,
    evidence_class: &'static str,
    input: String,
    output: String,
    order: u8,
    perturbation: f64,
    selection: SelectionReceipt,
    certificate_before_export: &'a Certificate,
    solver: SolverReceipt<'a>,
    mesh_before_export: &'a MeshEvidence,
    reimport: ReimportReceipt<'a>,
}

#[derive(Serialize)]
struct ReimportReceipt<'a> {
    selection: SelectionReceipt,
    certificate: &'a Certificate,
    persistence: &'a PersistenceEvidence,
    mesh: &'a MeshEvidence,
}

struct ReimportEvidence {
    selection: super::classify::SeamSelection,
    certificate: Certificate,
    persistence: PersistenceEvidence,
    mesh: MeshEvidence,
}

#[derive(Serialize)]
struct SelectionReceipt {
    first_face: usize,
    second_face: usize,
    first_boundary: SurfaceBoundary,
    second_boundary: SurfaceBoundary,
    alignment: BoundaryAlignment,
    classification_maximum: f64,
}

impl From<super::classify::SeamSelection> for SelectionReceipt {
    fn from(selection: super::classify::SeamSelection) -> Self {
        Self {
            first_face: selection.first_face + 1,
            second_face: selection.second_face + 1,
            first_boundary: selection.first_boundary,
            second_boundary: selection.second_boundary,
            alignment: selection.alignment,
            classification_maximum: selection.classification_maximum,
        }
    }
}

#[derive(Serialize)]
struct SolverReceipt<'a> {
    termination: ContinuityTermination,
    iterations: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    initial_objective: f64,
    final_objective: f64,
    residuals: &'a [OrderResidual],
    numerical_rank: usize,
    variable_count: usize,
    residual_count: usize,
}

impl<'a> From<&'a ContinuitySolveReport> for SolverReceipt<'a> {
    fn from(report: &'a ContinuitySolveReport) -> Self {
        Self {
            termination: report.termination(),
            iterations: report.iterations(),
            accepted_steps: report.accepted_steps(),
            rejected_steps: report.rejected_steps(),
            initial_objective: report.initial_objective(),
            final_objective: report.final_objective(),
            residuals: report.residuals(),
            numerical_rank: report.numerical_rank(),
            variable_count: report.variable_count(),
            residual_count: report.residual_count(),
        }
    }
}

fn write_receipt(
    args: &Args,
    selection: super::classify::SeamSelection,
    certificate: &Certificate,
    solver_report: &ContinuitySolveReport,
    mesh: &MeshEvidence,
    reimport: &ReimportEvidence,
) -> Result<()> {
    args.receipt.as_ref().map_or(Ok(()), |path| {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let receipt = ValidationReceipt {
            schema_version: 3,
            evidence_class: "imported_workflow_independent_dense_certificate",
            input: args.input.display().to_string(),
            output: args.output.display().to_string(),
            order: args.order,
            perturbation: args.perturbation,
            selection: selection.into(),
            certificate_before_export: certificate,
            solver: solver_report.into(),
            mesh_before_export: mesh,
            reimport: ReimportReceipt {
                selection: reimport.selection.into(),
                certificate: &reimport.certificate,
                persistence: &reimport.persistence,
                mesh: &reimport.mesh,
            },
        };
        let json = serde_json::to_string_pretty(&receipt).context("failed to serialize receipt")?;
        fs::write(path, format!("{json}\n"))
            .with_context(|| format!("failed to write {}", path.display()))
    })
}

fn load_shell(table: &Table, one_based_shell: usize) -> Result<ImportedShell> {
    if one_based_shell == 0 {
        Err(ValidationError::ZeroShellIndex.into())
    } else {
        let mut entries = table.shell.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(id, _)| *id);
        let available = entries.len();
        let (_, holder) =
            entries
                .get(one_based_shell - 1)
                .ok_or(ValidationError::ShellNotFound {
                    requested: one_based_shell,
                    available,
                })?;
        table
            .to_compressed_trimmed_shell(*holder)
            .map_err(|error| anyhow!("failed to convert imported STEP shell: {error}"))
    }
}

fn continuity_order(order: u8) -> Result<ContinuityOrder, ValidationError> {
    match order {
        1 => Ok(ContinuityOrder::G1),
        2 => Ok(ContinuityOrder::G2),
        3 => Ok(ContinuityOrder::G3),
        _ => Err(ValidationError::UnsupportedOrder(order)),
    }
}

fn maximum_residual_by_order(args: &Args) -> [f64; 4] {
    [
        args.position_tolerance,
        args.first_derivative_tolerance,
        args.second_derivative_tolerance,
        args.third_derivative_tolerance,
    ]
}

fn perturb_boundary_strip(
    surface: &NurbsSurface<Vector4>,
    boundary: SurfaceBoundary,
    amplitude: f64,
) -> Result<NurbsSurface<Vector4>, ValidationError> {
    let mut perturbed = surface.clone();
    let rows = surface.control_points().len();
    let columns = surface.control_points().first().map_or(0, Vec::len);
    let available = match boundary {
        SurfaceBoundary::UStart | SurfaceBoundary::UEnd => rows,
        SurfaceBoundary::VStart | SurfaceBoundary::VEnd => columns,
    };
    if available < 2 {
        Err(ValidationError::InsufficientBoundaryStrip {
            boundary,
            available,
            required: 2,
        })
    } else {
        (0..2).for_each(|distance| {
            let factor = [1.0, -0.5][distance];
            let along_count = match boundary {
                SurfaceBoundary::UStart | SurfaceBoundary::UEnd => columns,
                SurfaceBoundary::VStart | SurfaceBoundary::VEnd => rows,
            };
            (0..along_count).for_each(|along| {
                let (row, column) = control_point_index(rows, columns, boundary, distance, along);
                let point = perturbed.control_point_mut(row, column);
                point.z += amplitude * factor * point.w;
            });
        });
        Ok(perturbed)
    }
}

fn control_point_index(
    rows: usize,
    columns: usize,
    boundary: SurfaceBoundary,
    distance: usize,
    along: usize,
) -> (usize, usize) {
    match boundary {
        SurfaceBoundary::UStart => (distance, along),
        SurfaceBoundary::UEnd => (rows - 1 - distance, along),
        SurfaceBoundary::VStart => (along, distance),
        SurfaceBoundary::VEnd => (along, columns - 1 - distance),
    }
}

fn export_step(shell: &ImportedShell) -> String {
    CompleteStepDisplay::new(
        StepModel::from(shell),
        StepHeaderDescriptor {
            file_name: "continuity-step-validation.step".to_owned(),
            time_stamp: "2000-01-01T00:00:00".to_owned(),
            organization_system: "monstertruck Phase 5 continuity validation".to_owned(),
            ..Default::default()
        },
    )
    .to_string()
}

fn validate_reimport(
    step: &str,
    args: &Args,
    order: ContinuityOrder,
    transition: &BoundaryTransition,
    before_export: &ImportedShell,
    maximum_residual_by_order: &[f64; 4],
) -> Result<ReimportEvidence, ValidationError> {
    let table = Table::from_step(step).map_err(|_| ValidationError::EmptyReimport)?;
    let shell = load_shell(&table, args.shell).map_err(|_| ValidationError::EmptyReimport)?;
    let spline_count = shell
        .faces
        .iter()
        .filter(|face| {
            matches!(
                face.surface,
                step_geometry::Surface::BsplineSurface(_) | step_geometry::Surface::NurbsSurface(_)
            )
        })
        .count();
    if spline_count < 2 {
        Err(ValidationError::ReimportLostNurbsFaces)
    } else {
        let persistence =
            PersistenceEvidence::compare(before_export, &shell, args.bounding_box_tolerance)?;
        let mesh = validate_mesh(&shell, mesh_validation_config(args))?;
        let selection = select_full_nurbs_seam(&shell, args.classification_tolerance)?;
        let first = to_nurbs(&shell.faces[selection.first_face].surface)
            .ok_or(ValidationError::InsufficientNurbsFaces(0))?;
        let second = to_nurbs(&shell.faces[selection.second_face].surface)
            .ok_or(ValidationError::InsufficientNurbsFaces(0))?;
        let certificate = certify(
            &first,
            &second,
            transition,
            selection,
            order,
            CertificationConfig {
                intervals: args.certification_intervals,
                normalized_step: args.certification_step,
                stencil_radius: args.certification_stencil_radius,
                maximum_residual_by_order,
                maximum_normal_angle: args.tangent_tolerance,
            },
        )?;
        Ok(ReimportEvidence {
            selection,
            certificate,
            persistence,
            mesh,
        })
    }
}

fn mesh_validation_config(args: &Args) -> MeshValidationConfig {
    MeshValidationConfig {
        tessellation_tolerance: args.mesh_tolerance,
        normalized_double_area_tolerance: args.triangle_area_tolerance,
        minimum_normal_alignment: args.minimum_triangle_normal_alignment,
    }
}
