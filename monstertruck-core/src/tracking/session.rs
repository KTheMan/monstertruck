use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use super::TrackingResult;
use super::model::*;

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
    pub const fn id(&self) -> &TrackingSessionId { &self.id }

    /// Returns the current replay generation.
    pub const fn generation(&self) -> TrackingGeneration { self.generation }

    /// Returns the serial that the next allocation will receive.
    pub const fn next_serial(&self) -> u64 { self.next_serial }

    /// Returns semantic bindings in canonical semantic-reference order.
    pub fn bindings(&self) -> &[SemanticBinding] { &self.bindings }

    /// Returns current-generation lineage in recording order.
    pub fn lineage(&self) -> &[LineageEvent] { &self.lineage }

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

    /// Returns the semantic binding for a current runtime identifier.
    ///
    /// An allocated identifier may be intentionally unbound, in which case
    /// this returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Returns the same session, generation, and allocation errors as
    /// [`Self::validate_current`].
    pub fn binding_for_tracking_id(
        &self,
        tracking_id: &TrackingId,
    ) -> TrackingResult<Option<&SemanticBinding>> {
        self.validate_current(tracking_id)?;
        Ok(self
            .bindings
            .iter()
            .find(|binding| binding.tracking_id == *tracking_id))
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
    /// Returns a typed [`TrackingError`] when an identifier is not current, a
    /// non-deleted event has no children, a deleted event has children, or a
    /// child occurs more than once.
    pub fn record_lineage(
        &mut self,
        operation: OperationKind,
        relation: LineageRelation,
        parent: TrackingId,
        children: impl IntoIterator<Item = TrackingId>,
    ) -> TrackingResult<&LineageEvent> {
        self.validate_current(&parent)?;
        let children: Vec<_> = children.into_iter().collect();
        if children.is_empty() && relation != LineageRelation::Deleted {
            return Err(TrackingError::EmptyLineage);
        } else if !children.is_empty() && relation == LineageRelation::Deleted {
            return Err(TrackingError::DeletedLineageHasChildren);
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

    /// Resolves the current terminal descendants of a tracked selection.
    ///
    /// Events are applied in recording order, so preserved identities can
    /// continue through later operations while split identities expand into
    /// every surviving child. A deleted selection resolves to an empty set.
    ///
    /// # Errors
    ///
    /// Returns the same session, generation, and allocation errors as
    /// [`Self::validate_current`].
    pub fn descendants(&self, tracking_id: &TrackingId) -> TrackingResult<Vec<TrackingId>> {
        self.validate_current(tracking_id)?;
        Ok(self
            .lineage
            .iter()
            .fold(vec![tracking_id.clone()], |mut descendants, event| {
                if let Some(index) = descendants
                    .iter()
                    .position(|candidate| candidate == event.parent())
                {
                    if event.relation() == LineageRelation::Generated {
                        descendants.splice(index + 1..index + 1, event.children().iter().cloned());
                    } else {
                        descendants.splice(index..=index, event.children().iter().cloned());
                    }
                }
                descendants
            }))
    }

    /// Resolves a semantic selection to its current terminal descendants.
    ///
    /// # Errors
    ///
    /// Returns [`TrackingError::UnknownSemanticReference`] when the selection
    /// is unknown, or the same session, generation, and allocation errors as
    /// [`Self::descendants`].
    pub fn resolve_descendants(
        &self,
        reference: &SemanticTopologyRef,
    ) -> TrackingResult<Vec<TrackingId>> {
        self.descendants(self.resolve(reference)?)
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
            if event.children.is_empty() && event.relation() != LineageRelation::Deleted {
                return Err(TrackingError::EmptyLineage);
            } else if !event.children.is_empty() && event.relation() == LineageRelation::Deleted {
                return Err(TrackingError::DeletedLineageHasChildren);
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

    fn share_session_id_storage(&mut self) {
        self.bindings.iter_mut().for_each(|binding| {
            binding.tracking_id.session = self.id.clone();
        });
        self.lineage.iter_mut().for_each(|event| {
            event.parent.session = self.id.clone();
            event.children.iter_mut().for_each(|child| {
                child.session = self.id.clone();
            });
        });
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
    where D: Deserializer<'de> {
        let representation = TrackingSessionRepresentation::deserialize(deserializer)?;
        let mut session = Self {
            id: representation.id,
            generation: representation.generation,
            next_serial: representation.next_serial,
            bindings: representation.bindings,
            lineage: representation.lineage,
        };
        session.validate_state().map_err(serde::de::Error::custom)?;
        session.share_session_id_storage();
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
    /// A deleted lineage event contains a child.
    DeletedLineageHasChildren,
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
            Self::DeletedLineageHasChildren => {
                formatter.write_str("a deleted lineage event cannot contain a child")
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
