use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::slice;

use anyhow::{Context, Result, anyhow, bail, ensure};
use monstertruck_core::{
    FeatureId, SemanticLabel, SemanticTopologyRef, TopologyKind, TrackingId, TrackingSession,
    TrackingSessionId,
};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{ContinuityOrder, SurfaceBoundary};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, BoundaryEndpoint, ContinuityReplayError,
    ContinuityReplayExecutionError, ContinuitySolveError, ContinuitySolverConfig,
    ContinuityTermination, execute_boundary_continuity_contracts,
};
use monstertruck_geometry::nurbs::contract::{
    BoundaryAlignment, ContinuityContract, ContractId, SurfaceBoundaryRef,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};
use serde::Serialize;

#[derive(Serialize)]
struct Receipt {
    schema_version: u32,
    base_revision: String,
    harness_sha256: String,
    toolchain: String,
    target: String,
    solver_configurations: BTreeMap<&'static str, &'static str>,
    generated_input: &'static str,
    bounded_batch_basis: &'static str,
    solver_tripwire: SolverTripwireEvidence,
    cases: BTreeMap<&'static str, CaseEvidence>,
}

#[derive(Serialize)]
struct SolverTripwireEvidence {
    kind: &'static str,
    endpoint: BoundaryEndpoint,
    row: usize,
    column: usize,
    weight: f64,
}

#[derive(Serialize)]
struct CaseEvidence {
    story: &'static str,
    error: ErrorEvidence,
    repeated_error_equal: bool,
    geometry_unchanged: bool,
    tracking_unchanged: bool,
    contracts_unchanged: bool,
    execution: ExecutionEvidence,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExecutionEvidence {
    RejectedBeforeSolve,
    PreparationContradiction {
        reversed_order_error_equal: bool,
        individual_contracts_succeeded: Vec<bool>,
    },
    BoundedSolveFailure {
        returned_solution: bool,
        individual_contracts_succeeded: Vec<bool>,
        input_contract_order: Vec<String>,
        expected_dependency_order: Vec<String>,
        prefix_repeated_equal: bool,
        staged_prefix: StagedPrefixEvidence,
    },
    LateSolveFailure {
        returned_solution: bool,
        prefix_repeated_equal: bool,
        input_contract_order: Vec<String>,
        expected_dependency_order: Vec<String>,
        staged_prefix: StagedPrefixEvidence,
    },
}

#[derive(Serialize)]
struct StagedPrefixEvidence {
    contract_id: String,
    solve_count: usize,
    geometry_changed: bool,
    surfaces_equal_across_reruns: bool,
    transitions_equal_across_reruns: bool,
    reports_equal_across_reruns: bool,
    transition_order: ContinuityOrder,
    mapped_coordinates: Option<(f64, f64)>,
    termination: ContinuityTermination,
    iterations: usize,
    accepted_steps: usize,
    rejected_steps: usize,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ErrorEvidence {
    SameSurfaceContract {
        contract_id: String,
        tracking_id: String,
    },
    CoupledOptimizedSurface {
        tracking_id: String,
        first_contract: String,
        second_contract: String,
    },
    DependencyCycle {
        contracts: Vec<String>,
    },
    DuplicateContractId {
        contract_id: String,
    },
    DidNotConverge {
        contract_id: String,
        termination: ContinuityTermination,
        iterations: usize,
        accepted_steps: usize,
        rejected_steps: usize,
    },
    NonPositiveWeight {
        contract_id: String,
        endpoint: BoundaryEndpoint,
        row: usize,
        column: usize,
        weight: f64,
    },
}

struct Options {
    base_revision: String,
    harness_sha256: String,
    toolchain: String,
    receipt: PathBuf,
}

fn main() -> Result<()> {
    let options = options()?;
    let solver = BoundaryContinuitySolver::new(ContinuitySolverConfig::default())?;
    let bounded_solver =
        BoundaryContinuitySolver::new(ContinuitySolverConfig::default().with_max_iterations(1))?;
    let receipt = Receipt {
        schema_version: 2,
        base_revision: options.base_revision,
        harness_sha256: options.harness_sha256,
        toolchain: options.toolchain,
        target: format!("{}-{}", env::consts::ARCH, env::consts::OS),
        solver_configurations: BTreeMap::from([
            ("default", "ContinuitySolverConfig::default()"),
            (
                "bounded_nonconvergence",
                "ContinuitySolverConfig::default().with_max_iterations(1)",
            ),
        ]),
        generated_input: "deterministic bilinear and quintic tensor-product patches",
        bounded_batch_basis: "generated from resources/continuity-validation/v1/corpus.json fixture_version multispan_quintic_curved_seam_v2 with a 0.05 upstream G0 seam offset and a one-iteration batch budget",
        solver_tripwire: validate_solver_tripwire(&solver)?,
        cases: BTreeMap::from([
            (
                "bounded_nonconvergence",
                bounded_nonconvergence(&bounded_solver)?,
            ),
            (
                "coupled_optimized_surface",
                coupled_optimized_surface(&solver)?,
            ),
            ("dependency_cycle_three", dependency_cycle_three(&solver)?),
            ("dependency_cycle_two", dependency_cycle_two(&solver)?),
            ("duplicate_contract_id", duplicate_contract_id(&solver)?),
            ("late_dependency_failure", late_dependency_failure(&solver)?),
            ("same_surface_contract", same_surface_contract(&solver)?),
        ]),
    };
    if let Some(parent) = options.receipt.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.receipt, serde_json::to_vec_pretty(&receipt)?)
        .with_context(|| format!("failed to write {}", options.receipt.display()))?;
    println!("wrote {}", options.receipt.display());
    Ok(())
}

fn validate_solver_tripwire(solver: &BoundaryContinuitySolver) -> Result<SolverTripwireEvidence> {
    let references = [
        semantic("replay-validation", "tripwire-master")?,
        semantic("replay-validation", "tripwire-dependent")?,
    ];
    let contract = contract(
        "solver-tripwire",
        references[0].clone(),
        SurfaceBoundary::UEnd,
        references[1].clone(),
        SurfaceBoundary::UStart,
    )?;
    let (session, ids) = session_with_bindings(&references)?;
    let surfaces = invalid_surfaces(&ids);
    let error = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &[contract]),
        "the acyclic invalid-weight batch must reach the solver tripwire",
    )?;
    match error {
        ContinuityReplayExecutionError::Solve {
            source:
                ContinuitySolveError::NonPositiveWeight {
                    endpoint,
                    row,
                    column,
                    weight,
                },
            ..
        } => Ok(SolverTripwireEvidence {
            kind: "non_positive_weight",
            endpoint,
            row,
            column,
            weight,
        }),
        _ => Err(anyhow!("expected the solver tripwire, got {error}")),
    }
}

