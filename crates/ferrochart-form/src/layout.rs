// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What a person authored, which no specification governs.
//!
//! openEHR publishes no form artefact. openEHR AM Release-2.3.0 `AOM2.html`
//! section 3.5.1 scopes archetype annotations to documentation, and
//! `OPT2.html` section 3.3 permits a generator to delete the annotations
//! section outright, so none of the places a template could carry layout is
//! governed by specification prose. Everything in this module is therefore
//! FerroCHART's own design.
//!
//! A layout decorates one node of a form definition and never replaces it.
//! The definition says what the template admits; the layout says how a person
//! wants it presented, so nothing here can make a form admit what the
//! template refuses.

use std::collections::BTreeMap;
use std::fmt;
use std::num::{NonZeroU8, NonZeroU16};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::TemplateId;
use crate::key::NodeKey;
use crate::text::Localized;
use crate::value::Prefill;
use crate::values::{Datum, Entered};

/// What names one authored section.
///
/// No specification governs this: our own design. A section is the person's
/// own grouping, so its identifier is theirs too and FerroCHART never mints
/// or rewrites one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SectionId(String);

impl SectionId {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The identifier as the person stated it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name a renderer knows one control by.
///
/// No specification governs this: our own design. FerroCHART never interprets
/// the name, because the set of controls belongs to the renderer rather than
/// to the form definition, and a closed list here would refuse a control a
/// third-party renderer has.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WidgetName(String);

impl WidgetName {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The name as the person stated it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WidgetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a stated geometry is not one this format admits.
///
/// No specification governs this: our own design. A column count and a span
/// are bounded, and zero of either is meaningless, so the bounds live in the
/// types and an out-of-range number is refused before it is ever stored.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// The column count is outside the permitted range.
    #[error("a section is 1 to {most} columns wide, and {stated} was stated")]
    ColumnCount {
        /// The number that was stated.
        stated: u8,
        /// The widest a section may be.
        most: u8,
    },
    /// The span is outside the permitted range.
    #[error("an item spans 1 to {most} columns, and {stated} was stated")]
    ColumnSpan {
        /// The number that was stated.
        stated: u8,
        /// The widest span there can be.
        most: u8,
    },
    /// The width hint is zero characters.
    #[error("a width hint is a positive number of characters, and 0 was stated")]
    Width,
}

/// How many columns a section lays its items out on.
///
/// No specification governs this: our own design. The range is 1 to 12, and
/// the store advises against anything above
/// [`ColumnCount::CROWDED_ABOVE`]: OpenClinica caps its own column count at
/// three and usability research finds multi-column forms raise skipped and
/// misinterpreted fields, so density is bounded rather than trusted
/// (`docs/architecture.md` section 6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct ColumnCount(NonZeroU8);

impl ColumnCount {
    /// One column, which is what a section that declares nothing is.
    pub const ONE: Self = Self(NonZeroU8::MIN);

    /// The widest a section may be.
    pub const MOST: u8 = 12;

    /// The widest a section goes before the store advises against it.
    pub const CROWDED_ABOVE: u8 = 4;

    /// The count as a number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0.get()
    }

    /// Whether a form author should be told the section is this wide.
    #[must_use]
    pub const fn is_crowded(self) -> bool {
        self.0.get() > Self::CROWDED_ABOVE
    }
}

impl TryFrom<u8> for ColumnCount {
    type Error = GeometryError;

    /// Reads `columns` as a column count.
    ///
    /// # Errors
    /// [`GeometryError::ColumnCount`] when the number is zero or above
    /// [`ColumnCount::MOST`].
    fn try_from(columns: u8) -> Result<Self, Self::Error> {
        let refuse = || GeometryError::ColumnCount {
            stated: columns,
            most: Self::MOST,
        };
        if columns > Self::MOST {
            return Err(refuse());
        }
        NonZeroU8::new(columns).map(Self).ok_or_else(refuse)
    }
}

impl From<ColumnCount> for u8 {
    fn from(count: ColumnCount) -> Self {
        count.get()
    }
}

