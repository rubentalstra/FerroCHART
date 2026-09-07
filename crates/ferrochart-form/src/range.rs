// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A range of values, as distinct from a count of occurrences.

use serde::{Deserialize, Serialize};

/// The values one field admits, with each end's inclusivity.
///
/// The ends carry their own inclusivity because openEHR AM Release-2.3.0
/// `AOM1.4.html` section 6.2.4 constrains an integer with a `range` of type
/// `Interval<Integer>`, whose ends may be open, and sections 6.2.5 to 6.2.9
/// do the same for the other primitives. An end the range does not have is
/// never included: there is nothing there to include.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range<T> {
    /// The lower end, where the template states one.
    pub minimum: Option<T>,
    /// The upper end, where the template states one.
    pub maximum: Option<T>,
    /// Whether the lower end is itself admitted.
    pub minimum_included: bool,
    /// Whether the upper end is itself admitted.
    pub maximum_included: bool,
}

impl<T> Range<T> {
    /// A range with the given ends and inclusivity.
    ///
    /// An end the range does not have is recorded as not included, whatever
    /// the caller says, so two templates that state the same range through
    /// different constrainer classes compare equal.
    #[must_use]
    pub fn new(
        minimum: Option<T>,
        maximum: Option<T>,
        minimum_included: bool,
        maximum_included: bool,
    ) -> Self {
        Self {
            minimum_included: minimum_included && minimum.is_some(),
            maximum_included: maximum_included && maximum.is_some(),
            minimum,
            maximum,
        }
    }

    /// Whether the range constrains nothing.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.minimum.is_none() && self.maximum.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::Range;

    #[test]
    fn an_absent_end_is_never_included() {
        let range = Range::new(Some(0_i64), None, true, true);
        assert!(range.minimum_included);
        assert!(!range.maximum_included);
        assert!(!range.is_open());
    }
}
