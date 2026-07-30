//! Session-scoped topology tracking and deterministic operation lineage.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::result::Result;
use std::str::FromStr;

use crate::DeterministicContentHash;

/// Result type for topology tracking operations.
pub type TrackingResult<T> = Result<T, TrackingError>;

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
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = TrackingError;

            fn from_str(value: &str) -> TrackingResult<Self> {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
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
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct TrackingSessionId(String);

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
            Ok(Self(value))
        }
    }

    /// Returns this tracking-session identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for TrackingSessionId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for TrackingSessionId {
    type Err = TrackingError;

    fn from_str(value: &str) -> TrackingResult<Self> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for TrackingSessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl DeterministicContentHash for TrackingSessionId {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
    }
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
    feature: FeatureId,
    kind: TopologyKind,
    label: SemanticLabel,
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
    pub const fn feature(&self) -> &FeatureId {
        &self.feature
    }

    /// Returns the addressed topology kind.
    pub const fn kind(&self) -> TopologyKind {
        self.kind
    }

    /// Returns the semantic label within the feature result.
    pub const fn label(&self) -> &SemanticLabel {
        &self.label
    }
}

impl DeterministicContentHash for FeatureId {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
    }
}

impl DeterministicContentHash for SemanticLabel {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
    }
}

impl DeterministicContentHash for TopologyKind {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).content_hash(state);
    }
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
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric generation.
    pub const fn raw(self) -> u64 {
        self.0
    }

    fn next(self) -> TrackingResult<Self> {
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
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.0.content_hash(state);
    }
}

/// Immutable runtime topology identifier.
///
/// Identity is qualified by session, replay generation, and a session-wide
/// serial shared by vertices, edges, and faces.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrackingId {
    session: TrackingSessionId,
    generation: TrackingGeneration,
    serial: u64,
}

impl TrackingId {
    fn new(session: TrackingSessionId, generation: TrackingGeneration, serial: u64) -> Self {
        Self {
            session,
            generation,
            serial,
        }
    }

    /// Returns the owning tracking session.
    pub const fn session(&self) -> &TrackingSessionId {
        &self.session
    }

    /// Returns the replay generation in which this identifier was allocated.
    pub const fn generation(&self) -> TrackingGeneration {
        self.generation
    }

    /// Returns the generation-local monotonic serial.
    pub const fn serial(&self) -> u64 {
        self.serial
    }
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
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TrackingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
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
}

impl DeterministicContentHash for OperationKind {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).content_hash(state);
    }
}

impl DeterministicContentHash for LineageRelation {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).content_hash(state);
    }
}

/// One parent-to-children topology lineage event.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageEvent {
    operation: OperationKind,
    relation: LineageRelation,
    parent: TrackingId,
    children: Vec<TrackingId>,
}

impl LineageEvent {
    fn new(
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
    pub const fn operation(&self) -> OperationKind {
        self.operation
    }

    /// Returns the parent-to-child relationship.
    pub const fn relation(&self) -> LineageRelation {
        self.relation
    }

    /// Returns the parent identifier.
    pub const fn parent(&self) -> &TrackingId {
        &self.parent
    }

    /// Returns the child identifiers in deterministic operation order.
    pub fn children(&self) -> &[TrackingId] {
        &self.children
    }
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
    reference: SemanticTopologyRef,
    tracking_id: TrackingId,
}

impl SemanticBinding {
    fn new(reference: SemanticTopologyRef, tracking_id: TrackingId) -> Self {
        Self {
            reference,
            tracking_id,
        }
    }

    /// Returns the persistent semantic reference.
    pub const fn reference(&self) -> &SemanticTopologyRef {
        &self.reference
    }

    /// Returns the current runtime identifier.
    pub const fn tracking_id(&self) -> &TrackingId {
        &self.tracking_id
    }
}

impl DeterministicContentHash for SemanticBinding {
    fn content_hash<H: Hasher>(&self, state: &mut H) {
        self.reference.content_hash(state);
        self.tracking_id.content_hash(state);
    }
}

/// Serializable session state for topology tracking and replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrackingSession {
    id: TrackingSessionId,
    generation: TrackingGeneration,
    next_serial: u64,
    bindings: Vec<SemanticBinding>,
    lineage: Vec<LineageEvent>,
}

impl TrackingSession {
    /// Creates an empty tracking session at [`TrackingGeneration::INITIAL`].
    pub const fn new(id: TrackingSessionId) -> Self {
        Self {
            id,
            generation: TrackingGeneration::INITIAL,
            next_serial: 1,
            bindings: Vec::new(),
            lineage: Vec::new(),
        }
    }