fn duplicate_contract_id(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let first_master = semantic("replay-validation", "duplicate-first-master")?;
    let first_dependent = semantic("replay-validation", "duplicate-first-dependent")?;
    let second_master = semantic("replay-validation", "duplicate-second-master")?;
    let second_dependent = semantic("replay-validation", "duplicate-second-dependent")?;
    let references = [
        first_master.clone(),
        first_dependent.clone(),
        second_master.clone(),
        second_dependent.clone(),
    ];
    let first_contract = contract(
        "duplicate-contract",
        first_master,
        SurfaceBoundary::UEnd,
        first_dependent,
        SurfaceBoundary::UStart,
    )?;
    let second_contract = contract(
        "duplicate-contract",
        second_master,
        SurfaceBoundary::UEnd,
        second_dependent,
        SurfaceBoundary::UStart,
    )?;
    let contracts = [first_contract.clone(), second_contract.clone()];
    let reversed_contracts = [second_contract, first_contract];
    let original_contracts = contracts.clone();
    let original_reversed_contracts = reversed_contracts.clone();
    let (session, ids) = session_with_bindings(&references)?;
    let surfaces = valid_surfaces(&ids);
    let original_session = session.clone();
    let original_surfaces = surfaces.clone();
    let first = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "duplicate contract IDs must reject the batch",
    )?;
    let second = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "repeated duplicate contract IDs must reject the batch",
    )?;
    let reversed = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &reversed_contracts),
        "reversed duplicate contract IDs must reject the batch",
    )?;
    ensure!(
        first == second,
        "repeated execution returned a different error"
    );
    ensure!(
        first == reversed,
        "contract input order changed the duplicate-ID error"
    );
    ensure!(
        session == original_session,
        "replay changed the tracking session"
    );
    ensure!(
        surfaces == original_surfaces,
        "replay changed the geometry map"
    );
    ensure!(
        contracts == original_contracts && reversed_contracts == original_reversed_contracts,
        "replay changed the contract inputs"
    );
    let individual_contracts_succeeded = contracts
        .iter()
        .map(|contract| {
            execute_boundary_continuity_contracts(
                solver,
                &session,
                &surfaces,
                slice::from_ref(contract),
            )
            .map(|_| true)
            .map_err(anyhow::Error::from)
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        individual_contracts_succeeded.iter().all(|&value| value),
        "a duplicate-ID contract did not succeed independently"
    );
    let error = match first {
        ContinuityReplayExecutionError::Preparation(
            ContinuityReplayError::DuplicateContractId { contract_id },
        ) => ErrorEvidence::DuplicateContractId {
            contract_id: contract_id.as_str().to_owned(),
        },
        _ => {
            return Err(anyhow!(
                "expected duplicate-contract-ID rejection, got {first}"
            ));
        }
    };
    Ok(CaseEvidence {
        story: "MT-304",
        error,
        repeated_error_equal: true,
        geometry_unchanged: true,
        tracking_unchanged: true,
        contracts_unchanged: true,
        execution: ExecutionEvidence::PreparationContradiction {
            reversed_order_error_equal: true,
            individual_contracts_succeeded,
        },
    })
}

