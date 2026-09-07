// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The overlay document, and the authoring session that keys entries against
//! one form definition.
//!
//! The overlay is stored apart from the compiled definition and is never
//! merged into it. That separation is the whole design: a recompile produces a
//! new definition, the overlay is replayed against it, and the report says
//! what the revision did to the hand-authored work
//! (`docs/architecture.md` section 6.1).

use std::collections::{BTreeMap, BTreeSet};

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::ids::{RmTypeName, TemplateId};
use ferrochart_form::key::NodeKey;
use ferrochart_form::layout::{ColumnCount, Layout, Section, SectionId};
use serde::{Deserialize, Serialize};

use crate::advice::Advisory;
use crate::entry::{OverlayEntry, PositionalAnchor};
use crate::error::OverlayError;
use crate::index::Index;

/// The version of the overlay format this crate defines.
///
/// A reader refuses a document that states any other version, in both
/// directions: a build without geometry refuses a document written with it,
/// and this build refuses the version before it rather than reading a section
/// that states no column count as though the person had chosen one. A stored
/// key shape cannot change at all without a migration
/// (`docs/architecture.md` section 6.4).
pub const FORMAT_VERSION: u32 = 2;

/// The form the template identifier an overlay was keyed against takes.
///
/// openEHR ITS-REST Release-1.1.0 `definition.html`, "Get a template", records
/// four forms a `template_id` takes and states that "A partial `template_id`
/// will resolve to 'latest' major version of that template", so an identifier
/// that carries no full three-part version does not name one template
/// release. An overlay records which form it was keyed against, because
/// replaying it against "the latest major version" is a different act from
/// replaying it against a named release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TemplateIdForm {
    /// A legacy name with no version at all, such as `Vital Signs`.
    LegacyName,
    /// A legacy name carrying a major version, such as `vital_signs.v1`.
    LegacyMajorVersion,
    /// A human-readable identifier carrying a full three-part version, such
    /// as `openEHR-EHR-COMPOSITION.t_vital_signs.v1.0.0`.
    Hrid,
    /// A human-readable identifier carrying a major version only, such as
    /// `openEHR-EHR-COMPOSITION.t_vital_signs.v1`, which resolves to the
    /// latest release of that major version.
    PartialHrid,
}

impl TemplateIdForm {
    /// The form `id` takes.
    #[must_use]
    pub fn of(id: &TemplateId) -> Self {
        let text = id.as_str();
        let body = text.rsplit("::").next().unwrap_or(text);
        let segments: Vec<&str> = body.split('.').collect();
        let is_hrid = segments
            .first()
            .is_some_and(|head| head.split('-').filter(|part| !part.is_empty()).count() == 3);
        let version_at = segments
            .iter()
            .position(|segment| is_major_version(segment));
        match (is_hrid, version_at) {
            (true, Some(position)) => {
                if release_version_follows(&segments, position) {
                    Self::Hrid
                } else {
                    Self::PartialHrid
                }
            }
            (true, None) => Self::PartialHrid,
            (false, Some(_)) => Self::LegacyMajorVersion,
            (false, None) => Self::LegacyName,
        }
    }

    /// Whether the identifier names one release of one template.
    ///
    /// Only a full human-readable identifier does. Every other form leaves a
    /// server to pick a release, so an overlay keyed against one was keyed
    /// against whatever that server held at the time.
    #[must_use]
    pub const fn pins_one_release(self) -> bool {
        matches!(self, Self::Hrid)
    }
}

/// Whether `segment` is the `vN` part of an identifier.
fn is_major_version(segment: &str) -> bool {
    let Some(digits) = segment.strip_prefix('v') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// Whether a minor and a patch segment follow the major version.
fn release_version_follows(segments: &[&str], position: usize) -> bool {
    let rest = segments
        .get(position.saturating_add(1)..)
        .unwrap_or_default();
    rest.len() == 2
        && rest
            .iter()
            .all(|segment| segment.starts_with(|first: char| first.is_ascii_digit()))
}

/// The template an overlay was keyed against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateBinding {
    /// The identifier the operational template states for itself.
    pub id: TemplateId,
    /// The form that identifier takes.
    pub id_form: TemplateIdForm,
}