    /// Returns the session identifier.
    pub const fn id(&self) -> &TrackingSessionId {
        &self.id
    }

    /// Returns the current replay generation.
    pub const fn generation(&self) -> TrackingGeneration {
        self.generation
    }

    /// Returns the serial that the next allocation will receive.
    pub const fn next_serial(&self) -> u64 {
        self.next_serial
    }

    /// Returns semantic bindings in canonical semantic-reference order.
    pub fn bindings(&self) -> &[SemanticBinding] {
        &self.bindings
    }

    /// Returns current-generation lineage in recording order.
    pub fn lineage(&self) -> &[LineageEvent] {
        &self.lineage
    }

    /// Allocates a fresh current-generation [`TrackingId`].
    ///
    /// Serials share one monotonic namespace across all topology kinds.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::SerialOverflow`] when the session exhausts its
    /// serial range.
    pub fn allocate(&mut self) -> TrackingResult<TrackingId> {
        let serial = self.next_serial;
        self.next_serial = self
            .next_serial
            .checked_add(1)
            .ok_or(TrackingError::SerialOverflow)?;
        Ok(TrackingId::new(self.id.clone(), self.generation, serial))
    }

    /// Binds a semantic reference to an allocated current-generation ID.
    ///
    /// Repeating the same binding is idempotent.
    ///
    /// # Errors
    ///
    /// Returns a typed [`TrackingError`] when `tracking_id` is not current, the
    /// semantic reference already names a different ID, or the ID already has
    /// a different semantic reference.
    pub fn bind(
        &mut self,
        reference: SemanticTopologyRef,
        tracking_id: TrackingId,
    ) -> TrackingResult<&SemanticBinding> {
        self.validate_current(&tracking_id)?;
        match self
            .bindings
            .binary_search_by(|binding| binding.reference.cmp(&reference))
        {
            Ok(index) if self.bindings[index].tracking_id == tracking_id => {
                Ok(&self.bindings[index])
            }
            Ok(index) => Err(TrackingError::SemanticReferenceAlreadyBound {
                reference: Box::new(reference),
                existing: Box::new(self.bindings[index].tracking_id.clone()),
                requested: Box::new(tracking_id),
            }),
            Err(index) => {
                if let Some(existing) = self
                    .bindings
                    .iter()
                    .find(|binding| binding.tracking_id == tracking_id)
                {
                    Err(TrackingError::TrackingIdAlreadyBound {
                        tracking_id: Box::new(tracking_id),
                        existing: Box::new(existing.reference.clone()),
                        requested: Box::new(reference),
                    })
                } else {
                    self.bindings
                        .insert(index, SemanticBinding::new(reference, tracking_id));
                    Ok(&self.bindings[index])
                }
            }
        }
    }

    /// Resolves a semantic reference in the current generation.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::UnknownSemanticReference`] when no current
    /// binding exists.
    pub fn resolve(&self, reference: &SemanticTopologyRef) -> TrackingResult<&TrackingId> {
        self.bindings
            .binary_search_by(|binding| binding.reference.cmp(reference))
            .map(|index| &self.bindings[index].tracking_id)
            .map_err(|_| TrackingError::UnknownSemanticReference(reference.clone()))
    }

    /// Validates that an identifier belongs to this current session generation.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::WrongSession`],
    /// [`TrackingError::StaleGeneration`], or
    /// [`TrackingError::UnknownTrackingId`] for invalid runtime handles.
    pub fn validate_current(&self, tracking_id: &TrackingId) -> TrackingResult<()> {
        if tracking_id.session != self.id {
            Err(TrackingError::WrongSession {
                expected: self.id.clone(),
                actual: tracking_id.session.clone(),
            })
        } else if tracking_id.generation != self.generation {
            Err(TrackingError::StaleGeneration {
                current: self.generation,
                actual: tracking_id.generation,
            })
        } else if tracking_id.serial == 0 || tracking_id.serial >= self.next_serial {
            Err(TrackingError::UnknownTrackingId(tracking_id.clone()))
        } else {
            Ok(())
        }
    }

    /// Records one parent-to-children lineage event.
    ///
    /// Child order is preserved because it can encode deterministic split
    /// roles such as the parameter-start and parameter-end pieces.
    ///
    /// # Errors
    ///
    /// Returns a typed [`TrackingError`] when an identifier is not current,
    /// there are no children, or a child occurs more than once.
    pub fn record_lineage(
        &mut self,
        operation: OperationKind,
        relation: LineageRelation,
        parent: TrackingId,
        children: impl IntoIterator<Item = TrackingId>,
    ) -> TrackingResult<&LineageEvent> {
        self.validate_current(&parent)?;
        let children: Vec<_> = children.into_iter().collect();
        if children.is_empty() {
            return Err(TrackingError::EmptyLineage);
        }
        children
            .iter()
            .try_for_each(|child| self.validate_current(child))?;
        let unique: BTreeSet<_> = children.iter().collect();
        if unique.len() != children.len() {
            return Err(TrackingError::DuplicateLineageChild);
        }
        self.lineage
            .push(LineageEvent::new(operation, relation, parent, children));
        Ok(&self.lineage[self.lineage.len() - 1])
    }

