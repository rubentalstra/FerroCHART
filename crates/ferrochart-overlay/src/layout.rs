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

use ferrochart_form::key::NodeKey;
use ferrochart_form::text::Localized;
use ferrochart_form::value::Prefill;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[serde(tag = "visibility", rename_all = "snake_case")]
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
#[serde(tag = "test", rename_all = "snake_case")]
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

#[cfg(test)]
mod tests {
    use ferrochart_form::ids::{RmAttributeName, RmTypeName};
    use ferrochart_form::key::{KeyStep, NodeKey};

    use super::{Condition, Layout, Visibility};

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
        assert_eq!(read, ["/a", "/b", "/c"]);
    }
}