impl fmt::Display for ColumnCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How many of its section's columns one item occupies.
///
/// No specification governs this: our own design. A span is what places an
/// item beside another, and it is the only width the overlay stores: there is
/// no row and no column index, so the position of an item is the sibling order
/// the overlay already carries, plus this span, plus
/// [`Geometry::break_before`] (`docs/architecture.md` section 6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct ColumnSpan(NonZeroU8);

impl ColumnSpan {
    /// One column.
    pub const ONE: Self = Self(NonZeroU8::MIN);

    /// The span as a number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0.get()
    }
}

impl TryFrom<u8> for ColumnSpan {
    type Error = GeometryError;

    /// Reads `columns` as a span.
    ///
    /// # Errors
    /// [`GeometryError::ColumnSpan`] when the number is zero or above
    /// [`ColumnCount::MOST`], which is the widest a section can be and
    /// therefore the widest a span can be.
    fn try_from(columns: u8) -> Result<Self, Self::Error> {
        let refuse = || GeometryError::ColumnSpan {
            stated: columns,
            most: ColumnCount::MOST,
        };
        if columns > ColumnCount::MOST {
            return Err(refuse());
        }
        NonZeroU8::new(columns).map(Self).ok_or_else(refuse)
    }
}

impl From<ColumnSpan> for u8 {
    fn from(span: ColumnSpan) -> Self {
        span.get()
    }
}

impl fmt::Display for ColumnSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How wide a control should look, in character units.
///
/// No specification governs this: our own design. The unit is characters
/// rather than pixels, because a field three characters wide should look three
/// characters wide at any zoom level and a pixel does not survive the screen
/// it was authored on (`docs/architecture.md` section 6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct CharacterWidth(NonZeroU16);

impl CharacterWidth {
    /// The width as a number of characters.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

impl TryFrom<u16> for CharacterWidth {
    type Error = GeometryError;

    /// Reads `characters` as a width hint.
    ///
    /// # Errors
    /// [`GeometryError::Width`] when the number is zero, because a control
    /// nought characters wide is not a hint about anything.
    fn try_from(characters: u16) -> Result<Self, Self::Error> {
        NonZeroU16::new(characters)
            .map(Self)
            .ok_or(GeometryError::Width)
    }
}

impl From<CharacterWidth> for u16 {
    fn from(width: CharacterWidth) -> Self {
        width.get()
    }
}

impl fmt::Display for CharacterWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The name a renderer knows one uninterpreted hint by.
///
/// No specification governs this: our own design. FerroCHART never interprets
/// the name or the value, so the set is open and belongs to the renderer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HintName(String);

impl HintName {
    /// Wraps `value` verbatim.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The name as the person stated it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HintName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where one item sits on its section's column grid.
///
/// No specification governs this: our own design. Nothing here stores a row or
/// a column index. An item's place is the sibling order the overlay already
/// carries, plus [`Geometry::span`], plus [`Geometry::break_before`], resolved
/// by the renderer's own auto-placement, so the visual order of a form equals
/// its source order by construction. That is what W3C WCAG 2.2 success
/// criterion 1.3.2 asks for
/// (<https://www.w3.org/TR/WCAG22/#meaningful-sequence>) and what a stored
/// coordinate breaks (`docs/architecture.md` section 6.3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Geometry {
    /// How many of the section's columns the item occupies.
    ///
    /// An item that states none takes the section's full width, so an author
    /// who asks for nothing gets a one-column form.
    pub span: Option<ColumnSpan>,
    /// Whether the item starts a row of its own.
    pub break_before: bool,
    /// How wide the control should look, in character units.
    pub width: Option<CharacterWidth>,
}

impl Geometry {
    /// The columns the item occupies in a section `columns` wide.
    ///
    /// A stated span wider than the section is clamped here rather than
    /// dropped from the overlay, which is how one authored form serves a
    /// ward-round tablet and a desk: a renderer narrows the grid, and nothing
    /// stored is discarded (`docs/architecture.md` section 6.3).
    #[must_use]
    pub fn span_in(self, columns: ColumnCount) -> ColumnSpan {
        self.span
            .filter(|span| span.0 <= columns.0)
            .unwrap_or(ColumnSpan(columns.0))
    }

