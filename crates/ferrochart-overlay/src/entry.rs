// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One decorated node: the key that names it, and what a person authored.

use ferrochart_form::ids::RmTypeName;
use ferrochart_form::key::NodeKey;
use ferrochart_form::layout::Layout;
use serde::{Deserialize, Serialize};

/// What a positionally keyed entry was anchored to when it was authored.
///
/// No specification governs this: our own design. Where the five-part step
/// tuple ties, the only thing left telling one sibling from another is its
/// position, and a position is trustworthy only while the siblings are the
/// ones the person saw. The anchor is the record of those siblings, so a
/// replay can tell an unchanged container from a reordered one instead of
/// trusting the ordinal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionalAnchor {
    /// One signature per sibling the key could not tell apart, in template
    /// order.
    ///
    /// A signature is the depth-annotated pre-order of the sibling's own
    /// subtree, which determines that subtree. Two siblings identical to this
    /// depth are interchangeable, so a swap between them changes nothing a
    /// layout can see.
    pub tied_siblings: Vec<String>,
}

/// One node of a form, and what a person authored about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayEntry {
    /// What names the node this entry decorates.
    ///
    /// The key is normalized when the entry is authored: `is_positional` says
    /// whether the definition told the node from its siblings by nothing but
    /// its ordinal, which is measured against the definition rather than
    /// guessed from the shape of the key.
    pub key: NodeKey,
    /// The Reference Model class the node collected when the layout was
    /// authored.
    ///
    /// The key cannot carry this. openEHR RM Release-1.1.0
    /// `data_structures.html` section 5.2.3 makes `ELEMENT` the leaf a
    /// `DATA_VALUE` is attached to, so the field a person lays out collects
    /// the value's class while its key step carries `ELEMENT`. A revision that
    /// changes the value's class leaves the key alone, and a widget chosen for
    /// a number means nothing on a coded field, so the class the layout was
    /// authored against is recorded and the replay compares it.
    pub rm_type: RmTypeName,
    /// The siblings the key's position was measured against, where the key is
    /// positional.
    ///
    /// Present exactly when [`NodeKey::is_positional`] is set.
    pub anchor: Option<PositionalAnchor>,
    /// What the person authored.
    pub layout: Layout,
}