    /// Advances replay generation and clears generation-local state.
    ///
    /// The serial resets to one so replaying the same deterministic operation
    /// order produces the same serial sequence in the new generation.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::GenerationOverflow`] when the generation
    /// counter is exhausted.
    pub fn advance_generation(&mut self) -> TrackingResult<TrackingGeneration> {
        self.generation = self.generation.next()?;
        self.next_serial = 1;
        self.bindings.clear();
        self.lineage.clear();
        Ok(self.generation)
    }

    fn validate_state(&self) -> TrackingResult<()> {
        if self.next_serial == 0 {
            return Err(TrackingError::InvalidSessionState(
                "the next serial must be non-zero",
            ));
        }
        self.bindings
            .iter()
            .try_for_each(|binding| self.validate_current(&binding.tracking_id))?;
        if self
            .bindings
            .windows(2)
            .any(|pair| pair[0].reference >= pair[1].reference)
        {
            return Err(TrackingError::InvalidSessionState(
                "semantic bindings are not in unique canonical order",
            ));
        }
        let unique_binding_ids: BTreeSet<_> = self
            .bindings
            .iter()
            .map(|binding| &binding.tracking_id)
            .collect();
        if unique_binding_ids.len() != self.bindings.len() {
            return Err(TrackingError::InvalidSessionState(
                "a tracking ID has multiple semantic bindings",
            ));
        }
        self.lineage.iter().try_for_each(|event| {
            self.validate_current(&event.parent)?;
            if event.children.is_empty() {
                return Err(TrackingError::EmptyLineage);
            }
            event
                .children
                .iter()
                .try_for_each(|child| self.validate_current(child))?;
            let unique: BTreeSet<_> = event.children.iter().collect();
            if unique.len() == event.children.len() {
                Ok(())
            } else {
                Err(TrackingError::DuplicateLineageChild)
            }
        })
    }
}

#[derive(Deserialize)]
struct TrackingSessionRepresentation {
    id: TrackingSessionId,
    generation: TrackingGeneration,
    next_serial: u64,
    bindings: Vec<SemanticBinding>,
    lineage: Vec<LineageEvent>,
}

impl<'de> Deserialize<'de> for TrackingSession {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let representation = TrackingSessionRepresentation::deserialize(deserializer)?;
        let session = Self {
            id: representation.id,
            generation: representation.generation,
            next_serial: representation.next_serial,
            bindings: representation.bindings,
            lineage: representation.lineage,
        };
        session.validate_state().map_err(serde::de::Error::custom)?;
        Ok(session)
    }
}

/// Typed failures produced by topology tracking operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackingError {
    /// A textual identifier violates its validation rules.
    InvalidIdentifier {
        /// Kind of identifier being validated.
        kind: &'static str,
        /// Rejected value.
        value: String,
        /// Human-readable validation reason.
        reason: &'static str,
    },
    /// A compact [`TrackingId`] string is malformed.
    InvalidTrackingId {
        /// Rejected compact representation.
        value: String,
    },
    /// A runtime identifier belongs to another session.
    WrongSession {
        /// Required session.
        expected: TrackingSessionId,
        /// Identifier's actual session.
        actual: TrackingSessionId,
    },
    /// A runtime identifier belongs to an earlier or later replay generation.
    StaleGeneration {
        /// Current session generation.
        current: TrackingGeneration,
        /// Identifier's actual generation.
        actual: TrackingGeneration,
    },
    /// A runtime identifier was not allocated by the current generation.
    UnknownTrackingId(TrackingId),
    /// A semantic reference has no current-generation binding.
    UnknownSemanticReference(SemanticTopologyRef),
    /// A topological element has not been assigned a runtime identifier.
    UntrackedTopology(TopologyKind),
    /// A semantic reference addresses the wrong topology kind.
    TopologyKindMismatch {
        /// Required topology kind.
        expected: TopologyKind,
        /// Actual topology kind.
        actual: TopologyKind,
    },
    /// A semantic reference already points at a different runtime identifier.
    SemanticReferenceAlreadyBound {
        /// Conflicting semantic reference.
        reference: Box<SemanticTopologyRef>,
        /// Existing runtime identifier.
        existing: Box<TrackingId>,
        /// Requested runtime identifier.
        requested: Box<TrackingId>,
    },
    /// A runtime identifier already has another semantic reference.
    TrackingIdAlreadyBound {
        /// Conflicting runtime identifier.
        tracking_id: Box<TrackingId>,
        /// Existing semantic reference.
        existing: Box<SemanticTopologyRef>,
        /// Requested semantic reference.
        requested: Box<SemanticTopologyRef>,
    },
    /// A lineage event has no child.
    EmptyLineage,
    /// A lineage event repeats a child.
    DuplicateLineageChild,
    /// The replay-generation counter is exhausted.
    GenerationOverflow,
    /// The generation-local serial counter is exhausted.
    SerialOverflow,
    /// Serialized session state violates a runtime invariant.
    InvalidSessionState(&'static str),
}