    /// Whether this geometry decides nothing at all.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }
}

/// One grouping a person authored.
///
/// No specification governs this: our own design. The Reference Model tree is
/// the only grouping a template states, and a form author routinely wants
/// another one, so a section lives in the overlay and the items that belong
/// to it point at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    /// What names the section.
    pub id: SectionId,
    /// The section this one sits inside, where a person nested it.
    pub parent: Option<SectionId>,
    /// The heading the section is shown under.
    pub label: Localized,
    /// The longer text shown beneath the heading.
    pub help: Localized,
    /// Where the section sits among its siblings, smallest first.
    pub order: Option<u32>,
    /// How many columns the section lays its items out on.
    ///
    /// A section that declares nothing is [`ColumnCount::ONE`], so the form a
    /// person gets until they ask for otherwise is a single column.
    pub columns: ColumnCount,
}

impl Section {
    /// A top-level section with a heading and nothing else decided.
    #[must_use]
    pub fn new(id: SectionId, label: Localized) -> Self {
        Self {
            id,
            parent: None,
            label,
            help: Localized::empty(),
            order: None,
            columns: ColumnCount::ONE,
        }
    }

    /// The section, laid out on `columns` columns.
    #[must_use]
    pub fn on_columns(mut self, columns: ColumnCount) -> Self {
        self.columns = columns;
        self
    }
}

/// When an item is shown.
///
/// No specification governs this: our own design. ADL 1.4 section 8.5 and the
/// AOM 2 Rules package evaluate an assertion after entry rather than deciding
/// what a person sees, so a visibility rule is authored rather than derived.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Visibility {
    /// Always, which is what an item with no rule does.
    #[default]
    Always,
    /// Never: the item is entered by nobody and stays in the form so a
    /// composition builder still knows the node is there.
    Never,
    /// Only while the condition holds.
    When {
        /// The condition that decides it.
        condition: Condition,
    },
}

/// What decides whether an item is shown.
///
/// No specification governs this: our own design. The set is deliberately
/// small and closed, because a condition FerroCHART cannot evaluate is a
/// condition a renderer cannot honour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Condition {
    /// The node carries a value.
    Answered {
        /// The node whose answer decides it.
        node: NodeKey,
    },
    /// The node carries no value.
    NotAnswered {
        /// The node whose answer decides it.
        node: NodeKey,
    },
    /// The node carries exactly this value.
    Equals {
        /// The node whose answer decides it.
        node: NodeKey,
        /// The value the answer is compared against.
        value: Prefill,
    },
    /// The node carries anything but this value.
    NotEquals {
        /// The node whose answer decides it.
        node: NodeKey,
        /// The value the answer is compared against.
        value: Prefill,
    },
    /// Every one of these conditions holds.
    All {
        /// The conditions, in authored order.
        conditions: Vec<Condition>,
    },
    /// At least one of these conditions holds.
    Any {
        /// The conditions, in authored order.
        conditions: Vec<Condition>,
    },
}

impl Condition {
    /// Every node this condition reads, in the order the condition names them.
    ///
    /// A replay classifies these beside the entry itself, because a rule that
    /// reads a node the revision removed cannot be evaluated and the person
    /// has to be told.
    #[must_use]
    pub fn nodes(&self) -> Vec<&NodeKey> {
        let mut found = Vec::new();
        let mut pending = vec![self];
        while let Some(condition) = pending.pop() {
            match *condition {
                Self::Answered { ref node }
                | Self::NotAnswered { ref node }
                | Self::Equals { ref node, .. }
                | Self::NotEquals { ref node, .. } => found.push(node),
                Self::All { ref conditions } | Self::Any { ref conditions } => {
                    pending.extend(conditions.iter().rev());
                }
            }
        }
        found
    }
}

