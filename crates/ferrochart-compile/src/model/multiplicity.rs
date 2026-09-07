// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Counts: occurrences, existence and cardinality.

use std::fmt;

/// A count interval over the natural numbers.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.2 and `AOM2.html`
/// section 4.2.2 use one interval class for three different questions, and say
/// in near-identical prose that the three are distinct: existence says whether
/// an attribute slot is there at all, cardinality says how many members a
/// container holds, and occurrences says how many times one child node may
/// repeat. This type is the shape they share; the field that holds it says
/// which question it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Multiplicity {
    lower: u32,
    upper: Option<u32>,
}

impl Multiplicity {
    /// A closed interval from `lower` to `upper`.
    #[must_use]
    pub const fn bounded(lower: u32, upper: u32) -> Self {
        Self {
            lower,
            upper: Some(upper),
        }
    }

    /// An interval from `lower` with no upper bound.
    #[must_use]
    pub const fn unbounded_from(lower: u32) -> Self {
        Self { lower, upper: None }
    }

    /// The interval admitting exactly `n`.
    #[must_use]
    pub const fn exactly(n: u32) -> Self {
        Self::bounded(n, n)
    }

    /// The lower bound, which is inclusive.
    #[must_use]
    pub const fn lower(self) -> u32 {
        self.lower
    }

    /// The inclusive upper bound, or `None` when the interval is unbounded
    /// above.
    #[must_use]
    pub const fn upper(self) -> Option<u32> {
        self.upper
    }

    /// Whether the interval has no upper bound.
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        self.upper.is_none()
    }

    /// Whether at least one is required.
    #[must_use]
    pub const fn is_mandatory(self) -> bool {
        self.lower >= 1
    }

    /// Whether more than one is admitted, which is what makes a group
    /// repeatable.
    #[must_use]
    pub const fn admits_many(self) -> bool {
        match self.upper {
            None => true,
            Some(upper) => upper > 1,
        }
    }

    /// Whether the interval admits nothing at all, which is how a template
    /// removes a node (`occurrences matches {0}`).
    #[must_use]
    pub const fn is_prohibited(self) -> bool {
        matches!(self.upper, Some(0))
    }
}

impl fmt::Display for Multiplicity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.upper {
            None => write!(f, "{}..*", self.lower),
            Some(upper) if upper == self.lower => write!(f, "{upper}"),
            Some(upper) => write!(f, "{}..{upper}", self.lower),
        }
    }
}

/// How many members a container attribute holds, and how it behaves.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.2.2,
/// `C_MULTIPLE_ATTRIBUTE.cardinality`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cardinality {
    interval: Multiplicity,
    is_ordered: bool,
    is_unique: bool,
}

impl Cardinality {
    /// A cardinality over `interval` with the two container flags.
    #[must_use]
    pub const fn new(interval: Multiplicity, is_ordered: bool, is_unique: bool) -> Self {
        Self {
            interval,
            is_ordered,
            is_unique,
        }
    }

    /// How many members the container holds.
    #[must_use]
    pub const fn interval(self) -> Multiplicity {
        self.interval
    }

    /// Whether the order of the members carries meaning.
    #[must_use]
    pub const fn is_ordered(self) -> bool {
        self.is_ordered
    }

    /// Whether the members must differ from one another.
    #[must_use]
    pub const fn is_unique(self) -> bool {
        self.is_unique
    }
}

/// What the owning Reference Model attribute says about a node's slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeContext {
    existence: Option<Multiplicity>,
    cardinality: Option<Cardinality>,
    is_container: bool,
}

impl AttributeContext {
    /// A slot under a single-valued attribute.
    #[must_use]
    pub const fn single(existence: Option<Multiplicity>) -> Self {
        Self {
            existence,
            cardinality: None,
            is_container: false,
        }
    }

    /// A slot under a container attribute.
    #[must_use]
    pub const fn container(
        existence: Option<Multiplicity>,
        cardinality: Option<Cardinality>,
    ) -> Self {
        Self {
            existence,
            cardinality,
            is_container: true,
        }
    }

    /// Whether the attribute slot is there at all, where the template states
    /// it.
    #[must_use]
    pub const fn existence(self) -> Option<Multiplicity> {
        self.existence
    }

    /// The container's cardinality, where the attribute is a container and the
    /// template states one.
    #[must_use]
    pub const fn cardinality(self) -> Option<Cardinality> {
        self.cardinality
    }

    /// Whether the attribute holds many values.
    #[must_use]
    pub const fn is_container(self) -> bool {
        self.is_container
    }
}

/// A value range, as distinct from a count.
///
/// The bounds carry their own inclusivity because openEHR AM Release-2.3.0
/// `AOM1.4.html` sections 6.2.2 to 6.2.9 constrain a primitive with an
/// `Interval<T>`, whose ends may be open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_included: bool,
    upper_included: bool,
}

impl<T> Bounds<T> {
    /// A range with the given ends and inclusivity.
    ///
    /// An end the range does not have is not included, whatever the caller
    /// says: there is nothing there to include. Canonicalizing it here is what
    /// lets two templates that state the same range through different
    /// constrainer classes compare equal.
    #[must_use]
    pub fn new(
        lower: Option<T>,
        upper: Option<T>,
        lower_included: bool,
        upper_included: bool,
    ) -> Self {
        Self {
            lower_included: lower_included && lower.is_some(),
            upper_included: upper_included && upper.is_some(),
            lower,
            upper,
        }
    }

    /// The lower end, where the range has one.
    #[must_use]
    pub const fn lower(&self) -> Option<&T> {
        self.lower.as_ref()
    }

    /// The upper end, where the range has one.
    #[must_use]
    pub const fn upper(&self) -> Option<&T> {
        self.upper.as_ref()
    }

    /// Whether the lower end is itself admitted, which is false where the
    /// range has no lower end.
    #[must_use]
    pub const fn lower_included(&self) -> bool {
        self.lower_included
    }

    /// Whether the upper end is itself admitted, which is false where the
    /// range has no upper end.
    #[must_use]
    pub const fn upper_included(&self) -> bool {
        self.upper_included
    }

    /// Whether the range constrains nothing.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.lower.is_none() && self.upper.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::Multiplicity;

    #[test]
    fn a_zero_upper_bound_is_a_removed_node() {
        assert!(Multiplicity::exactly(0).is_prohibited());
        assert!(!Multiplicity::bounded(0, 1).is_prohibited());
    }

    #[test]
    fn an_upper_bound_above_one_makes_a_group_repeatable() {
        assert!(Multiplicity::unbounded_from(0).admits_many());
        assert!(Multiplicity::bounded(1, 4).admits_many());
        assert!(!Multiplicity::bounded(0, 1).admits_many());
        assert!(!Multiplicity::exactly(1).admits_many());
    }

    #[test]
    fn the_display_form_reads_as_the_adl_one() {
        assert_eq!(Multiplicity::unbounded_from(0).to_string(), "0..*");
        assert_eq!(Multiplicity::bounded(0, 1).to_string(), "0..1");
        assert_eq!(Multiplicity::exactly(1).to_string(), "1");
    }
}