impl Display for TrackingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentifier {
                kind,
                value,
                reason,
            } => write!(formatter, "invalid {kind} `{value}`: {reason}"),
            Self::InvalidTrackingId { value } => {
                write!(formatter, "invalid tracking ID `{value}`")
            }
            Self::WrongSession { expected, actual } => write!(
                formatter,
                "tracking ID belongs to session `{actual}`, expected `{expected}`",
            ),
            Self::StaleGeneration { current, actual } => write!(
                formatter,
                "tracking ID belongs to generation `{actual}`, current generation is `{current}`",
            ),
            Self::UnknownTrackingId(tracking_id) => {
                write!(formatter, "unknown tracking ID `{tracking_id}`")
            }
            Self::UnknownSemanticReference(reference) => write!(
                formatter,
                "unknown semantic topology reference `{}:{:?}:{}`",
                reference.feature, reference.kind, reference.label,
            ),
            Self::UntrackedTopology(kind) => {
                write!(formatter, "the {kind:?} topology is not tracked")
            }
            Self::TopologyKindMismatch { expected, actual } => write!(
                formatter,
                "topology kind `{actual:?}` does not match expected kind `{expected:?}`",
            ),
            Self::SemanticReferenceAlreadyBound {
                reference,
                existing,
                requested,
            } => write!(
                formatter,
                "semantic reference `{}:{:?}:{}` is bound to `{existing}`, not `{requested}`",
                reference.feature, reference.kind, reference.label,
            ),
            Self::TrackingIdAlreadyBound {
                tracking_id,
                existing,
                requested,
            } => write!(
                formatter,
                "tracking ID `{tracking_id}` is bound to `{}:{:?}:{}`, not `{}:{:?}:{}`",
                existing.feature,
                existing.kind,
                existing.label,
                requested.feature,
                requested.kind,
                requested.label,
            ),
            Self::EmptyLineage => formatter.write_str("a lineage event must have a child"),
            Self::DuplicateLineageChild => {
                formatter.write_str("a lineage event contains a duplicate child")
            }
            Self::GenerationOverflow => {
                formatter.write_str("the tracking generation counter is exhausted")
            }
            Self::SerialOverflow => formatter.write_str("the tracking serial counter is exhausted"),
            Self::InvalidSessionState(reason) => {
                write!(formatter, "invalid serialized tracking session: {reason}")
            }
        }
    }
}