impl TemplateBinding {
    /// The binding of `id`, with its form classified.
    #[must_use]
    pub fn of(id: TemplateId) -> Self {
        let id_form = TemplateIdForm::of(&id);
        Self { id, id_form }
    }
}

/// Where one authored entry landed.
///
/// A key that names several nodes is reported here rather than resolved
/// silently, because the entry then depends on a sibling ordinal and a replay
/// has to treat it differently (`docs/architecture.md` section 6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
#[non_exhaustive]
pub enum Placement {
    /// The key names exactly one node of the definition.
    Unique,
    /// The key names several nodes, told apart by nothing but their position.
    Positional {
        /// How many nodes the key names, this one included.
        tied: usize,
    },
}

/// The overlay document, as it is stored and read back.
///
/// Entries are held in key order and sections in identifier order, so
/// serializing the same overlay twice produces the same bytes and an
/// unchanged overlay round-trips identically
/// (`.claude/rules/reliability.md`).
///
/// The type writes itself as JSON and is read back only through
/// [`Overlay::from_json`], which is the one path that checks the document
/// against itself. It implements no [`serde::Deserialize`], because a
/// deserializer that reported "not this format" for a section cycle or a
/// duplicated entry would flatten every structural refusal into one.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(into = "Document")]
pub struct Overlay {
    template: TemplateBinding,
    sections: Vec<Section>,
    entries: Vec<OverlayEntry>,
}

/// The overlay as it appears on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format_version: u32,
    template: TemplateBinding,
    sections: Vec<Section>,
    entries: Vec<OverlayEntry>,
}

impl From<Overlay> for Document {
    fn from(overlay: Overlay) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            template: overlay.template,
            sections: overlay.sections,
            entries: overlay.entries,
        }
    }
}

impl Overlay {
    /// The overlay one read document describes.
    fn from_document(document: Document) -> Result<Self, OverlayError> {
        if document.format_version != FORMAT_VERSION {
            return Err(OverlayError::UnsupportedFormatVersion {
                stated: document.format_version,
                supported: FORMAT_VERSION,
            });
        }
        let mut overlay = Self {
            template: document.template,
            sections: document.sections,
            entries: document.entries,
        };
        overlay
            .sections
            .sort_by(|left, right| left.id.cmp(&right.id));
        overlay
            .entries
            .sort_by(|left, right| left.key.cmp(&right.key));
        overlay.validate()?;
        Ok(overlay)
    }
}

