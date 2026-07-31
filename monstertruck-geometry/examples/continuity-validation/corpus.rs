use monstertruck_geometry::nurbs::continuity::{
    ContinuityMaturity, ContinuityOrder, SurfaceBoundary,
};
use monstertruck_geometry::nurbs::continuity_solver::{
    BoundaryContinuityRequest, ContinuitySolverConfig,
};
use monstertruck_geometry::nurbs::contract::BoundaryAlignment;
use serde::{Deserialize, Serialize};

/// Versioned continuity-validation corpus.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Corpus {
    pub schema_version: u32,
    pub fixture_version: String,
    pub dense_defaults: DenseSpec,
    pub cases: Vec<CaseSpec>,
}

/// One deterministic validation case.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CaseSpec {
    pub id: String,
    pub geometry: GeometrySpec,
    pub request: RequestSpec,
    #[serde(default)]
    pub solver: SolverSpec,
    pub expectation: Expectation,
}

/// Procedural tensor-product fixture controls.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct GeometrySpec {
    pub weight_model: WeightModel,
    pub second_seam_parameterization: SeamParameterization,
    pub scale: f64,
    #[serde(default)]
    pub boundary_offset: f64,
    #[serde(default)]
    pub planar: bool,
    #[serde(default = "default_domain_scale")]
    pub second_cross_domain_scale: f64,
    #[serde(default = "default_cross_degree")]
    pub cross_degree: usize,
    #[serde(default)]
    pub mutation: FixtureMutation,
}

const fn default_cross_degree() -> usize { 5 }

const fn default_domain_scale() -> f64 { 1.0 }

/// Polynomial or rational homogeneous weights.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightModel {
    Polynomial,
    Rational,
}

/// Parameterization applied to the dependent surface's seam.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeamParameterization {
    Equal,
    Unequal,
    Reversed,
    UnequalReversed,
}

impl SeamParameterization {
    pub const fn alignment(self) -> BoundaryAlignment {
        match self {
            Self::Equal | Self::Unequal => BoundaryAlignment::Aligned,
            Self::Reversed | Self::UnequalReversed => BoundaryAlignment::Reversed,
        }
    }

    pub const fn is_unequal(self) -> bool { matches!(self, Self::Unequal | Self::UnequalReversed) }

    pub const fn is_reversed(self) -> bool {
        matches!(self, Self::Reversed | Self::UnequalReversed)
    }
}

/// Deliberate mutation used by a structured negative case.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMutation {
    #[default]
    None,
    ZeroSecondWeight,
    DegenerateFirstBoundary,
}

/// Serializable request form with stable enum spellings.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct RequestSpec {
    pub first_boundary: SurfaceBoundary,
    pub second_boundary: SurfaceBoundary,
    pub alignment: BoundaryAlignment,
    pub order: ContinuityOrder,
}

impl RequestSpec {
    pub const fn build(self) -> BoundaryContinuityRequest {
        BoundaryContinuityRequest::new(
            self.first_boundary,
            self.second_boundary,
            self.alignment,
            self.order,
        )
    }
}

/// Public solver overrides used by the corpus.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SolverSpec {
    pub max_iterations: usize,
    pub samples_per_span: usize,
    pub transition_degree: usize,
    pub anchor_weight: f64,
    pub fairness_weight: f64,
    pub transition_weight: f64,
    pub experimental_g4: bool,
}

impl Default for SolverSpec {
    fn default() -> Self {
        Self {
            max_iterations: 80,
            samples_per_span: 3,
            transition_degree: 3,
            anchor_weight: 0.0,
            fairness_weight: 0.0,
            transition_weight: 1.0e-6,
            experimental_g4: false,
        }
    }
}

impl SolverSpec {
    pub fn build(self) -> ContinuitySolverConfig {
        ContinuitySolverConfig::default()
            .with_max_iterations(self.max_iterations)
            .with_samples_per_span(self.samples_per_span)
            .with_transition_degree(self.transition_degree)
            .with_anchor_weight(self.anchor_weight)
            .with_fairness_weight(self.fairness_weight)
            .with_transition_weight(self.transition_weight)
            .with_experimental_g4(self.experimental_g4)
    }
}

/// Independent dense finite-difference controls.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct DenseSpec {
    pub seam_samples: usize,
    pub stencil_radius: usize,
    pub normalized_step: f64,
}

/// Expected solver outcome.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expectation {
    Converged {
        maturity: ContinuityMaturity,
        maximum_dense_residual_by_order: Vec<f64>,
        maximum_normal_angle: f64,
    },
    Error {
        error: ErrorKind,
    },
}

/// Stable classification of expected solver errors.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidConfig,
    ExperimentalG4Disabled,
    UnsupportedCapability,
    InvalidBoundary,
    NonPositiveWeight,
    NonFiniteControlPoint,
    DegenerateBoundary,
    NonFiniteResidual,
    NonFiniteJacobian,
    NoDescentDirection,
    DidNotConverge,
    ResourceLimitExceeded,
}

impl ErrorKind {
    /// Returns the stable evidence-schema name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "invalid_config",
            Self::ExperimentalG4Disabled => "experimental_g4_disabled",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::InvalidBoundary => "invalid_boundary",
            Self::NonPositiveWeight => "non_positive_weight",
            Self::NonFiniteControlPoint => "non_finite_control_point",
            Self::DegenerateBoundary => "degenerate_boundary",
            Self::NonFiniteResidual => "non_finite_residual",
            Self::NonFiniteJacobian => "non_finite_jacobian",
            Self::NoDescentDirection => "no_descent_direction",
            Self::DidNotConverge => "did_not_converge",
            Self::ResourceLimitExceeded => "resource_limit_exceeded",
        }
    }
}

/// Baseline file containing reviewed deterministic digests.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Baseline {
    pub schema_version: u32,
    pub digest_version: String,
    pub cases: std::collections::BTreeMap<String, String>,
}
