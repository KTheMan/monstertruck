use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::str::FromStr;
use std::sync::Arc;

use crate::DeterministicContentHash;

use super::{TrackingError, TrackingResult};

fn validate_text(value: &str, kind: &'static str) -> TrackingResult<()> {
    if value.is_empty() {
        Err(TrackingError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "the value is empty",
        })
    } else if value.trim() != value {
        Err(TrackingError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "leading or trailing whitespace is not allowed",
        })
    } else if value.chars().any(char::is_control) {
        Err(TrackingError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
            reason: "control characters are not allowed",
        })
    } else {
        Ok(())
    }
}

macro_rules! string_identifier {
    ($name:ident, $kind:literal) => {
        #[doc = concat!("Validated ", $kind, ".")]
        #[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a validated ", $kind, ".")]
            ///
            /// # Errors
            ///
            /// Returns [`TrackingError::InvalidIdentifier`] when `value` is
            /// empty, contains control characters, or has leading or trailing
            /// whitespace.
            pub fn new(value: impl Into<String>) -> TrackingResult<Self> {
                let value = value.into();
                validate_text(&value, $kind)?;
                Ok(Self(value))
            }

            #[doc = concat!("Returns this ", $kind, " as a string slice.")]
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = TrackingError;

            fn from_str(value: &str) -> TrackingResult<Self> { Self::new(value) }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

string_identifier!(FeatureId, "feature identifier");
string_identifier!(SemanticLabel, "semantic topology label");

/// Validated identifier for one live tracking session.
///
/// The characters `@` and `:` are reserved by [`TrackingId`]'s compact
/// representation.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrackingSessionId(Arc<str>);

impl TrackingSessionId {
    /// Creates a validated tracking-session identifier.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::InvalidIdentifier`] when `value` is empty,
    /// contains reserved or control characters, or has leading or trailing
    /// whitespace.
    pub fn new(value: impl Into<String>) -> TrackingResult<Self> {
        let value = value.into();
        validate_text(&value, "tracking session identifier")?;
        if value.contains(['@', ':']) {
            Err(TrackingError::InvalidIdentifier {
                kind: "tracking session identifier",
                value,
                reason: "`@` and `:` are reserved separators",
            })
        } else {
            Ok(Self(Arc::from(value)))
        }
    }

    /// Returns this tracking-session identifier as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl Display for TrackingSessionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for TrackingSessionId {
    type Err = TrackingError;

    fn from_str(value: &str) -> TrackingResult<Self> { Self::new(value) }
}

impl Serialize for TrackingSessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TrackingSessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl DeterministicContentHash for TrackingSessionId {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.as_str().content_hash(state); }
}

/// Kind of topological element addressed by a [`SemanticTopologyRef`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyKind {
    /// A topological vertex.
    Vertex,
    /// A topological edge.
    Edge,
    /// A topological face.
    Face,
}

/// Persistent semantic address of a topology element in a feature result.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticTopologyRef {
    pub(super) feature: FeatureId,
    pub(super) kind: TopologyKind,
    pub(super) label: SemanticLabel,
}

impl SemanticTopologyRef {
    /// Creates a semantic topology reference.
    pub const fn new(feature: FeatureId, kind: TopologyKind, label: SemanticLabel) -> Self {
        Self {
            feature,
            kind,
            label,
        }
    }

    /// Returns the feature that owns the topology result.
    pub const fn feature(&self) -> &FeatureId { &self.feature }

    /// Returns the addressed topology kind.
    pub const fn kind(&self) -> TopologyKind { self.kind }

    /// Returns the semantic label within the feature result.
    pub const fn label(&self) -> &SemanticLabel { &self.label }
}

impl DeterministicContentHash for FeatureId {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.0.content_hash(state); }
}

impl DeterministicContentHash for SemanticLabel {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.0.content_hash(state); }
}

impl DeterministicContentHash for TopologyKind {
    fn content_hash<H: Hasher>(&self, state: &mut H) { (*self as u8).content_hash(state); }
}

impl DeterministicContentHash for SemanticTopologyRef {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.feature.content_hash(state);
        self.kind.content_hash(state);
        self.label.content_hash(state);
    }
}

/// Replay generation within a [`TrackingSession`].
#[derive(
    Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TrackingGeneration(u64);

impl TrackingGeneration {
    /// Initial replay generation.
    pub const INITIAL: Self = Self(0);

    /// Creates a generation from its numeric value.
    pub const fn new(value: u64) -> Self { Self(value) }

    /// Returns the numeric generation.
    pub const fn raw(self) -> u64 { self.0 }

    pub(super) fn next(self) -> TrackingResult<Self> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(TrackingError::GenerationOverflow)
    }
}

impl Display for TrackingGeneration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl DeterministicContentHash for TrackingGeneration {
    fn content_hash<H: Hasher>(&self, state: &mut H) { self.0.content_hash(state); }
}

/// Immutable runtime topology identifier.
///
/// Identity is qualified by session, replay generation, and a session-wide
/// serial shared by vertices, edges, and faces.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrackingId {
    pub(super) session: TrackingSessionId,
    pub(super) generation: TrackingGeneration,
    pub(super) serial: u64,
}