impl Overlay {
    /// An overlay with nothing authored, keyed against `definition`.
    #[must_use]
    pub fn for_definition(definition: &FormDefinition) -> Self {
        Self {
            template: TemplateBinding::of(definition.template_id.clone()),
            sections: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// The template this overlay was keyed against.
    #[must_use]
    pub fn template(&self) -> &TemplateBinding {
        &self.template
    }

    /// Every authored section, in identifier order.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// The section `id` names.
    #[must_use]
    pub fn section(&self, id: &SectionId) -> Option<&Section> {
        self.sections.iter().find(|section| section.id == *id)
    }

    /// Everything the overlay stores and a person should still see, in
    /// section order and then in key order.
    ///
    /// An advisory is not a refusal. Each one is a state a person's own edit
    /// reaches, and the overlay keeps their work and says what is wrong with
    /// it rather than trimming the document to stay self-consistent.
    #[must_use]
    pub fn advisories(&self) -> Vec<Advisory> {
        let mut found: Vec<Advisory> = self
            .sections
            .iter()
            .filter(|section| section.columns.is_crowded())
            .map(|section| Advisory::CrowdedSection {
                section: section.id.clone(),
                columns: section.columns,
            })
            .collect();
        found.extend(
            self.entries
                .iter()
                .filter_map(|entry| self.geometry_of(entry)),
        );
        found
    }

    /// What is wrong with the geometry of one entry.
    ///
    /// At most one thing can be: a section that is gone has no column count
    /// for a span to overflow, so the missing grouping is the whole report.
    fn geometry_of(&self, entry: &OverlayEntry) -> Option<Advisory> {
        let id = entry.layout.section.as_ref()?;
        let Some(section) = self.section(id) else {
            return Some(Advisory::SectionDisappeared {
                key: entry.key.clone(),
                section: id.clone(),
            });
        };
        let span = entry.layout.geometry.span?;
        (span.get() > section.columns.get()).then(|| Advisory::SpanOverflows {
            key: entry.key.clone(),
            span,
            section: id.clone(),
            columns: section.columns,
        })
    }

    /// Every entry, in key order.
    #[must_use]
    pub fn entries(&self) -> &[OverlayEntry] {
        &self.entries
    }

    /// The entry decorating the node `key` names.
    #[must_use]
    pub fn entry(&self, key: &NodeKey) -> Option<&OverlayEntry> {
        self.position(key).and_then(|at| self.entries.get(at))
    }

    /// Drops the entry decorating the node `key` names, returning it.
    ///
    /// Nothing else drops an entry. A replay retains an entry it could not
    /// resolve, so a later revision that restores the node restores its
    /// layout, and only a person removes one.
    pub fn remove(&mut self, key: &NodeKey) -> Option<OverlayEntry> {
        self.position(key).map(|at| self.entries.remove(at))
    }

    /// Drops the section `id` names, returning it.
    ///
    /// Entries that belong to the section keep naming it. Clearing them would
    /// drop the grouping of every item in the section, which is the work the
    /// overlay exists to keep, so the reference is retained and
    /// [`Overlay::advisories`] reports it: the same rule an unresolved entry
    /// follows, and restoring the section restores the grouping.
    ///
    /// # Errors
    /// [`OverlayError::SectionInUse`] when another section sits inside this
    /// one, because that section would then name a parent the overlay does not
    /// define and no reader accepts such a document.
    pub fn remove_section(&mut self, id: &SectionId) -> Result<Option<Section>, OverlayError> {
        if let Some(inner) = self
            .sections
            .iter()
            .find(|section| section.parent.as_ref() == Some(id))
        {
            return Err(OverlayError::SectionInUse {
                section: id.to_string(),
                holds: inner.id.to_string(),
            });
        }
        let Some(at) = self.sections.iter().position(|section| section.id == *id) else {
            return Ok(None);
        };
        Ok(Some(self.sections.remove(at)))
    }

    /// Reads an overlay document.
    ///
    /// # Errors
    /// [`OverlayError::Malformed`] when the text is not this format,
    /// [`OverlayError::UnsupportedFormatVersion`] when it states another
    /// version, and one of the structural variants when the document
    /// contradicts itself.
    pub fn from_json(text: &str) -> Result<Self, OverlayError> {
        // NOTE: no specification governs this: our own design. A member this
        // format does not define is refused rather than preserved, because
        // the format is FerroCHART's own.
        let document = serde_json::from_str::<Document>(text)
            .map_err(|source| OverlayError::Malformed { source })?;
        Self::from_document(document)
    }

    /// Writes the overlay as a document.
    ///
    /// The bytes are deterministic: entries are in key order, sections are in
    /// identifier order, and the one map the format holds is a
    /// [`std::collections::BTreeMap`] in `ferrochart-form`, so an unchanged
    /// overlay writes identically every time. This crate cannot keep that last
    /// part true on its own, which is why the layout types assert their own
    /// ordering where they are defined.
    ///
    /// # Errors
    /// [`OverlayError::NotSerializable`] when the JSON writer refuses a value,
    /// which a layout built through this crate's own types cannot produce.
    pub fn to_json(&self) -> Result<String, OverlayError> {
        serde_json::to_string_pretty(self)
            .map_err(|source| OverlayError::NotSerializable { source })
    }

    /// Drops the entry decorating the node `key` names, if there is one.
    fn discard(&mut self, key: &NodeKey) {
        if let Some(at) = self.position(key) {
            self.entries.remove(at);
        }
    }

    fn position(&self, key: &NodeKey) -> Option<usize> {
        self.entries
            .binary_search_by(|entry| entry.key.steps.cmp(&key.steps))
            .ok()
    }

    /// Records `entry`, replacing whatever decorated the same node.
    fn put(&mut self, entry: OverlayEntry) {
        match self
            .entries
            .binary_search_by(|held| held.key.steps.cmp(&entry.key.steps))
        {
            Ok(at) => {
                if let Some(held) = self.entries.get_mut(at) {
                    *held = entry;
                }
            }
            Err(at) => self.entries.insert(at, entry),
        }
    }

    /// Refuses a document that contradicts itself.
    fn validate(&self) -> Result<(), OverlayError> {
        let mut known: BTreeMap<&SectionId, Option<&SectionId>> = BTreeMap::new();
        for section in &self.sections {
            if known.insert(&section.id, section.parent.as_ref()).is_some() {
                return Err(OverlayError::DuplicateSection {
                    section: section.id.to_string(),
                });
            }
        }
        for section in &self.sections {
            if let Some(parent) = section.parent.as_ref() {
                if !known.contains_key(parent) {
                    return Err(OverlayError::UnknownSection {
                        referrer: format!("section {}", section.id),
                        section: parent.to_string(),
                    });
                }
                walk_to_root(&known, &section.id)?;
            }
        }
        // The entries are in key order, so two entries on one node are
        // adjacent.
        for pair in self.entries.windows(2) {
            if let [left, right] = pair
                && left.key.steps == right.key.steps
            {
                return Err(OverlayError::DuplicateEntry {
                    key: left.key.to_string(),
                });
            }
        }
        // NOTE: no specification governs this: our own design. An entry naming
        // a section the document does not define is reported rather than
        // refused, because dropping a section is a person's own edit.
        for entry in &self.entries {
            if entry.key.is_positional != entry.anchor.is_some() {
                return Err(OverlayError::AnchorMismatch {
                    key: entry.key.to_string(),
                    is_positional: entry.key.is_positional,
                    has_anchor: entry.anchor.is_some(),
                });
            }
        }
        Ok(())
    }
}

/// Refuses a chain of section parents that leads back to where it started.
fn walk_to_root(
    known: &BTreeMap<&SectionId, Option<&SectionId>>,
    from: &SectionId,
) -> Result<(), OverlayError> {
    let mut seen: BTreeSet<&SectionId> = BTreeSet::new();
    let mut at = Some(from);
    while let Some(section) = at {
        if !seen.insert(section) {
            return Err(OverlayError::SectionCycle {
                section: section.to_string(),
            });
        }
        at = known.get(section).copied().flatten();
    }
    Ok(())
}

/// What an error calls the container an item is laid out in.
fn named(section: Option<&SectionId>) -> String {
    section.map_or_else(
        || "the form itself".to_owned(),
        |id| format!("section \"{id}\""),
    )
}

/// Whether two keys name the same node, whatever either says about being
/// positionally keyed.
fn same_steps(left: &NodeKey, right: &NodeKey) -> bool {
    left.steps == right.steps
}

/// An authoring session over one form definition.
///
/// The session holds the index of the definition, so keying a whole form costs
/// one walk of it rather than one per entry. Every entry it writes is checked
/// against the definition: an entry that decorates no node of the form is
/// refused, and an entry the definition tells from its siblings by nothing but
/// position is reported as such rather than stored as if it were unique.
#[derive(Debug)]
pub struct Author<'a> {
    definition: &'a FormDefinition,
    index: Index<'a>,
    overlay: Overlay,
}

impl<'a> Author<'a> {
    /// An authoring session over `definition`, starting from nothing.
    #[must_use]
    pub fn new(definition: &'a FormDefinition) -> Self {
        Self {
            definition,
            index: Index::new(definition),
            overlay: Overlay::for_definition(definition),
        }
    }