/// Whether an entered value is the one a condition names.
///
/// The comparison is by identity rather than by display: a code matches its
/// code, a magnitude matches its number, and a rubric a terminology server
/// happened to return is not part of the answer. A shape a condition cannot
/// name matches nothing, which is the safe direction: a rule nobody can
/// evaluate hides its item rather than showing it on a guess.
#[must_use]
pub fn matches(wanted: &Prefill, entered: &Datum) -> bool {
    match (wanted, entered) {
        (&Prefill::Boolean(wanted), &Datum::Boolean(entered)) => wanted == entered,
        (&Prefill::Integer(wanted), &Datum::Count(entered)) => wanted == entered,
        (
            Prefill::Coded { code: wanted, .. },
            Datum::Coded {
                terminology, code, ..
            },
        ) => wanted.terminology.as_str() == terminology && wanted.code == *code,
        (
            Prefill::Ordinal { symbol, .. },
            Datum::Ordinal {
                terminology, code, ..
            },
        ) => symbol.terminology.as_str() == terminology && symbol.code == *code,
        (&Prefill::Text(ref wanted), &Datum::Text(ref entered))
        | (
            &Prefill::Temporal(ref wanted),
            &Datum::Date(ref entered)
            | &Datum::Time(ref entered)
            | &Datum::DateTime(ref entered)
            | &Datum::Duration(ref entered),
        ) => wanted == entered,
        _ => false,
    }
}

/// How a condition reads the values a form holds.
///
/// A condition names a node, and a form holds that node once per occurrence of
/// every repeating group above it, so the reader answers for the address the
/// item being judged sits at rather than for the node in the abstract.
///
/// The answer is owned rather than borrowed, because a renderer holds its
/// values behind a signal that lends nothing across a read. A condition names
/// a handful of nodes, so the clone is bounded by the rule rather than by the
/// form.
pub trait Answers {
    /// What was entered at `node`, where anything was.
    fn entered(&self, node: &NodeKey) -> Option<Entered>;
}

impl Condition {
    /// Whether the condition holds over `answers`.
    ///
    /// No specification governs this: our own design. ADL 1.4 section 8.5 and
    /// the AOM 2 Rules package evaluate an assertion after entry rather than
    /// deciding what a person sees, so both the rule and its evaluation are
    /// ours.
    ///
    /// A null flavour is not an answer. openEHR RM Release-1.1.0
    /// `data_structures.html` section 5.2.3 makes a null flavour the record
    /// that there is no value, so a field carrying one is not answered and
    /// equals nothing.
    #[must_use]
    pub fn holds<A: Answers + ?Sized>(&self, answers: &A) -> bool {
        match *self {
            Self::Answered { ref node } => {
                matches!(answers.entered(node), Some(Entered::Value(_)))
            }
            Self::NotAnswered { ref node } => {
                !matches!(answers.entered(node), Some(Entered::Value(_)))
            }
            Self::Equals {
                ref node,
                ref value,
            } => match answers.entered(node) {
                Some(Entered::Value(ref datum)) => matches(value, datum),
                _ => false,
            },
            Self::NotEquals {
                ref node,
                ref value,
            } => match answers.entered(node) {
                Some(Entered::Value(ref datum)) => !matches(value, datum),
                // Nothing entered is not the value, so the rule holds.
                _ => true,
            },
            Self::All { ref conditions } => conditions.iter().all(|inner| inner.holds(answers)),
            Self::Any { ref conditions } => conditions.iter().any(|inner| inner.holds(answers)),
        }
    }
}

impl Visibility {
    /// Whether an item under this rule is shown, given `answers`.
    #[must_use]
    pub fn shows<A: Answers + ?Sized>(&self, answers: &A) -> bool {
        match *self {
            Self::Always => true,
            Self::Never => false,
            Self::When { ref condition } => condition.holds(answers),
        }
    }
}

/// What a person authored about one node of a form.
///
/// No specification governs any member of this record: it is the half of a
/// form the openEHR specifications leave open, and it is the work a template
/// revision would otherwise destroy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    /// Where the item sits among its siblings, smallest first.
    ///
    /// An item with no order keeps template order, which is the only order
    /// the definition has.
    pub order: Option<u32>,
    /// The authored section the item belongs to.
    pub section: Option<SectionId>,
    /// The label shown instead of the archetype rubric.
    pub label: Localized,
    /// The help text shown instead of the archetype description.
    pub help: Localized,
    /// The value the form starts with.
    ///
    /// The shape is the definition's own [`Prefill`], so an authored default
    /// is comparable with the one the template states rather than being a
    /// second value model.
    pub default: Option<Prefill>,
    /// When the item is shown.
    pub visibility: Visibility,
    /// The control the renderer is asked for.
    pub widget: Option<WidgetName>,
    /// Where the item sits on the column grid of its section.
    pub geometry: Geometry,
    /// Renderer hints FerroCHART never reads, in name order.
    ///
    /// The escape hatch for a form the column grid cannot express, on the
    /// precedent of the HL7 FHIR R4 `rendering-style` extension
    /// (<https://hl7.org/fhir/R4/extension-rendering-style.html>), where a
    /// renderer reads what it recognizes and ignores the rest. It is
    /// deliberately outside the clean-replay guarantee: FerroCHART cannot say
    /// what a hint it never interprets means after a template revision, and a
    /// hint is not portable between renderers, so a replay reports nothing
    /// about what is in here (`docs/architecture.md` section 6.3).
    pub hints: BTreeMap<HintName, String>,
}