fn bounded_nonconvergence(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let upstream_master_ref = semantic("replay-validation", "bounded-upstream-master")?;
    let middle_ref = semantic("replay-validation", "bounded-middle")?;
    let dependent_ref = semantic("replay-validation", "bounded-dependent")?;
    let references = [
        upstream_master_ref.clone(),
        middle_ref.clone(),
        dependent_ref.clone(),
    ];
    let upstream = contract_with(
        "z-bounded-upstream",
        upstream_master_ref,
        SurfaceBoundary::UEnd,
        middle_ref.clone(),
        SurfaceBoundary::UEnd,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G0,
    )?;
    let downstream = contract_with(
        "a-bounded-downstream",
        middle_ref,
        SurfaceBoundary::UEnd,
        dependent_ref,
        SurfaceBoundary::UStart,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G3,
    )?;
    let contracts = [downstream.clone(), upstream.clone()];
    let original_contracts = contracts.clone();
    let (session, ids) = session_with_bindings(&references)?;
    let [upstream_master_id, middle_id, dependent_id]: [TrackingId; 3] = ids
        .try_into()
        .map_err(|_| anyhow!("the bounded batch requires three tracking IDs"))?;
    let (middle, dependent) = exact_quintic_surfaces()?;
    let mut upstream_master = middle.clone();
    let boundary_row = upstream_master
        .control_points()
        .len()
        .checked_sub(1)
        .ok_or_else(|| anyhow!("the upstream master requires a boundary row"))?;
    let boundary_columns = upstream_master
        .control_points()
        .last()
        .ok_or_else(|| anyhow!("the upstream master requires a boundary row"))?
        .len();
    (0..boundary_columns).for_each(|column| {
        upstream_master.control_point_mut(boundary_row, column).z += 0.05;
    });
    let surfaces = BTreeMap::from([
        (upstream_master_id, upstream_master),
        (middle_id.clone(), middle),
        (dependent_id, dependent),
    ]);
    let original_session = session.clone();
    let original_surfaces = surfaces.clone();
    let upstream_prefix = execute_boundary_continuity_contracts(
        solver,
        &session,
        &surfaces,
        slice::from_ref(&upstream),
    )?;
    let repeated_upstream_prefix = execute_boundary_continuity_contracts(
        solver,
        &session,
        &surfaces,
        slice::from_ref(&upstream),
    )?;
    let downstream_control = execute_boundary_continuity_contracts(
        solver,
        &session,
        &surfaces,
        slice::from_ref(&downstream),
    )?;
    ensure!(
        upstream_prefix.solves().len() == 1 && downstream_control.solves().len() == 1,
        "each bounded contract must succeed independently"
    );
    ensure!(
        upstream_prefix
            .surfaces()
            .get(&middle_id)
            .ok_or_else(|| anyhow!("the staged prefix omitted the middle surface"))?
            != surfaces
                .get(&middle_id)
                .ok_or_else(|| anyhow!("the input omitted the middle surface"))?,
        "the upstream prefix did not stage changed geometry"
    );
    let surfaces_equal_across_reruns =
        upstream_prefix.surfaces() == repeated_upstream_prefix.surfaces();
    let transitions_equal_across_reruns = upstream_prefix
        .solves()
        .iter()
        .map(|solve| solve.transition())
        .eq(repeated_upstream_prefix
            .solves()
            .iter()
            .map(|solve| solve.transition()));
    let reports_equal_across_reruns = upstream_prefix
        .solves()
        .iter()
        .map(|solve| solve.report())
        .eq(repeated_upstream_prefix
            .solves()
            .iter()
            .map(|solve| solve.report()));
    ensure!(
        surfaces_equal_across_reruns
            && transitions_equal_across_reruns
            && reports_equal_across_reruns,
        "repeated bounded prefix state differed"
    );
    let staged = upstream_prefix
        .solves()
        .first()
        .ok_or_else(|| anyhow!("the bounded prefix omitted its staged solve"))?;
    let staged_prefix = StagedPrefixEvidence {
        contract_id: staged.contract_id().as_str().to_owned(),
        solve_count: upstream_prefix.solves().len(),
        geometry_changed: true,
        surfaces_equal_across_reruns,
        transitions_equal_across_reruns,
        reports_equal_across_reruns,
        transition_order: staged.transition().order(),
        mapped_coordinates: staged.transition().mapped_coordinates(0.25, 0.0),
        termination: staged.report().termination(),
        iterations: staged.report().iterations(),
        accepted_steps: staged.report().accepted_steps(),
        rejected_steps: staged.report().rejected_steps(),
    };
    let first = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "the dependency-ordered one-iteration batch must be bounded",
    )?;
    let second = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "the repeated dependency-ordered one-iteration batch must be bounded",
    )?;
    ensure!(
        first == second,
        "repeated execution returned a different error"
    );
    ensure!(
        session == original_session,
        "replay changed the tracking session"
    );
    ensure!(
        surfaces == original_surfaces,
        "replay changed the geometry map"
    );
    ensure!(
        contracts == original_contracts,
        "replay changed the contract inputs"
    );
    let error = match first {
        ContinuityReplayExecutionError::Solve {
            contract_id,
            source: ContinuitySolveError::DidNotConverge(report),
        } => {
            ensure!(
                contract_id == *downstream.id(),
                "the bounded failure did not occur after the upstream solve"
            );
            ensure!(
                report.termination() == ContinuityTermination::MaximumIterations,
                "bounded solve terminated for an unexpected reason"
            );
            ensure!(
                report.iterations() == 1,
                "bounded solve exceeded one iteration"
            );
            ErrorEvidence::DidNotConverge {
                contract_id: contract_id.as_str().to_owned(),
                termination: report.termination(),
                iterations: report.iterations(),
                accepted_steps: report.accepted_steps(),
                rejected_steps: report.rejected_steps(),
            }
        }
        _ => return Err(anyhow!("expected bounded nonconvergence, got {first}")),
    };
    Ok(CaseEvidence {
        story: "MT-304",
        error,
        repeated_error_equal: true,
        geometry_unchanged: true,
        tracking_unchanged: true,
        contracts_unchanged: true,
        execution: ExecutionEvidence::BoundedSolveFailure {
            returned_solution: false,
            individual_contracts_succeeded: vec![true, true],
            input_contract_order: contract_ids(&contracts),
            expected_dependency_order: vec![
                upstream.id().as_str().to_owned(),
                downstream.id().as_str().to_owned(),
            ],
            prefix_repeated_equal: true,
            staged_prefix,
        },
    })
}