    /// An authoring session over `definition`, continuing `overlay`.
    ///
    /// # Errors
    /// [`OverlayError::TemplateMismatch`] when the overlay was keyed against
    /// another template. A revision of one template keeps its identifier, so a
    /// different identifier is a different form rather than a newer one.
    pub fn resume(definition: &'a FormDefinition, overlay: Overlay) -> Result<Self, OverlayError> {
        if overlay.template.id != definition.template_id {
            return Err(OverlayError::TemplateMismatch {
                overlay: overlay.template.id.to_string(),
                definition: definition.template_id.to_string(),
            });
        }
        Ok(Self {
            definition,
            index: Index::new(definition),
            overlay,
        })
    }

    /// The overlay as it stands.
    #[must_use]
    pub fn overlay(&self) -> &Overlay {
        &self.overlay
    }

    /// The overlay, taken out of the session.
    #[must_use]
    pub fn finish(self) -> Overlay {
        self.overlay
    }

    /// Records `section`, replacing any section with the same identifier, and
    /// returns what a person should be told about it.
    ///
    /// A section is accepted at any width the type admits. Narrowing one an
    /// item was already laid out in is accepted too, and the entries that no
    /// longer fit come back as advisories rather than being refused or
    /// rewritten, because the span the person authored is the record of what
    /// they wanted (`docs/architecture.md` section 6.3).
    ///
    /// # Errors
    /// [`OverlayError::UnknownSection`] when the section names a parent the
    /// overlay does not define, and [`OverlayError::SectionCycle`] when the
    /// chain of parents leads back to the section itself.
    pub fn add_section(&mut self, section: Section) -> Result<Vec<Advisory>, OverlayError> {
        if let Some(parent) = section.parent.as_ref()
            && self.overlay.section(parent).is_none()
        {
            return Err(OverlayError::UnknownSection {
                referrer: format!("section {}", section.id),
                section: parent.to_string(),
            });
        }
        let id = section.id.clone();
        let mut sections = self.overlay.sections.clone();
        sections.retain(|held| held.id != section.id);
        sections.push(section);
        sections.sort_by(|left, right| left.id.cmp(&right.id));
        let known: BTreeMap<&SectionId, Option<&SectionId>> = sections
            .iter()
            .map(|held| (&held.id, held.parent.as_ref()))
            .collect();
        for held in &sections {
            walk_to_root(&known, &held.id)?;
        }
        self.overlay.sections = sections;
        Ok(self
            .overlay
            .advisories()
            .into_iter()
            .filter(|advisory| *advisory.section() == id)
            .collect())
    }