impl Layout {
    /// A layout that decides nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this layout decides nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Every node this layout reads, in the order it names them.
    ///
    /// The layout of one item can depend on the answer to another, and that
    /// dependency is a key like any other, so a replay resolves it too.
    #[must_use]
    pub fn referenced_nodes(&self) -> Vec<&NodeKey> {
        match self.visibility {
            Visibility::Always | Visibility::Never => Vec::new(),
            Visibility::When { ref condition } => condition.nodes(),
        }
    }
}

/// The version of the layout document format this crate defines.
///
/// Its own number rather than [`crate::definition::FORMAT_VERSION`], because
/// the two documents change for different reasons: a form definition changes
/// when the derivation does, and a layout changes when what a person may
/// author does. What the number covers, and what is and is not a bump, is the
/// same contract that constant states.
pub const LAYOUT_FORMAT_VERSION: u32 = 1;

/// One item's layout, keyed by the node it decorates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutItem {
    /// What names the node this layout decorates.
    pub key: NodeKey,
    /// What the person authored for it.
    pub layout: Layout,
}

/// The layout a person authored over one form, as a renderer reads it.
///
/// No specification governs this: our own design. The form definition is a
/// projection of the operational template and carries no layout
/// (`crate::definition::FormDefinition`), so everything a person decided
/// travels beside it rather than inside it. A renderer that fetches no layout
/// draws the form in template order, which is what it did before there was
/// one.
///
/// It is a separate document rather than a definition the server has already
/// laid out, because the authoring surface has to show the template's own
/// label beside the one a person wrote over it. A definition with the
/// overrides baked in could not tell the two apart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormLayout {
    /// The version of the format this document is written in.
    pub format_version: u32,
    /// The template the layout was authored against.
    pub template_id: TemplateId,
    /// The sections a person grouped items into, in authored order.
    pub sections: Vec<Section>,
    /// One entry per node a person decided anything about, in key order.
    pub items: Vec<LayoutItem>,
}