fn late_dependency_failure(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let references = [
        semantic("replay-validation", "late-master")?,
        semantic("replay-validation", "late-middle")?,
        semantic("replay-validation", "late-dependent")?,
    ];
    let upstream = contract(
        "z-master-to-middle",
        references[0].clone(),
        SurfaceBoundary::UEnd,
        references[1].clone(),
        SurfaceBoundary::UStart,
    )?;
    let downstream = contract(
        "a-middle-to-dependent",
        references[1].clone(),
        SurfaceBoundary::UEnd,
        references[2].clone(),
        SurfaceBoundary::UStart,
    )?;
    let contracts = [downstream, upstream.clone()];
    let original_contracts = contracts.clone();
    let (session, ids) = session_with_bindings(&references)?;
    let [master_id, middle_id, dependent_id]: [TrackingId; 3] = ids
        .try_into()
        .map_err(|_| anyhow!("the late-failure batch requires three tracking IDs"))?;
    let mut dependent = plane(2.0, 1.75);
    dependent.control_point_mut(0, 0).w = 0.0;
    let surfaces = BTreeMap::from([
        (master_id, plane(0.0, 1.75)),
        (middle_id.clone(), plane(1.0, 1.5)),
        (dependent_id, dependent),
    ]);
    let original_session = session.clone();
    let original_surfaces = surfaces.clone();
    let prefix = execute_boundary_continuity_contracts(
        solver,
        &session,
        &surfaces,
        slice::from_ref(&upstream),
    )?;
    let repeated_prefix = execute_boundary_continuity_contracts(
        solver,
        &session,
        &surfaces,
        slice::from_ref(&upstream),
    )?;
    let surfaces_equal_across_reruns = prefix.surfaces() == repeated_prefix.surfaces();
    let transitions_equal_across_reruns = prefix
        .solves()
        .iter()
        .map(|solve| solve.transition())
        .eq(repeated_prefix
            .solves()
            .iter()
            .map(|solve| solve.transition()));
    let reports_equal_across_reruns = prefix
        .solves()
        .iter()
        .map(|solve| solve.report())
        .eq(repeated_prefix.solves().iter().map(|solve| solve.report()));
    ensure!(
        surfaces_equal_across_reruns
            && transitions_equal_across_reruns
            && reports_equal_across_reruns,
        "repeated staged prefix state differed"
    );
    ensure!(
        prefix.solves().len() == 1,
        "the prefix did not stage one solve"
    );
    ensure!(
        prefix
            .surfaces()
            .get(&middle_id)
            .ok_or_else(|| anyhow!("the staged prefix omitted the middle surface"))?
            != surfaces
                .get(&middle_id)
                .ok_or_else(|| anyhow!("the input omitted the middle surface"))?,
        "the prefix did not stage changed geometry"
    );
    let staged = prefix
        .solves()
        .first()
        .ok_or_else(|| anyhow!("the late-failure prefix omitted its staged solve"))?;
    let staged_prefix = StagedPrefixEvidence {
        contract_id: staged.contract_id().as_str().to_owned(),
        solve_count: prefix.solves().len(),
        geometry_changed: true,
        surfaces_equal_across_reruns,
        transitions_equal_across_reruns,
        reports_equal_across_reruns,
        transition_order: staged.transition().order(),
        mapped_coordinates: staged.transition().mapped_coordinates(0.25, 0.0),
        termination: staged.report().termination(),
        iterations: staged.report().iterations(),
        accepted_steps: staged.report().accepted_steps(),
        rejected_steps: staged.report().rejected_steps(),
    };
    let first = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "the downstream invalid weight must reject the complete batch",
    )?;
    let second = require_error(
        execute_boundary_continuity_contracts(solver, &session, &surfaces, &contracts),
        "the repeated downstream invalid weight must reject the complete batch",
    )?;
    ensure!(
        first == second,
        "repeated execution returned a different error"
    );
    ensure!(
        session == original_session,
        "replay changed the tracking session"
    );
    ensure!(
        surfaces == original_surfaces,
        "replay changed the geometry map"
    );
    ensure!(
        contracts == original_contracts,
        "replay changed the contract inputs"
    );
    let error = match first {
        ContinuityReplayExecutionError::Solve {
            contract_id,
            source:
                ContinuitySolveError::NonPositiveWeight {
                    endpoint,
                    row,
                    column,
                    weight,
                },
        } => {
            ensure!(
                contract_id.as_str() == "a-middle-to-dependent",
                "the wrong dependency contract failed"
            );
            ensure!(
                endpoint == BoundaryEndpoint::Second,
                "the wrong endpoint failed"
            );
            ensure!(
                row == 0 && column == 0 && weight == 0.0,
                "the tripwire moved"
            );
            ErrorEvidence::NonPositiveWeight {
                contract_id: contract_id.as_str().to_owned(),
                endpoint,
                row,
                column,
                weight,
            }
        }
        _ => return Err(anyhow!("expected late solve failure, got {first}")),
    };
    Ok(CaseEvidence {
        story: "MT-305",
        error,
        repeated_error_equal: true,
        geometry_unchanged: true,
        tracking_unchanged: true,
        contracts_unchanged: true,
        execution: ExecutionEvidence::LateSolveFailure {
            returned_solution: false,
            prefix_repeated_equal: true,
            input_contract_order: contract_ids(&contracts),
            expected_dependency_order: vec![
                upstream.id().as_str().to_owned(),
                contracts
                    .first()
                    .ok_or_else(|| {
                        anyhow!("the late-failure batch omitted its downstream contract")
                    })?
                    .id()
                    .as_str()
                    .to_owned(),
            ],
            staged_prefix,
        },
    })
}

