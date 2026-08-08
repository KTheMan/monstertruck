use anyhow::{Context, Result, anyhow, bail, ensure};
use monstertruck_geometry::nurbs::continuity::ContinuityOrder;
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuitySolver, BoundaryEndpoint, ContinuitySolveError, ContinuitySolveReport,
    ContinuityTermination, ContinuityWork, take_continuity_work,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod corpus;
mod dense;
mod digest;
mod fixture;

use corpus::{CaseSpec, Corpus, ErrorKind, Expectation};
use dense::DenseMetrics;

#[derive(Serialize)]
struct ObservedRun {
    schema_version: u32,
    digest_version: &'static str,
    fixture_version: String,
    cases: BTreeMap<String, ObservedCase>,
}

#[derive(Serialize)]
struct ObservedCase {
    #[serde(flatten)]
    evidence: CaseEvidence,
    run_elapsed_milliseconds: [u128; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct CaseEvidence {
    outcome: String,
    digest: String,
    dense: Option<DenseMetrics>,
    work: Option<WorkCounters>,
    error: Option<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct WorkCounters {
    iterations: u64,
    jacobian_elements: u64,
    qr_elements: u64,
    truncated: bool,
}

impl From<ContinuityWork> for WorkCounters {
    fn from(work: ContinuityWork) -> Self {
        Self {
            iterations: work.iterations,
            jacobian_elements: work.jacobian_elements,
            qr_elements: work.qr_elements,
            truncated: work.truncated,
        }
    }
}

enum Mode {
    Emit(PathBuf),
    List,
}

struct Options {
    corpus: PathBuf,
    case: Option<String>,
    mode: Mode,
}

fn main() -> Result<()> {
    let options = options()?;
    let corpus: Corpus = read_json(&options.corpus)?;
    ensure!(corpus.schema_version == 1, "unsupported corpus schema");
    let mut ids = HashSet::new();
    ensure!(
        corpus.cases.iter().all(|case| ids.insert(case.id.clone())),
        "corpus case IDs must be unique",
    );
    if matches!(options.mode, Mode::List) {
        corpus.cases.iter().for_each(|case| println!("{}", case.id));
        return Ok(());
    }
    let observed = run_corpus(&corpus, options.case.as_deref())?;
    match options.mode {
        Mode::Emit(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, serde_json::to_vec_pretty(&observed)?)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        Mode::List => unreachable!(),
    }
}

fn run_corpus(corpus: &Corpus, selected_case: Option<&str>) -> Result<ObservedRun> {
    let cases = corpus
        .cases
        .iter()
        .filter(|case| selected_case.is_none_or(|selected| case.id == selected))
        .map(|case| {
            let started = Instant::now();
            let first = run_case(case, corpus.dense_defaults, &corpus.fixture_version)
                .with_context(|| format!("case `{}` failed", case.id))?;
            let first_elapsed = started.elapsed().as_millis();
            let started = Instant::now();
            let second = run_case(case, corpus.dense_defaults, &corpus.fixture_version)
                .with_context(|| format!("case `{}` repeat failed", case.id))?;
            let second_elapsed = started.elapsed().as_millis();
            ensure!(
                first == second,
                "case `{}` was not deterministic across immediate reruns",
                case.id,
            );
            println!(
                "{} {} {} {}ms/{}ms",
                case.id, first.outcome, first.digest, first_elapsed, second_elapsed,
            );
            Ok((
                case.id.clone(),
                ObservedCase {
                    evidence: first,
                    run_elapsed_milliseconds: [first_elapsed, second_elapsed],
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    ensure!(
        selected_case.is_none() || cases.len() == 1,
        "selected corpus case was not found",
    );
    Ok(ObservedRun {
        schema_version: 1,
        digest_version: digest::DIGEST_VERSION,
        fixture_version: corpus.fixture_version.clone(),
        cases,
    })
}

fn run_case(
    case: &CaseSpec,
    dense_spec: corpus::DenseSpec,
    fixture_version: &str,
) -> Result<CaseEvidence> {
    let fixture = fixture::build(case)?;
    let config = case.solver.build();
    let request = case.request.build()?;
    take_continuity_work();
    let outcome = BoundaryContinuitySolver::new(config.clone())
        .and_then(|solver| solver.solve(&fixture.first, &fixture.second, request));
    let work = take_continuity_work();
    match (&case.expectation, outcome) {
        (
            Expectation::Converged {
                maximum_dense_residual_by_order,
                maximum_normal_angle,
            },
            Ok(solution),
        ) => {
            ensure!(
                solution.report().termination() == ContinuityTermination::Converged,
                "the solver returned a non-converged solution",
            );
            let dense = dense::certify(&solution, request, dense_spec, case.geometry.scale)?;
            ensure!(
                dense.maximum_normalized_residual_by_order.len()
                    == maximum_dense_residual_by_order.len(),
                "dense tolerance count does not match the requested order",
            );
            dense
                .maximum_normalized_residual_by_order
                .iter()
                .zip(maximum_dense_residual_by_order)
                .enumerate()
                .try_for_each(|(order, (&actual, &limit))| -> Result<()> {
                    ensure!(
                        actual <= limit,
                        "dense order {order} residual {actual} exceeds {limit}",
                    );
                    Ok(())
                })?;
            ensure!(
                dense.maximum_normal_angle <= *maximum_normal_angle,
                "dense normal angle {} exceeds {}",
                dense.maximum_normal_angle,
                maximum_normal_angle,
            );
            let digest = digest::solved(fixture_version, case, dense_spec, &solution, &dense)?;
            Ok(CaseEvidence {
                outcome: "converged".to_owned(),
                digest,
                dense: Some(dense),
                work: Some(work.into()),
                error: None,
            })
        }
        (Expectation::Converged { .. }, Err(error)) => {
            Err(anyhow!("expected convergence, got {error}"))
        }
        (Expectation::Error { error: expected }, Err(error)) => {
            let actual = classify_error(&error);
            ensure!(
                actual == *expected,
                "expected error {expected:?}, got {actual:?}: {error}",
            );
            let outcome = actual.as_str().to_owned();
            let error = error_evidence(&error);
            Ok(CaseEvidence {
                outcome: outcome.clone(),
                digest: digest::error(fixture_version, case, dense_spec, &error)?,
                dense: None,
                work: Some(work.into()),
                error: Some(error),
            })
        }
        (Expectation::Error { error }, Ok(_)) => {
            bail!("expected error {error:?}, but the solver converged")
        }
    }
}

fn error_evidence(error: &ContinuitySolveError) -> Value {
    match error {
        ContinuitySolveError::InvalidConfig(message) => json!({
            "kind": "invalid_config",
            "message": message,
        }),
        ContinuitySolveError::Truncated(truncated) => json!({
            "kind": "truncated",
            "resource": format!("{:?}", truncated.resource),
            "spent": truncated.spent,
            "requested": truncated.requested,
            "budget": truncated.budget,
        }),
        ContinuitySolveError::ExperimentalG4Disabled => json!({
            "kind": "experimental_g4_disabled",
        }),
        ContinuitySolveError::UnsupportedCapability {
            endpoint,
            capability,
            reason,
        } => json!({
            "kind": "unsupported_capability",
            "endpoint": endpoint_name(*endpoint),
            "reason": reason.to_string(),
            "capability": {
                "side": format!("{:?}", capability.side()),
                "requested": capability.requested().as_usize(),
                "maximum_supported_order": capability
                    .maximum_supported_order()
                    .map(ContinuityOrder::as_usize),
            },
        }),
        ContinuitySolveError::InvalidBoundary(endpoint) => json!({
            "kind": "invalid_boundary",
            "endpoint": endpoint_name(*endpoint),
        }),
        ContinuitySolveError::NonPositiveWeight {
            endpoint,
            row,
            column,
            weight,
        } => json!({
            "kind": "non_positive_weight",
            "endpoint": endpoint_name(*endpoint),
            "row": row,
            "column": column,
            "weight": weight,
        }),
        ContinuitySolveError::NonFiniteControlPoint {
            endpoint,
            row,
            column,
        } => json!({
            "kind": "non_finite_control_point",
            "endpoint": endpoint_name(*endpoint),
            "row": row,
            "column": column,
        }),
        ContinuitySolveError::DegenerateBoundary { endpoint, sample } => json!({
            "kind": "degenerate_boundary",
            "endpoint": endpoint_name(*endpoint),
            "sample": sample,
        }),
        ContinuitySolveError::NonFiniteResidual => json!({
            "kind": "non_finite_residual",
        }),
        ContinuitySolveError::NonFiniteJacobian => json!({
            "kind": "non_finite_jacobian",
        }),
        ContinuitySolveError::NoDescentDirection => json!({
            "kind": "no_descent_direction",
        }),
        ContinuitySolveError::DidNotConverge(report) => json!({
            "kind": "did_not_converge",
            "report": report_evidence(report),
        }),
    }
}

const fn endpoint_name(endpoint: BoundaryEndpoint) -> &'static str {
    match endpoint {
        BoundaryEndpoint::First => "First",
        BoundaryEndpoint::Second => "Second",
    }
}

fn report_evidence(report: &ContinuitySolveReport) -> Value {
    json!({
        "termination": format!("{:?}", report.termination()),
        "iterations": report.iterations(),
        "accepted_steps": report.accepted_steps(),
        "rejected_steps": report.rejected_steps(),
        "initial_objective": report.initial_objective(),
        "final_objective": report.final_objective(),
        "residuals": report.residuals().iter().map(|residual| json!({
            "order": residual.order().as_usize(),
            "rms": residual.rms(),
            "maximum": residual.maximum(),
            "worst_sample": residual.worst_sample(),
            "validation_sample": residual.is_validation_sample(),
            "cross_derivative": residual.cross_derivative(),
            "seam_derivative": residual.seam_derivative(),
        })).collect::<Vec<_>>(),
        "gradient_infinity_norm": report.gradient_infinity_norm(),
        "step_norm": report.step_norm(),
        "damping": report.damping(),
        "numerical_rank": report.numerical_rank(),
        "variable_count": report.variable_count(),
        "residual_count": report.residual_count(),
    })
}

fn classify_error(error: &ContinuitySolveError) -> ErrorKind {
    match error {
        ContinuitySolveError::InvalidConfig(_) => ErrorKind::InvalidConfig,
        ContinuitySolveError::ExperimentalG4Disabled => ErrorKind::ExperimentalG4Disabled,
        ContinuitySolveError::UnsupportedCapability { .. } => ErrorKind::UnsupportedCapability,
        ContinuitySolveError::InvalidBoundary(_) => ErrorKind::InvalidBoundary,
        ContinuitySolveError::NonPositiveWeight { .. } => ErrorKind::NonPositiveWeight,
        ContinuitySolveError::NonFiniteControlPoint { .. } => ErrorKind::NonFiniteControlPoint,
        ContinuitySolveError::DegenerateBoundary { .. } => ErrorKind::DegenerateBoundary,
        ContinuitySolveError::NonFiniteResidual => ErrorKind::NonFiniteResidual,
        ContinuitySolveError::NonFiniteJacobian => ErrorKind::NonFiniteJacobian,
        ContinuitySolveError::NoDescentDirection => ErrorKind::NoDescentDirection,
        ContinuitySolveError::DidNotConverge(_) => ErrorKind::DidNotConverge,
        ContinuitySolveError::Truncated(_) => ErrorKind::Truncated,
    }
}

fn options() -> Result<Options> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resource = manifest.join("resources/continuity-validation/v1");
    let mut corpus = resource.join("corpus.json");
    let mut case = None;
    let mut mode = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--corpus" => {
                corpus = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| anyhow!("--corpus requires a path"))?,
                );
            }
            "--emit" => {
                mode = Some(Mode::Emit(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| anyhow!("--emit requires a path"))?,
                )));
            }
            "--case" => {
                case = Some(
                    arguments
                        .next()
                        .ok_or_else(|| anyhow!("--case requires an ID"))?,
                );
            }
            "--list" => mode = Some(Mode::List),
            _ => bail!("unknown argument `{argument}`"),
        }
    }
    Ok(Options {
        corpus,
        case,
        mode: mode.ok_or_else(|| anyhow!("use --emit <path> or --list"))?,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}