    /// Records `layout` against the node `key` names, replacing what was
    /// there.
    ///
    /// The key is normalized against the definition: it is stored as the
    /// definition spells it, and it is marked positionally keyed only where
    /// the definition genuinely tells the node from its siblings by nothing
    /// but its ordinal.
    ///
    /// # Errors
    /// [`OverlayError::NoSuchNode`] when the key names no group and no field
    /// of the form, [`OverlayError::UnknownSection`] when the layout names a
    /// section the overlay does not define, and
    /// [`OverlayError::SpanExceedsColumns`] when it spans more columns than
    /// that section has.
    pub fn set(&mut self, key: &NodeKey, layout: Layout) -> Result<Placement, OverlayError> {
        let columns = self.container(key, &layout)?;
        if let Some(span) = layout.geometry.span
            && span.get() > columns.get()
        {
            return Err(OverlayError::SpanExceedsColumns {
                key: key.to_string(),
                span: span.get(),
                container: named(layout.section.as_ref()),
                columns: columns.get(),
            });
        }
        let (stored, rm_type, anchor, placement) = self.place(key)?;
        self.overlay.put(OverlayEntry {
            key: stored,
            rm_type,
            anchor,
            layout,
        });
        Ok(placement)
    }

    /// Moves the entry at `from` onto the node `to` names, which is how a
    /// person accepts a move a replay suggested.
    ///
    /// A move is never applied by the replay itself. This is the one call that
    /// re-keys an entry, and it is the person's decision
    /// (`docs/architecture.md` section 6.5). What the person authored travels
    /// unchanged, geometry included, so a layout this call carries is not
    /// checked again: the node changed and the layout did not, and refusing
    /// the move would strand the entry on a key the definition no longer has.
    ///
    /// # Errors
    /// [`OverlayError::NoSuchNode`] when `from` decorates nothing or `to`
    /// names no node of the form. The entry stays where it is when the move is
    /// refused.
    pub fn accept_move(&mut self, from: &NodeKey, to: &NodeKey) -> Result<Placement, OverlayError> {
        let Some(entry) = self.overlay.entry(from).cloned() else {
            return Err(OverlayError::NoSuchNode {
                template: self.definition.template_id.to_string(),
                key: from.to_string(),
            });
        };
        let (stored, rm_type, anchor, placement) = self.place(to)?;
        if stored.steps != from.steps {
            self.overlay.discard(from);
        }
        self.overlay.put(OverlayEntry {
            key: stored,
            rm_type,
            anchor,
            layout: entry.layout,
        });
        Ok(placement)
    }