fn same_surface_contract(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let reference = semantic("replay-validation", "same-surface")?;
    let contract = contract(
        "same-surface",
        reference.clone(),
        SurfaceBoundary::UEnd,
        reference.clone(),
        SurfaceBoundary::UStart,
    )?;
    let (session, ids) = session_with_bindings(&[reference])?;
    let surfaces = BTreeMap::from([(ids[0].clone(), invalid_plane(0.0))]);
    validate_case(
        "MT-301",
        solver,
        &session,
        &surfaces,
        &[contract],
        |error| match error {
            ContinuityReplayExecutionError::SameSurfaceContract {
                contract_id,
                tracking_id,
            } => Ok(ErrorEvidence::SameSurfaceContract {
                contract_id: contract_id.as_str().to_owned(),
                tracking_id: tracking_id.to_string(),
            }),
            _ => Err(anyhow!("expected same-surface rejection, got {error}")),
        },
    )
}

fn coupled_optimized_surface(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let references = [
        semantic("replay-validation", "coupled-master-a")?,
        semantic("replay-validation", "coupled-dependent")?,
        semantic("replay-validation", "coupled-master-b")?,
    ];
    let contracts = [
        contract(
            "z-coupled",
            references[2].clone(),
            SurfaceBoundary::UEnd,
            references[1].clone(),
            SurfaceBoundary::UStart,
        )?,
        contract(
            "a-coupled",
            references[0].clone(),
            SurfaceBoundary::UEnd,
            references[1].clone(),
            SurfaceBoundary::UStart,
        )?,
    ];
    let (session, ids) = session_with_bindings(&references)?;
    let surfaces = invalid_surfaces(&ids);
    validate_case(
        "MT-302",
        solver,
        &session,
        &surfaces,
        &contracts,
        |error| match error {
            ContinuityReplayExecutionError::CoupledOptimizedSurface {
                tracking_id,
                first_contract,
                second_contract,
            } => {
                ensure!(first_contract.as_str() == "a-coupled");
                ensure!(second_contract.as_str() == "z-coupled");
                Ok(ErrorEvidence::CoupledOptimizedSurface {
                    tracking_id: tracking_id.to_string(),
                    first_contract: first_contract.as_str().to_owned(),
                    second_contract: second_contract.as_str().to_owned(),
                })
            }
            _ => Err(anyhow!("expected coupled-writer rejection, got {error}")),
        },
    )
}