impl FormLayout {
    /// An empty layout over `template_id`, which decides nothing.
    #[must_use]
    pub fn new(template_id: TemplateId) -> Self {
        Self {
            format_version: LAYOUT_FORMAT_VERSION,
            template_id,
            sections: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Whether this document is written in the format version this crate
    /// defines.
    #[must_use]
    pub const fn is_current_format(&self) -> bool {
        self.format_version == LAYOUT_FORMAT_VERSION
    }

    /// What the person authored for `key`, where they authored anything.
    #[must_use]
    pub fn of(&self, key: &NodeKey) -> Option<&Layout> {
        self.items
            .iter()
            .find(|item| item.key == *key)
            .map(|item| &item.layout)
    }

    /// When the item at `key` is shown.
    ///
    /// [`Visibility::Always`] where the person decided nothing, which is what
    /// a form with no layout does everywhere.
    #[must_use]
    pub fn visibility(&self, key: &NodeKey) -> &Visibility {
        self.of(key)
            .map_or(&Visibility::Always, |layout| &layout.visibility)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hints_serialize_in_name_order() {
        // `ferrochart-overlay` promises a byte-deterministic document, and the
        // one map in the format lives here, so the ordering claim is asserted
        // beside the type that has to keep it.
        let mut layout = Layout::new();
        for name in ["zebra", "alpha", "mid"] {
            layout.hints.insert(HintName::new(name), "value".to_owned());
        }
        let json = serde_json::to_string(&layout.hints).expect("hints serialize");
        assert_eq!(json, r#"{"alpha":"value","mid":"value","zebra":"value"}"#);
    }

    use super::{
        Answers, CharacterWidth, ColumnCount, ColumnSpan, Condition, FormLayout, Geometry,
        HintName, Layout, LayoutItem, Section, SectionId, Visibility,
    };
    use crate::ids::{RmAttributeName, RmTypeName, TemplateId};
    use crate::key::{KeyStep, NodeKey};
    use crate::text::Localized;
    use crate::value::{Code, Prefill};
    use crate::values::{Datum, Entered};

    fn columns(count: u8) -> ColumnCount {
        ColumnCount::try_from(count).unwrap()
    }

    fn span(count: u8) -> ColumnSpan {
        ColumnSpan::try_from(count).unwrap()
    }

    fn key(attribute: &str) -> NodeKey {
        NodeKey::root().child(KeyStep {
            rm_attribute: RmAttributeName::new(attribute),
            node_id: None,
            archetype_id: None,
            rm_type: RmTypeName::new("ELEMENT"),
            pinned_name: None,
            sibling_ordinal: 0,
        })
    }

    #[test]
    fn a_layout_that_decides_nothing_is_empty() {
        assert!(Layout::new().is_empty());
        let mut layout = Layout::new();
        layout.order = Some(3);
        assert!(!layout.is_empty());
    }

    #[test]
    fn a_nested_condition_names_its_nodes_in_the_order_it_reads_them() {
        let condition = Condition::All {
            conditions: vec![
                Condition::Answered { node: key("a") },
                Condition::Any {
                    conditions: vec![
                        Condition::NotAnswered { node: key("b") },
                        Condition::Answered { node: key("c") },
                    ],
                },
            ],
        };
        let mut layout = Layout::new();
        layout.visibility = Visibility::When { condition };
        let read: Vec<String> = layout
            .referenced_nodes()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(read, ["/a[ELEMENT]", "/b[ELEMENT]", "/c[ELEMENT]"]);
    }

    #[test]
    fn a_section_that_declares_nothing_is_one_column() {
        let section = Section::new(SectionId::new("counts"), Localized::empty());
        assert_eq!(section.columns, ColumnCount::ONE);
        assert_eq!(section.columns.get(), 1);
        assert!(!section.columns.is_crowded());
    }

    #[test]
    fn a_column_count_outside_one_to_twelve_is_not_representable() {
        assert!(ColumnCount::try_from(0).is_err());
        assert!(ColumnCount::try_from(13).is_err());
        assert_eq!(columns(1), ColumnCount::ONE);
        assert_eq!(columns(12).get(), 12);
        let refused = ColumnCount::try_from(13).unwrap_err();
        assert!(refused.to_string().contains("1 to 12 columns"), "{refused}");
        assert!(ColumnSpan::try_from(0).is_err());
        assert!(ColumnSpan::try_from(13).is_err());
        assert!(CharacterWidth::try_from(0).is_err());
    }

    #[test]
    fn an_item_that_asks_for_nothing_takes_its_sections_full_width() {
        let geometry = Geometry::default();
        assert!(geometry.is_empty());
        assert_eq!(geometry.span, None);
        assert_eq!(geometry.span_in(ColumnCount::ONE), ColumnSpan::ONE);
        assert_eq!(geometry.span_in(columns(4)).get(), 4);
    }

    #[test]
    fn a_stated_span_is_clamped_by_a_narrower_grid_and_nothing_stored_is_lost() {
        let geometry = Geometry {
            span: Some(span(6)),
            ..Geometry::default()
        };
        assert_eq!(geometry.span_in(columns(12)).get(), 6);
        assert_eq!(geometry.span_in(columns(3)).get(), 3);
        assert_eq!(
            geometry.span,
            Some(span(6)),
            "clamping is the renderer's, so nothing stored is discarded"
        );
    }
    /// The answers a form holds, for a test that needs no form.
    struct Held(std::collections::BTreeMap<NodeKey, Entered>);

    impl Answers for Held {
        fn entered(&self, node: &NodeKey) -> Option<Entered> {
            self.0.get(node).cloned()
        }
    }

    fn held(pairs: Vec<(NodeKey, Entered)>) -> Held {
        Held(pairs.into_iter().collect())
    }

    #[test]
    fn an_item_with_no_rule_is_always_shown() {
        assert!(Visibility::Always.shows(&held(vec![])));
        assert!(!Visibility::Never.shows(&held(vec![])));
    }

    #[test]
    fn a_rule_reading_an_unanswered_node_hides_its_item() {
        let deceased = key("deceased");
        let rule = Visibility::When {
            condition: Condition::Equals {
                node: deceased.clone(),
                value: Prefill::Boolean(true),
            },
        };
        assert!(!rule.shows(&held(vec![])));
        assert!(!rule.shows(&held(vec![(
            deceased.clone(),
            Entered::Value(Datum::Boolean(false)),
        )])));
        assert!(rule.shows(&held(vec![(
            deceased,
            Entered::Value(Datum::Boolean(true)),
        )])));
    }

    #[test]
    fn a_null_flavour_is_not_an_answer() {
        // openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 makes
        // a null flavour the record that there is no value, so a field
        // carrying one has not been answered.
        let node = key("asked");
        let null = Entered::Null {
            code: crate::ids::LocalCode::new("271"),
            reason: None,
        };
        let answered = Visibility::When {
            condition: Condition::Answered { node: node.clone() },
        };
        assert!(!answered.shows(&held(vec![(node.clone(), null.clone())])));
        let not_answered = Visibility::When {
            condition: Condition::NotAnswered { node: node.clone() },
        };
        assert!(not_answered.shows(&held(vec![(node, null)])));
    }

    #[test]
    fn a_value_of_another_shape_matches_nothing() {
        // A rule nobody can evaluate hides its item rather than showing it on
        // a guess.
        let node = key("count");
        let rule = Visibility::When {
            condition: Condition::Equals {
                node: node.clone(),
                value: Prefill::Boolean(true),
            },
        };
        assert!(!rule.shows(&held(vec![(node, Entered::Value(Datum::Count(1)))])));
    }

    #[test]
    fn a_code_matches_by_its_code_and_never_by_its_rubric() {
        let node = key("coded");
        let wanted = Prefill::Coded {
            code: Code::new(crate::ids::local_terminology(), "at0007"),
            rubric: Some("Yes".to_owned()),
        };
        let entered = |code: &str, rubric: &str| {
            Entered::Value(Datum::Coded {
                terminology: "local".to_owned(),
                code: code.to_owned(),
                rubric: rubric.to_owned(),
            })
        };
        let rule = Visibility::When {
            condition: Condition::Equals {
                node: node.clone(),
                value: wanted,
            },
        };
        // A terminology server may return another rubric for the same code.
        assert!(rule.shows(&held(vec![(node.clone(), entered("at0007", "Ja"))])));
        assert!(!rule.shows(&held(vec![(node, entered("at0008", "Yes"))])));
    }

    #[test]
    fn all_and_any_read_the_way_they_are_named() {
        let one = key("one");
        let two = key("two");
        let both = vec![
            Condition::Answered { node: one.clone() },
            Condition::Answered { node: two.clone() },
        ];
        let only_one = held(vec![(one, Entered::Value(Datum::Count(1)))]);
        assert!(
            !Condition::All {
                conditions: both.clone()
            }
            .holds(&only_one)
        );
        assert!(Condition::Any { conditions: both }.holds(&only_one));
    }

    #[test]
    fn a_layout_that_says_nothing_about_a_node_shows_it() {
        let layout = FormLayout::new(TemplateId::new("x"));
        assert_eq!(layout.visibility(&key("anything")), &Visibility::Always);
        assert!(layout.is_current_format());
    }

    #[test]
    fn a_layout_document_round_trips() {
        let mut layout = FormLayout::new(TemplateId::new("vital_signs.v1"));
        let node = key("systolic");
        let mut item = Layout::new();
        item.visibility = Visibility::When {
            condition: Condition::Answered { node: node.clone() },
        };
        layout.items.push(LayoutItem {
            key: node.clone(),
            layout: item,
        });
        let json = serde_json::to_string(&layout).expect("serialises");
        let read: FormLayout = serde_json::from_str(&json).expect("reads back");
        assert_eq!(read, layout);
        assert!(matches!(read.visibility(&node), &Visibility::When { .. }));
    }
}
