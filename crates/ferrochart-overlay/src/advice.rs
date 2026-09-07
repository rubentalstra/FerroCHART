// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What the overlay stores and still tells a person about.
//!
//! No specification governs this: our own design. An advisory is the third
//! answer beside storing a value and refusing it: the overlay keeps what a
//! person authored, and says what is wrong with it. Nothing here is a refusal,
//! because every one of these states is reached by an edit a person is allowed
//! to make, and discarding their work to keep the document tidy is the loss the
//! overlay exists to prevent (`docs/architecture.md` section 6.1).

use std::fmt;

use ferrochart_form::key::NodeKey;

use crate::layout::{ColumnCount, ColumnSpan, SectionId};

/// Something the overlay stores and a person should still see.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Advisory {
    /// The section lays its items out on more columns than a form should use.
    ///
    /// Accepted rather than refused, because the bound where density starts to
    /// hurt is not the bound where a layout stops making sense: OpenClinica
    /// caps its own column count at three, and usability research finds
    /// multi-column forms raise skipped and misinterpreted fields
    /// (`docs/architecture.md` section 6.3).
    CrowdedSection {
        /// The section that is this wide.
        section: SectionId,
        /// How many columns it declares.
        columns: ColumnCount,
    },
    /// The entry names a section the overlay no longer defines.
    ///
    /// The reference is retained, so restoring the section restores the
    /// grouping.
    SectionDisappeared {
        /// The entry that names it.
        key: NodeKey,
        /// The section that resolves to nothing.
        section: SectionId,
    },
    /// The entry spans more columns than the section it sits in now has.
    ///
    /// A renderer clamps the span, so nothing is lost and nothing is
    /// misplaced, but the layout is no longer the one the person authored.
    SpanOverflows {
        /// The entry that spans too far.
        key: NodeKey,
        /// The span the person authored.
        span: ColumnSpan,
        /// The section the entry sits in.
        section: SectionId,
        /// How many columns that section declares now.
        columns: ColumnCount,
    },
}

impl Advisory {
    /// The entry this is about, where it is about one entry.
    #[must_use]
    pub const fn key(&self) -> Option<&NodeKey> {
        match *self {
            Self::CrowdedSection { .. } => None,
            Self::SectionDisappeared { ref key, .. } | Self::SpanOverflows { ref key, .. } => {
                Some(key)
            }
        }
    }

    /// The section this is about.
    #[must_use]
    pub const fn section(&self) -> &SectionId {
        match *self {
            Self::CrowdedSection { ref section, .. }
            | Self::SectionDisappeared { ref section, .. }
            | Self::SpanOverflows { ref section, .. } => section,
        }
    }
}

impl fmt::Display for Advisory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CrowdedSection {
                ref section,
                columns,
            } => write!(
                f,
                "section \"{section}\" is {columns} columns wide, and a form more than \
                 {} columns wide raises skipped and misinterpreted fields",
                ColumnCount::CROWDED_ABOVE
            ),
            Self::SectionDisappeared {
                ref key,
                ref section,
            } => write!(
                f,
                "the entry at {key} belongs to section \"{section}\", which the overlay \
                 no longer defines"
            ),
            Self::SpanOverflows {
                ref key,
                span,
                ref section,
                columns,
            } => write!(
                f,
                "the entry at {key} spans {span} columns, and section \"{section}\" is \
                 {columns} wide"
            ),
        }
    }
}