fn dependency_cycle_two(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let references = [
        semantic("replay-validation", "cycle-two-a")?,
        semantic("replay-validation", "cycle-two-b")?,
    ];
    let contracts = [
        contract(
            "z-cycle-two",
            references[0].clone(),
            SurfaceBoundary::UEnd,
            references[1].clone(),
            SurfaceBoundary::UStart,
        )?,
        contract(
            "a-cycle-two",
            references[1].clone(),
            SurfaceBoundary::UEnd,
            references[0].clone(),
            SurfaceBoundary::UStart,
        )?,
    ];
    let (session, ids) = session_with_bindings(&references)?;
    let surfaces = invalid_surfaces(&ids);
    validate_cycle_case(
        solver,
        &session,
        &surfaces,
        &contracts,
        &["a-cycle-two", "z-cycle-two"],
    )
}

fn dependency_cycle_three(solver: &BoundaryContinuitySolver) -> Result<CaseEvidence> {
    let references = [
        semantic("replay-validation", "cycle-three-a")?,
        semantic("replay-validation", "cycle-three-b")?,
        semantic("replay-validation", "cycle-three-c")?,
    ];
    let contracts = [
        contract(
            "m-cycle-three",
            references[1].clone(),
            SurfaceBoundary::UEnd,
            references[2].clone(),
            SurfaceBoundary::UStart,
        )?,
        contract(
            "z-cycle-three",
            references[2].clone(),
            SurfaceBoundary::UEnd,
            references[0].clone(),
            SurfaceBoundary::UStart,
        )?,
        contract(
            "a-cycle-three",
            references[0].clone(),
            SurfaceBoundary::UEnd,
            references[1].clone(),
            SurfaceBoundary::UStart,
        )?,
    ];
    let (session, ids) = session_with_bindings(&references)?;
    let surfaces = invalid_surfaces(&ids);
    validate_cycle_case(
        solver,
        &session,
        &surfaces,
        &contracts,
        &["a-cycle-three", "m-cycle-three", "z-cycle-three"],
    )
}

