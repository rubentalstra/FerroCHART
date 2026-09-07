// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! How many times one item may appear.

use std::fmt;

use serde::{Deserialize, Serialize};

/// How many times a group or a field may appear.
///
/// This is the `C_OBJECT.occurrences` of the node the item was derived from
/// (openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.6), and the three
/// questions it answers are the three a renderer asks: an upper bound above
/// one makes the item repeatable, a lower bound of zero makes it optional, and
/// a lower bound of one or more makes it mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Occurrences {
    /// How many the template requires.
    pub minimum: u32,
    /// How many the template admits, or `None` where it sets no upper bound.
    pub maximum: Option<u32>,
}

impl Occurrences {
    /// A closed interval from `minimum` to `maximum`.
    #[must_use]
    pub const fn bounded(minimum: u32, maximum: u32) -> Self {
        Self {
            minimum,
            maximum: Some(maximum),
        }
    }

    /// An interval from `minimum` with no upper bound.
    #[must_use]
    pub const fn unbounded_from(minimum: u32) -> Self {
        Self {
            minimum,
            maximum: None,
        }
    }

    /// Whether the item must appear at least once.
    #[must_use]
    pub const fn is_mandatory(self) -> bool {
        self.minimum >= 1
    }

    /// Whether the item may be left out.
    #[must_use]
    pub const fn is_optional(self) -> bool {
        self.minimum == 0
    }

    /// Whether the item may appear more than once, which is what makes a group
    /// or a field repeatable.
    #[must_use]
    pub const fn is_repeatable(self) -> bool {
        match self.maximum {
            None => true,
            Some(maximum) => maximum > 1,
        }
    }
}

impl fmt::Display for Occurrences {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.maximum {
            None => write!(f, "{}..*", self.minimum),
            Some(maximum) => write!(f, "{}..{maximum}", self.minimum),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Occurrences;

    #[test]
    fn an_upper_bound_above_one_is_what_makes_an_item_repeatable() {
        assert!(Occurrences::unbounded_from(0).is_repeatable());
        assert!(Occurrences::bounded(1, 4).is_repeatable());
        assert!(!Occurrences::bounded(0, 1).is_repeatable());
        assert!(!Occurrences::bounded(1, 1).is_repeatable());
    }

    #[test]
    fn optional_and_mandatory_are_read_off_the_lower_bound() {
        assert!(Occurrences::bounded(0, 1).is_optional());
        assert!(!Occurrences::bounded(0, 1).is_mandatory());
        assert!(Occurrences::bounded(1, 1).is_mandatory());
        assert!(!Occurrences::bounded(1, 1).is_optional());
    }
}