    /// How many columns the container of `layout` lays its items out on.
    ///
    /// The container is the authored section the item belongs to. An item in
    /// no authored section is in a one-column container, so it has no second
    /// column to span into (`docs/architecture.md` section 6.3).
    fn container(&self, key: &NodeKey, layout: &Layout) -> Result<ColumnCount, OverlayError> {
        let Some(id) = layout.section.as_ref() else {
            return Ok(ColumnCount::ONE);
        };
        self.overlay
            .section(id)
            .map(|section| section.columns)
            .ok_or_else(|| OverlayError::UnknownSection {
                referrer: format!("the entry at {key}"),
                section: id.to_string(),
            })
    }

    /// Where `key` lands in the definition, what the node collects there, and
    /// what a positional entry is anchored to.
    ///
    /// Positionality is recomputed here rather than read from
    /// `NodeKey::is_positional`, because resolving the key against the
    /// definition is required anyway to reject an entry that decorates no
    /// node, and the number of candidates that resolution returns IS the
    /// condition. The flag is the compiler's answer to the same question and
    /// agrees with this one (#67); this is the authority because it holds the
    /// definition.
    fn place(
        &self,
        key: &NodeKey,
    ) -> Result<(NodeKey, RmTypeName, Option<PositionalAnchor>, Placement), OverlayError> {
        let candidates = self.index.resolve(key);
        let missing = || OverlayError::NoSuchNode {
            template: self.definition.template_id.to_string(),
            key: key.to_string(),
        };
        let [only] = candidates.as_slice() else {
            let tied = candidates.len();
            if tied == 0 {
                return Err(missing());
            }
            let chosen = candidates
                .iter()
                .find(|target| same_steps(target.key(), key))
                .ok_or_else(missing)?;
            let mut stored = chosen.key().clone();
            stored.is_positional = true;
            let anchor = PositionalAnchor {
                tied_siblings: candidates
                    .iter()
                    .map(crate::index::Target::signature)
                    .collect(),
            };
            return Ok((
                stored,
                chosen.rm_type().clone(),
                Some(anchor),
                Placement::Positional { tied },
            ));
        };
        let mut stored = only.key().clone();
        stored.is_positional = false;
        Ok((stored, only.rm_type().clone(), None, Placement::Unique))
    }
}