fn validate_cycle_case(
    solver: &BoundaryContinuitySolver,
    session: &TrackingSession,
    surfaces: &BTreeMap<TrackingId, NurbsSurface<Vector4>>,
    contracts: &[ContinuityContract],
    expected_contracts: &[&str],
) -> Result<CaseEvidence> {
    validate_case(
        "MT-303",
        solver,
        session,
        surfaces,
        contracts,
        |error| match error {
            ContinuityReplayExecutionError::DependencyCycle { contracts } => {
                let contracts = contracts
                    .iter()
                    .map(|contract| contract.as_str().to_owned())
                    .collect::<Vec<_>>();
                ensure!(
                    contracts
                        .iter()
                        .map(String::as_str)
                        .eq(expected_contracts.iter().copied()),
                    "dependency-cycle contract order was not canonical",
                );
                Ok(ErrorEvidence::DependencyCycle { contracts })
            }
            _ => Err(anyhow!("expected dependency-cycle rejection, got {error}")),
        },
    )
}

fn validate_case(
    story: &'static str,
    solver: &BoundaryContinuitySolver,
    session: &TrackingSession,
    surfaces: &BTreeMap<TrackingId, NurbsSurface<Vector4>>,
    contracts: &[ContinuityContract],
    classify: impl Fn(&ContinuityReplayExecutionError) -> Result<ErrorEvidence>,
) -> Result<CaseEvidence> {
    let original_session = session.clone();
    let original_surfaces = surfaces.clone();
    let original_contracts = contracts.to_vec();
    let first = require_error(
        execute_boundary_continuity_contracts(solver, session, surfaces, contracts),
        "the validation batch must be rejected",
    )?;
    let second = require_error(
        execute_boundary_continuity_contracts(solver, session, surfaces, contracts),
        "the repeated validation batch must be rejected",
    )?;
    ensure!(
        first == second,
        "repeated execution returned a different error"
    );
    ensure!(
        session == &original_session,
        "replay changed the tracking session"
    );
    ensure!(
        surfaces == &original_surfaces,
        "replay changed the geometry map"
    );
    ensure!(
        contracts == original_contracts,
        "replay changed the contract inputs"
    );
    let error = classify(&first)?;
    Ok(CaseEvidence {
        story,
        error,
        repeated_error_equal: true,
        geometry_unchanged: true,
        tracking_unchanged: true,
        contracts_unchanged: true,
        execution: ExecutionEvidence::RejectedBeforeSolve,
    })
}

fn require_error<T>(
    result: StdResult<T, ContinuityReplayExecutionError>,
    message: &str,
) -> Result<ContinuityReplayExecutionError> {
    result.map_or_else(Ok, |_| Err(anyhow!(message.to_owned())))
}

fn contract_ids(contracts: &[ContinuityContract]) -> Vec<String> {
    contracts
        .iter()
        .map(|contract| contract.id().as_str().to_owned())
        .collect()
}

fn semantic(feature: &str, label: &str) -> Result<SemanticTopologyRef> {
    Ok(SemanticTopologyRef::new(
        FeatureId::new(feature)?,
        TopologyKind::Face,
        SemanticLabel::new(label)?,
    ))
}

fn contract(
    id: &str,
    first: SemanticTopologyRef,
    first_boundary: SurfaceBoundary,
    second: SemanticTopologyRef,
    second_boundary: SurfaceBoundary,
) -> Result<ContinuityContract> {
    contract_with(
        id,
        first,
        first_boundary,
        second,
        second_boundary,
        BoundaryAlignment::Aligned,
        ContinuityOrder::G0,
    )
}

fn contract_with(
    id: &str,
    first: SemanticTopologyRef,
    first_boundary: SurfaceBoundary,
    second: SemanticTopologyRef,
    second_boundary: SurfaceBoundary,
    alignment: BoundaryAlignment,
    order: ContinuityOrder,
) -> Result<ContinuityContract> {
    Ok(ContinuityContract::new(
        ContractId::new(id)?,
        SurfaceBoundaryRef::new(first, first_boundary)?,
        SurfaceBoundaryRef::new(second, second_boundary)?,
        alignment,
        order,
    )?)
}

fn session_with_bindings(
    references: &[SemanticTopologyRef],
) -> Result<(TrackingSession, Vec<TrackingId>)> {
    let mut session = TrackingSession::new(TrackingSessionId::new("replay-validation")?);
    let ids = references
        .iter()
        .map(|reference| {
            let tracking_id = session.allocate()?;
            session.bind(reference.clone(), tracking_id.clone())?;
            Ok(tracking_id)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((session, ids))
}

fn invalid_surfaces(ids: &[TrackingId]) -> BTreeMap<TrackingId, NurbsSurface<Vector4>> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), invalid_plane(index as f64)))
        .collect()
}