impl std::error::Error for TrackingError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_id(value: &str) -> TrackingSessionId {
        TrackingSessionId::new(value).expect("the test session identifier is valid")
    }

    fn semantic(label: &str) -> SemanticTopologyRef {
        SemanticTopologyRef::new(
            FeatureId::new("feature-1").expect("the test feature identifier is valid"),
            TopologyKind::Face,
            SemanticLabel::new(label).expect("the test semantic label is valid"),
        )
    }

    #[test]
    fn allocation_uses_one_serial_namespace_for_all_topology() {
        let mut session = TrackingSession::new(session_id("browser-session"));
        let vertex = session.allocate().expect("the serial range is available");
        let edge = session.allocate().expect("the serial range is available");
        let face = session.allocate().expect("the serial range is available");

        assert_eq!([vertex.serial(), edge.serial(), face.serial()], [1, 2, 3]);
        assert_eq!(BTreeSet::from([vertex, edge, face]).len(), 3);
    }

    #[test]
    fn session_qualification_makes_equal_serials_globally_unique() {
        let mut first = TrackingSession::new(session_id("first"));
        let mut second = TrackingSession::new(session_id("second"));
        let first_id = first.allocate().expect("the serial range is available");
        let second_id = second.allocate().expect("the serial range is available");

        assert_eq!(first_id.serial(), second_id.serial());
        assert_ne!(first_id, second_id);
    }

    #[test]
    fn replay_resets_serials_but_invalidates_old_ids() {
        let mut session = TrackingSession::new(session_id("replay"));
        let reference = semantic("top");
        let old_id = session.allocate().expect("the serial range is available");
        session
            .bind(reference.clone(), old_id.clone())
            .expect("the binding is valid");
        session
            .record_lineage(
                OperationKind::Map,
                LineageRelation::Preserved,
                old_id.clone(),
                [old_id.clone()],
            )
            .expect("the lineage is valid");

        let generation = session
            .advance_generation()
            .expect("the generation range is available");
        let replayed_id = session.allocate().expect("the serial range is available");

        assert_eq!(generation, TrackingGeneration::new(1));
        assert_eq!(old_id.serial(), replayed_id.serial());
        assert_ne!(old_id, replayed_id);
        assert!(session.bindings().is_empty());
        assert!(session.lineage().is_empty());
        assert!(matches!(
            session.validate_current(&old_id),
            Err(TrackingError::StaleGeneration { .. }),
        ));
    }

    #[test]
    fn validation_rejects_cross_session_and_unknown_ids() {
        let mut first = TrackingSession::new(session_id("first"));
        let second = TrackingSession::new(session_id("second"));
        let allocated = first.allocate().expect("the serial range is available");
        let unknown: TrackingId = "first@0:2"
            .parse()
            .expect("the compact tracking ID is valid");

        assert!(matches!(
            second.validate_current(&allocated),
            Err(TrackingError::WrongSession { .. }),
        ));
        assert_eq!(
            first.validate_current(&unknown),
            Err(TrackingError::UnknownTrackingId(unknown)),
        );
    }

    #[test]
    fn binding_is_idempotent_and_resolves_current_id() {
        let mut session = TrackingSession::new(session_id("binding"));
        let reference = semantic("top");
        let tracking_id = session.allocate().expect("the serial range is available");

        session
            .bind(reference.clone(), tracking_id.clone())
            .expect("the binding is valid");
        session
            .bind(reference.clone(), tracking_id.clone())
            .expect("repeating the binding is idempotent");

        assert_eq!(session.bindings().len(), 1);
        assert_eq!(session.resolve(&reference), Ok(&tracking_id));
    }

    #[test]
    fn tracking_id_serde_uses_a_compact_string() {
        let mut session = TrackingSession::new(session_id("cad-session"));
        session
            .advance_generation()
            .expect("the generation range is available");
        let tracking_id = session.allocate().expect("the serial range is available");

        let json = serde_json::to_string(&tracking_id).expect("serialization succeeds");
        let restored: TrackingId = serde_json::from_str(&json).expect("deserialization succeeds");

        assert_eq!(json, "\"cad-session@1:1\"");
        assert_eq!(restored, tracking_id);
    }

    #[test]
    fn tracking_id_content_hash_covers_every_identity_component() {
        let first: TrackingId = "first@0:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_session: TrackingId = "second@0:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_generation: TrackingId = "first@1:1"
            .parse()
            .expect("the compact tracking ID is valid");
        let other_serial: TrackingId = "first@0:2"
            .parse()
            .expect("the compact tracking ID is valid");

        assert_ne!(first.content_hash64(), other_session.content_hash64());
        assert_ne!(first.content_hash64(), other_generation.content_hash64());
        assert_ne!(first.content_hash64(), other_serial.content_hash64());
    }

    #[test]
    fn tracking_session_round_trip_preserves_bindings_and_lineage() {
        let mut session = TrackingSession::new(session_id("serialized"));
        let parent = session.allocate().expect("the serial range is available");
        let child = session.allocate().expect("the serial range is available");
        session
            .bind(semantic("source"), parent.clone())
            .expect("the binding is valid");
        session
            .bind(semantic("result"), child.clone())
            .expect("the binding is valid");
        session
            .record_lineage(
                OperationKind::Extrude,
                LineageRelation::Generated,
                parent,
                [child],
            )
            .expect("the lineage is valid");

        let json = serde_json::to_string(&session).expect("serialization succeeds");
        let restored: TrackingSession =
            serde_json::from_str(&json).expect("deserialization succeeds");

        assert_eq!(restored, session);
    }

    #[test]
    fn validated_strings_reject_ambiguous_or_unstable_values() {
        assert!(TrackingSessionId::new("contains:separator").is_err());
        assert!(TrackingSessionId::new("contains@separator").is_err());
        assert!(FeatureId::new(" feature").is_err());
        assert!(SemanticLabel::new("").is_err());
    }
}