impl TrackingId {
    pub(super) fn new(
        session: TrackingSessionId,
        generation: TrackingGeneration,
        serial: u64,
    ) -> Self {
        Self {
            session,
            generation,
            serial,
        }
    }

    /// Returns the owning tracking session.
    pub const fn session(&self) -> &TrackingSessionId { &self.session }

    /// Returns the replay generation in which this identifier was allocated.
    pub const fn generation(&self) -> TrackingGeneration { self.generation }

    /// Returns the generation-local monotonic serial.
    pub const fn serial(&self) -> u64 { self.serial }
}

impl Display for TrackingId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}@{}:{}",
            self.session, self.generation, self.serial
        )
    }
}

impl FromStr for TrackingId {
    type Err = TrackingError;

    fn from_str(value: &str) -> TrackingResult<Self> {
        let (session, suffix) =
            value
                .split_once('@')
                .ok_or_else(|| TrackingError::InvalidTrackingId {
                    value: value.to_owned(),
                })?;
        let (generation, serial) =
            suffix
                .split_once(':')
                .ok_or_else(|| TrackingError::InvalidTrackingId {
                    value: value.to_owned(),
                })?;
        let session = TrackingSessionId::new(session)?;
        let generation = generation
            .parse()
            .map_err(|_| TrackingError::InvalidTrackingId {
                value: value.to_owned(),
            })?;
        let serial = serial
            .parse()
            .map_err(|_| TrackingError::InvalidTrackingId {
                value: value.to_owned(),
            })?;
        if serial == 0 {
            Err(TrackingError::InvalidTrackingId {
                value: value.to_owned(),
            })
        } else {
            Ok(Self::new(
                session,
                TrackingGeneration::new(generation),
                serial,
            ))
        }
    }
}

impl Serialize for TrackingId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TrackingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl DeterministicContentHash for TrackingId {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.session.content_hash(state);
        self.generation.content_hash(state);
        self.serial.content_hash(state);
    }
}

/// Topology operation that created a lineage event.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// A geometric mapping.
    Map,
    /// A rotation.
    Rotate,
    /// A generic sweep.
    Sweep,
    /// A linear extrusion.
    Extrude,
    /// A rotational sweep.
    Revolve,
    /// A cut operation.
    Cut,
    /// A topology split.
    Split,
    /// A topology clone.
    Clone,
    /// A translation.
    Translate,
    /// A non-uniform scale.
    Scale,
    /// A Boolean intersection.
    BooleanIntersection,
    /// A Boolean union.
    BooleanUnion,
    /// A Boolean difference.
    BooleanDifference,
    /// A Boolean symmetric difference.
    BooleanSymmetricDifference,
    /// An edge fillet.
    Fillet,
}

/// Relationship between a lineage parent and its children.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageRelation {
    /// The semantic element was preserved by the operation.
    Preserved,
    /// The operation generated the children from the parent.
    Generated,
    /// The operation split the parent into multiple children.
    Split,
    /// The operation removed the parent without a surviving descendant.
    Deleted,
}

impl DeterministicContentHash for OperationKind {
    fn content_hash<H: Hasher>(&self, state: &mut H) { (*self as u8).content_hash(state); }
}

impl DeterministicContentHash for LineageRelation {
    fn content_hash<H: Hasher>(&self, state: &mut H) { (*self as u8).content_hash(state); }
}

/// One parent-to-children topology lineage event.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageEvent {
    operation: OperationKind,
    relation: LineageRelation,
    pub(super) parent: TrackingId,
    pub(super) children: Vec<TrackingId>,
}

impl LineageEvent {
    pub(super) fn new(
        operation: OperationKind,
        relation: LineageRelation,
        parent: TrackingId,
        children: Vec<TrackingId>,
    ) -> Self {
        Self {
            operation,
            relation,
            parent,
            children,
        }
    }

    /// Returns the operation that produced this event.
    pub const fn operation(&self) -> OperationKind { self.operation }

    /// Returns the parent-to-child relationship.
    pub const fn relation(&self) -> LineageRelation { self.relation }

    /// Returns the parent identifier.
    pub const fn parent(&self) -> &TrackingId { &self.parent }

    /// Returns the child identifiers in deterministic operation order.
    pub fn children(&self) -> &[TrackingId] { &self.children }
}

impl DeterministicContentHash for LineageEvent {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.operation.content_hash(state);
        self.relation.content_hash(state);
        self.parent.content_hash(state);
        self.children.content_hash(state);
    }
}

/// Current-generation binding from a semantic reference to a runtime identifier.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticBinding {
    pub(super) reference: SemanticTopologyRef,
    pub(super) tracking_id: TrackingId,
}

impl SemanticBinding {
    pub(super) fn new(reference: SemanticTopologyRef, tracking_id: TrackingId) -> Self {
        Self {
            reference,
            tracking_id,
        }
    }

    /// Returns the persistent semantic reference.
    pub const fn reference(&self) -> &SemanticTopologyRef { &self.reference }

    /// Returns the current runtime identifier.
    pub const fn tracking_id(&self) -> &TrackingId { &self.tracking_id }
}

impl DeterministicContentHash for SemanticBinding {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.reference.content_hash(state);
        self.tracking_id.content_hash(state);
    }
}