fn valid_surfaces(ids: &[TrackingId]) -> BTreeMap<TrackingId, NurbsSurface<Vector4>> {
    ids.iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), plane(index as f64, 1.0)))
        .collect()
}

fn invalid_plane(x_start: f64) -> NurbsSurface<Vector4> {
    let mut surface = plane(x_start, 1.0);
    surface.control_point_mut(0, 0).w = 0.0;
    surface
}

fn plane(x_start: f64, height: f64) -> NurbsSurface<Vector4> {
    let knots = KnotVector::bezier_knot(1);
    NurbsSurface::new(BsplineSurface::new(
        (knots.clone(), knots),
        vec![
            vec![
                Vector4::new(x_start, 0.0, 0.0, 1.0),
                Vector4::new(x_start, height, 0.0, 1.0),
            ],
            vec![
                Vector4::new(x_start + 1.0, 0.0, 0.0, 1.0),
                Vector4::new(x_start + 1.0, height, 0.0, 1.0),
            ],
        ],
    ))
}

fn exact_quintic_surfaces() -> Result<(NurbsSurface<Vector4>, NurbsSurface<Vector4>)> {
    const DEGREE: usize = 5;
    let cross_knots = KnotVector::try_from(vec![
        -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -0.5, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ])?;
    let seam_knots = KnotVector::try_from(vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    ])?;
    let cross_count = cross_knots.len() - DEGREE - 1;
    let seam_count = seam_knots.len() - DEGREE - 1;
    let seam_values = (0..seam_count)
        .map(|index| seam_knots[index + 1..=index + DEGREE].iter().sum::<f64>() / DEGREE as f64)
        .collect::<Vec<_>>();
    let seam_start = *seam_values
        .first()
        .ok_or_else(|| anyhow!("the quintic fixture requires seam controls"))?;
    let control_points = (0..cross_count)
        .map(|cross| {
            let normalized = cross as f64 / (cross_count - 1) as f64;
            let x = -1.0 + 2.0 * normalized;
            let cross_z = 0.18 * x * x + 0.07 * x * x * x - 0.1;
            seam_values
                .iter()
                .map(|&y| Vector4::new(x, y, cross_z + seam_z(y), 1.0))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut first = NurbsSurface::new(BsplineSurface::new(
        (cross_knots, seam_knots.clone()),
        control_points,
    ));
    let cut = first.cut_u(0.0);
    let second_control_points = cut
        .control_points()
        .iter()
        .map(|row| {
            let reference = row
                .first()
                .copied()
                .ok_or_else(|| anyhow!("the cut surface contains an empty control row"))?;
            let cross_z = reference.z - seam_z(seam_start);
            Ok(seam_values
                .iter()
                .map(|&y| Vector4::new(reference.x, y, cross_z + seam_z(y), 1.0))
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>>>()?;
    let second_cross_knots =
        KnotVector::try_from(cut.knot_vector_u().iter().copied().collect::<Vec<_>>())?;
    let second = NurbsSurface::new(BsplineSurface::new(
        (second_cross_knots, seam_knots),
        second_control_points,
    ));
    Ok((first, second))
}

fn seam_z(parameter: f64) -> f64 {
    0.08 * parameter * (1.0 - parameter) + 0.03 * parameter * parameter * parameter
}

fn options() -> Result<Options> {
    let mut base_revision = None;
    let mut harness_sha256 = None;
    let mut toolchain = None;
    let mut receipt = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--base-revision" => base_revision = arguments.next(),
            "--harness-sha256" => harness_sha256 = arguments.next(),
            "--toolchain" => toolchain = arguments.next(),
            "--receipt" => receipt = arguments.next().map(PathBuf::from),
            _ => bail!("unknown argument `{argument}`"),
        }
    }
    Ok(Options {
        base_revision: base_revision.ok_or_else(|| anyhow!("--base-revision requires a value"))?,
        harness_sha256: harness_sha256
            .ok_or_else(|| anyhow!("--harness-sha256 requires a value"))?,
        toolchain: toolchain.ok_or_else(|| anyhow!("--toolchain requires a value"))?,
        receipt: receipt.ok_or_else(|| anyhow!("--receipt requires a path"))?,
    })
}
