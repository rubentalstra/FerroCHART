// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One thing a clinician enters, and everything the template decided about it.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ids::{RmTypeName, TerminologyName};
use crate::key::NodeKey;
use crate::occurrences::Occurrences;
use crate::range::Range;
use crate::text::Localized;
use crate::value::{BindingStrictness, Code, Prefill, ValueSet};

/// One entry field of a form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent fact the template states, and this is a published record a renderer reads field by field"
)]
pub struct FormField {
    /// What names the field.
    pub key: NodeKey,
    /// The Reference Model class the field collects.
    ///
    /// A field derived from an `ELEMENT` collects the class of the data value
    /// the element holds, so this is `DV_QUANTITY` where the node itself is an
    /// `ELEMENT` (openEHR RM Release-1.1.0 `data_structures.html` section
    /// 5.2.3). The `rm_type` of a [`crate::key::KeyStep`] is the other
    /// notion, the class the node IS, and the two differ on every such field.
    pub rm_type: RmTypeName,
    /// The text the field is labelled with.
    pub label: Localized,
    /// The longer text shown beside the field.
    pub help: Localized,
    /// How many times the field may appear.
    pub occurrences: Occurrences,
    /// What the field collects.
    pub kind: FieldKind,
    /// The constraint on the node's Reference Model name, where the template
    /// leaves the name open.
    pub name_constraint: Option<NameConstraint>,
    /// The reference bands shown beside the value, where the template states
    /// any.
    ///
    /// `None` where it states none, and `None` on a field whose kind is a
    /// [`ChoiceField`]: each alternative of a choice is a data value of its
    /// own, so its bands sit on [`ChoiceAlternative::reference_ranges`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_ranges: Option<Box<ReferenceRanges>>,
    /// Whether the order of the field's repeats carries meaning.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.5,
    /// `CARDINALITY.is_ordered`, of the attribute the field sits under.
    pub is_ordered: bool,
    /// Whether the field's repeats must differ from one another.
    pub is_unique: bool,
    /// The null-flavour affordance beside the field.
    pub null_flavour: NullFlavour,
    /// The value the template prefills the field with.
    pub prefill: Option<Prefill>,
    /// Whether the archetype marks the node deprecated.
    ///
    /// openEHR AM Release-2.3.0 `AOM2.html` section 4.5.2,
    /// `C_OBJECT.is_deprecated`. ADL 1.4 has no way to say this, so a field
    /// derived from an ADL 1.4 template is never deprecated.
    pub is_deprecated: bool,
    /// Whether the constraint admits exactly one value, so the template has
    /// already decided it and nothing is entered.
    pub is_fixed: bool,
}

/// The constraint a template puts on a node's Reference Model name.
///
/// openEHR RM Release-1.1.0 `common.html` gives every `LOCATABLE` a mandatory
/// `name` of type `DV_TEXT`. Where the template pins a single name, that name
/// is the node's identity and appears in its [`NodeKey`]; where the template
/// admits several, the name is a second thing to decide. No specification
/// governs whether that decision belongs to the clinician, so FerroCHART
/// records the constraint beside the item rather than as an item of its own: a
/// renderer that offers a name selector has what it needs, and one that does
/// not still renders a complete form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NameConstraint {
    /// What names the constraint, so a composition builder knows where the
    /// chosen name belongs.
    pub key: NodeKey,
    /// The Reference Model class the name is stated as.
    pub rm_type: RmTypeName,
    /// What the name collects.
    pub kind: FieldKind,
}

/// The reference bands a template states beside a value, for a reader rather
/// than for entry.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 gives every
/// `DV_ORDERED` an optional `normal_status`, `normal_range` and
/// `other_reference_ranges`, and its `is_simple()` function is "True if this
/// quantity has no reference ranges". None of the three is entered: a
/// laboratory sends them with a result so a reader can tell the result from
/// the band it is read against.
///
/// A renderer tells them from an entry field by where they sit. They are
/// reachable only through [`FormField::reference_ranges`] and
/// [`ChoiceAlternative::reference_ranges`], never through
/// [`FormField::kind`] and never as an item of a group, and this type carries
/// no [`NodeKey`], no [`Occurrences`] and no [`NullFlavour`], so nothing in
/// it can be filled in or submitted. Reading a Reference Model class name
/// decides nothing here.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReferenceRanges {
    /// The normal-status indicator, where the template constrains it.
    ///
    /// `DV_ORDERED.normal_status` is a `CODE_PHRASE`, "coded by ordinals in
    /// series HHH, HH, H, (nothing), L, LL, LLL", which the invariant
    /// `Normal_status_validity` draws from the openEHR `normal_status` code
    /// set. It is a coded value, so it carries the value set of any coded
    /// field.
    pub normal_status: Option<CodedField>,
    /// The normal range, where the template constrains it.
    ///
    /// `DV_ORDERED.normal_range` is a `DV_INTERVAL` over the class the value
    /// itself collects, so it carries the same interval shape an entry field
    /// of that class would (`data_types.html` section 6.2.2).
    pub normal_range: Option<IntervalField>,
    /// Every other reference range, in the order the template states them.
    ///
    /// `DV_ORDERED.other_reference_ranges` is "optional tagged other
    /// reference ranges for this value in its particular measurement
    /// context", typed `List<REFERENCE_RANGE>`, and a list has an order, so
    /// template order is kept and nothing is sorted by meaning.
    ///
    /// Empty where the template states none. The invariant
    /// `Other_reference_ranges_validity` reads
    /// `other_reference_ranges /= Void implies not
    /// other_reference_ranges.is_empty`, so an empty list is not a state the
    /// Reference Model admits and an empty collection cannot be mistaken for
    /// one; wrapping it in an [`Option<Vec<ReferenceRange>>`] would spell
    /// absence twice.
    pub other: Vec<ReferenceRange>,
}

impl ReferenceRanges {
    /// Whether the template stated no band at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.normal_status.is_none() && self.normal_range.is_none() && self.other.is_empty()
    }
}

/// One named band shown beside a value.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.3 defines
/// `REFERENCE_RANGE<T>` as "a named range to be associated with any
/// `DV_ORDERED` datum", and gives it a `meaning` and a `range`. Both are
/// mandatory in data and either may be left unconstrained by a template,
/// which is what `None` records here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceRange {
    /// What the band means, where the template constrains it.
    ///
    /// `REFERENCE_RANGE.meaning` is a `DV_TEXT` "whose value indicates the
    /// meaning of this range, e.g. normal, critical, therapeutic", so this is
    /// a [`FieldKind::Text`], or a [`FieldKind::Coded`] where the template
    /// states the meaning as the `DV_CODED_TEXT` subtype.
    pub meaning: Option<FieldKind>,
    /// The band itself, where the template constrains it.
    ///
    /// `REFERENCE_RANGE.range` is "the data range for this meaning", a
    /// `DV_INTERVAL` over the class the value collects.
    pub range: Option<IntervalField>,
}

/// The null-flavour affordance beside a field.
///
/// openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 makes the
/// `ELEMENT` invariants the whole null mechanism: `Inv_is_null_valid`
/// (`is_null() = (value = Void)`), `Inv_null_flavour_indicated`
/// (`is_null() xor null_flavour = Void`) and `Inv_null_reason_valid`
/// (`null_reason /= Void implies is_null()`). Read as a form rule, a field has
/// either a value or a null flavour, never both and never neither.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullFlavour {
    /// Whether the form offers a null flavour instead of a value.
    pub is_offered: bool,
    /// The flavours the form offers, in the order the openEHR terminology
    /// lists them.
    pub options: Vec<NullFlavourOption>,
    /// Whether the form offers free text for `ELEMENT.null_reason`, which is
    /// legal only once a null flavour is set.
    pub accepts_reason: bool,
}

impl NullFlavour {
    /// The affordance a field that carries no null mechanism at all has.
    #[must_use]
    pub fn absent() -> Self {
        Self {
            is_offered: false,
            options: Vec::new(),
            accepts_reason: false,
        }
    }

    /// The affordance beside an `ELEMENT` the template permits to be omitted.
    #[must_use]
    pub fn offered() -> Self {
        Self {
            is_offered: true,
            options: null_flavour_options(),
            accepts_reason: true,
        }
    }
}

/// One null flavour a clinician may choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullFlavourOption {
    /// What the composition stores.
    pub code: Code,
    /// The rubric openEHR RM Release-1.1.0 `data_structures.html` section 4.1
    /// prints beside the code.
    pub rubric: String,
}

/// Whether a component of a temporal value must, may or must not be filled.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` sections 6.2.6 to 6.2.9 spell a
/// temporal pattern with the component letter for a mandatory component, `?`
/// for an optional one and `X` for one that is not allowed: "`YYYY-??-??`
/// (date with optional month and day)" and "`HH:??:xx` (time with optional
/// minutes and seconds not allowed)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ComponentValidity {
    /// The component must be filled.
    Mandatory,
    /// The component may be filled.
    Optional,
    /// The component must not be filled.
    Prohibited,
}

/// What a field collects.
///
/// One variant per row of the derivation table, so a renderer chooses its
/// control from the variant and never from the Reference Model class name.
/// The variant names the kind of value and never the widget: which control a
/// four-member selection gets is a layout decision no specification governs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FieldKind {
    /// `DV_BOOLEAN` (openEHR RM Release-1.1.0 `data_types.html` section
    /// 4.2.2).
    Boolean(BooleanField),
    /// `DV_TEXT` (`data_types.html` section 5.2.1).
    Text(TextField),
    /// `DV_URI` and `DV_EHR_URI` (`data_types.html` sections 10.3.1 and
    /// 10.3.2).
    Uri(UriField),
    /// `DV_CODED_TEXT` and `CODE_PHRASE` (`data_types.html` sections 5.2.4
    /// and 5.2.3).
    Coded(CodedField),
    /// `DV_ORDINAL` and `DV_SCALE` (`data_types.html` sections 6.2.4 and
    /// 6.2.5).
    Ordinal(OrdinalField),
    /// `DV_COUNT` (`data_types.html` section 6.2.9).
    Count(CountField),
    /// `DV_QUANTITY` (`data_types.html` section 6.2.8).
    Quantity(QuantityField),
    /// `DV_PROPORTION` (`data_types.html` section 6.2.10).
    Proportion(ProportionField),
    /// `DV_DATE` (`data_types.html` section 7.2.2).
    Date(DateField),
    /// `DV_TIME` (`data_types.html` section 7.2.3).
    Time(TimeField),
    /// `DV_DATE_TIME` (`data_types.html` section 7.2.4).
    DateTime(DateTimeField),
    /// `DV_DURATION` (`data_types.html` section 7.2.5).
    Duration(DurationField),
    /// `DV_IDENTIFIER` (`data_types.html` section 4.2.4).
    Identifier(IdentifierField),
    /// `DV_MULTIMEDIA` (`data_types.html` section 9.2.2).
    Multimedia(MultimediaField),
    /// `DV_PARSABLE` (`data_types.html` section 9.2.3).
    Parsable(ParsableField),
    /// `DV_INTERVAL<T>` (`data_types.html` section 6.2.2).
    Interval(Box<IntervalField>),
    /// `DV_STATE` (`data_types.html` section 4.2.3).
    State(StateField),
    /// More than one alternative under one single-valued attribute, of which
    /// the clinician fills exactly one.
    Choice(ChoiceField),
}

impl FieldKind {
    /// A short, stable name for the kind, for a report a person reads.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match *self {
            Self::Boolean(_) => "boolean",
            Self::Text(_) => "text",
            Self::Uri(_) => "uri",
            Self::Coded(_) => "coded",
            Self::Ordinal(_) => "ordinal",
            Self::Count(_) => "count",
            Self::Quantity(_) => "quantity",
            Self::Proportion(_) => "proportion",
            Self::Date(_) => "date",
            Self::Time(_) => "time",
            Self::DateTime(_) => "date-time",
            Self::Duration(_) => "duration",
            Self::Identifier(_) => "identifier",
            Self::Multimedia(_) => "multimedia",
            Self::Parsable(_) => "parsable",
            Self::Interval(_) => "interval",
            Self::State(_) => "state",
            Self::Choice(_) => "choice",
        }
    }
}

/// A two-way control over `DV_BOOLEAN.value`.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.2 constrains a boolean
/// with `true_valid` and `false_valid`; where only one of the two is admitted
/// the template has fixed the value and nothing is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BooleanField {
    /// Whether `true` is admitted.
    pub true_allowed: bool,
    /// Whether `false` is admitted.
    pub false_allowed: bool,
}

/// Free text, a mask, or a closed pick list of plain strings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextField {
    /// The regular expressions the value may match, in the order the template
    /// lists them, each without the delimiters ADL 2 writes around one.
    pub patterns: Vec<String>,
    /// The values the template enumerates, in the order it lists them.
    pub options: Vec<String>,
    /// Whether the enumeration admits nothing outside itself.
    ///
    /// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.3 defines
    /// `C_STRING.list_open` as "True if the list is being used to specify the
    /// constraint but is not considered exhaustive", so a list the template
    /// states without that flag is exhaustive.
    pub options_closed: bool,
}

/// A URI, optionally restricted to one scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UriField {
    /// The regular expressions the value may match.
    pub patterns: Vec<String>,
    /// The scheme the Reference Model class itself requires.
    ///
    /// openEHR RM Release-1.1.0 `data_types.html` section 10.3.2 invariant
    /// `Scheme_valid` requires the `ehr` scheme of every `DV_EHR_URI`; a plain
    /// `DV_URI` takes any scheme.
    pub required_scheme: Option<String>,
}

/// A selection over a value set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodedField {
    /// Where the permitted codes come from.
    pub value_set: ValueSet,
    /// How strictly the set binds, where the template says.
    pub strictness: Option<BindingStrictness>,
    /// The constraint on the rubric `DV_CODED_TEXT` inherits from `DV_TEXT`,
    /// where the template states one.
    pub rubric: Option<TextField>,
}

/// One option of an ordinal or a scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrdinalOption {
    /// The score.
    pub score: f64,
    /// The symbol that carries the score, which is what a composition stores.
    pub symbol: Code,
    /// The rubric the option is shown under.
    pub label: Localized,
    /// The longer rubric, where the template carries one.
    pub description: Localized,
}

/// An ordered list of scored symbols.
///
/// openEHR RM Release-1.1.0 `data_types.html` sections 6.2.4 and 6.2.5. List
/// order is display order, because the specification gives an ordered list and
/// nothing else to order by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrdinalField {
    /// The options, in the order the template lists them.
    pub options: Vec<OrdinalOption>,
}

/// A whole number.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CountField {
    /// The values the template enumerates, in the order it lists them.
    pub options: Vec<i64>,
    /// Every range the template admits.
    pub ranges: Vec<Range<i64>>,
}

/// A real number.
///
/// This is not a field kind of its own: no Reference Model data type collects
/// a bare real. It is the shape a real-valued attribute of one takes, such as
/// `DV_PROPORTION.numerator` (openEHR RM Release-1.1.0 `data_types.html`
/// section 6.2.10).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RealField {
    /// The values the template enumerates, in the order it lists them.
    pub options: Vec<f64>,
    /// Every range the template admits.
    pub ranges: Vec<Range<f64>>,
}

/// One permitted unit of a quantity.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.8: each permitted
/// unit carries its own magnitude interval and its own decimal precision, so
/// changing the unit changes both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantityUnitOption {
    /// The unit symbol.
    pub units: String,
    /// The magnitudes this unit admits.
    pub magnitude: Option<Range<f64>>,
    /// The decimal places this unit admits.
    pub decimals: Option<Range<i64>>,
}

/// A magnitude with a unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantityField {
    /// The property the magnitude measures, where the template names one.
    pub property: Option<Code>,
    /// The permitted units, in the order the template lists them.
    ///
    /// Empty where the template names no unit at all, which admits any unit
    /// of the property. No unit is marked preferred: the template gives an
    /// ordered list and says nothing about preference.
    pub units: Vec<QuantityUnitOption>,
}

/// What a proportion's numerator and denominator mean.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.11,
/// `PROPORTION_KIND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProportionKind {
    /// `pk_ratio = 0`: "Numerator and denominator may be any value."
    Ratio,
    /// `pk_unitary = 1`: "Denominator must be 1."
    Unitary,
    /// `pk_percent = 2`: "Denominator is 100, numerator is understood as a
    /// percentage value."
    Percent,
    /// `pk_fraction = 3`: numerator and denominator are integral, presented
    /// with a slash.
    Fraction,
    /// `pk_integer_fraction = 4`: as a fraction, and a numerator above the
    /// denominator is presented as a mixed number.
    IntegerFraction,
    /// A kind outside the five the specification names, kept as the integer
    /// the template states rather than dropped.
    Other(i64),
}

impl ProportionKind {
    /// The kind the integer names, per openEHR RM Release-1.1.0
    /// `data_types.html` section 6.2.11.
    #[must_use]
    pub const fn from_code(code: i64) -> Self {
        match code {
            0 => Self::Ratio,
            1 => Self::Unitary,
            2 => Self::Percent,
            3 => Self::Fraction,
            4 => Self::IntegerFraction,
            other => Self::Other(other),
        }
    }

    /// The integer the kind is, per openEHR RM Release-1.1.0
    /// `data_types.html` section 6.2.11.
    ///
    /// The inverse of [`ProportionKind::from_code`], so a consumer that has
    /// to write `DV_PROPORTION.type` back out does not restate the table.
    #[must_use]
    pub const fn code(self) -> i64 {
        match self {
            Self::Ratio => 0,
            Self::Unitary => 1,
            Self::Percent => 2,
            Self::Fraction => 3,
            Self::IntegerFraction => 4,
            Self::Other(other) => other,
        }
    }
}

/// A numerator over a denominator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProportionField {
    /// The kinds the template admits, in the order it lists them.
    ///
    /// Empty where the template constrains `type` not at all, which admits
    /// every kind. The kind decides how many numbers the form collects:
    /// `pk_percent` and `pk_unitary` fix the denominator, so one number is
    /// entered, and the other three collect both.
    pub kinds: Vec<ProportionKind>,
    /// The numerators the template admits.
    pub numerator: RealField,
    /// The denominators the template admits.
    pub denominator: RealField,
    /// The decimal places the template admits.
    pub decimals: CountField,
    /// Whether both parts must be whole numbers, where the template says.
    pub is_integral: Option<bool>,
}

/// A calendar date, at the precision its pattern allows.
///
/// The year is always collected: openEHR RM Release-1.1.0 `data_types.html`
/// section 7.1.2.2 builds a partial date by dropping components from the
/// right, so a date without a year is not a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateField {
    /// Whether the month must, may or must not be filled.
    pub month: ComponentValidity,
    /// Whether the day must, may or must not be filled.
    pub day: ComponentValidity,
    /// Every range the template admits, with each end as the template spells
    /// it.
    pub ranges: Vec<Range<String>>,
}

/// A time of day, at the precision its pattern allows.
///
/// The hour is always collected, for the reason the year always is on a date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeField {
    /// Whether the minute must, may or must not be filled.
    pub minute: ComponentValidity,
    /// Whether the second must, may or must not be filled.
    pub second: ComponentValidity,
    /// Whether a timezone must, may or must not be filled.
    pub timezone: ComponentValidity,
    /// Every range the template admits, with each end as the template spells
    /// it.
    pub ranges: Vec<Range<String>>,
}

/// A date and a time, at the precision its pattern allows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateTimeField {
    /// Whether the month must, may or must not be filled.
    pub month: ComponentValidity,
    /// Whether the day must, may or must not be filled.
    pub day: ComponentValidity,
    /// Whether the hour must, may or must not be filled.
    pub hour: ComponentValidity,
    /// Whether the minute must, may or must not be filled.
    pub minute: ComponentValidity,
    /// Whether the second must, may or must not be filled.
    pub second: ComponentValidity,
    /// Whether a timezone must, may or must not be filled.
    pub timezone: ComponentValidity,
    /// Every range the template admits, with each end as the template spells
    /// it.
    pub ranges: Vec<Range<String>>,
}

/// One ISO 8601 duration slot.
///
/// openEHR AM Release-2.3.0 `AOM1.4.html` section 6.2.9 gives the permitted
/// duration patterns as `P[Y|y][M|m][D|d][T[H|h][M|m][S|s]]` and `P[W|w]`, and
/// a designator the pattern omits is a slot the value may not fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DurationComponent {
    /// The `Y` designator.
    Years,
    /// The `M` designator before the `T`.
    Months,
    /// The `W` designator.
    Weeks,
    /// The `D` designator.
    Days,
    /// The `H` designator.
    Hours,
    /// The `M` designator after the `T`.
    Minutes,
    /// The `S` designator.
    Seconds,
}

/// A length of time.
///
/// A negative duration is legal (openEHR RM Release-1.1.0 `data_types.html`
/// section 7.1.2.1), so only [`DurationField::ranges`] restricts the sign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationField {
    /// The slots the value may fill, in ISO 8601 order.
    pub components: BTreeSet<DurationComponent>,
    /// Every range the template admits, with each end as the template spells
    /// it.
    pub ranges: Vec<Range<String>>,
}

/// The four parts of an identifier.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 4.2.4.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IdentifierField {
    /// The organisation that issued the identifier.
    pub issuer: TextField,
    /// The organisation that assigned it.
    pub assigner: TextField,
    /// The identifier itself.
    pub id: TextField,
    /// What kind of identifier it is.
    #[serde(rename = "type")]
    pub identifier_type: TextField,
}

/// A file, given either by reference or inline.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 9.2.2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimediaField {
    /// The media types the template accepts, which is the file input's filter.
    pub media_types: ValueSet,
    /// The constraint on the reference, where the template states one.
    pub uri: Option<TextField>,
}

/// Text in a named formalism.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParsableField {
    /// The formalisms the template admits.
    pub formalism: TextField,
    /// The text itself.
    pub value: TextField,
}

/// Two ends of the same field.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 6.2.2: a `DV_INTERVAL`
/// carries a lower and an upper value of the same type, plus the inclusivity
/// and unbounded flags a form collects beside them. Neither ADL generation has
/// a dedicated constrainer, so each end is constrained by the constrainer of
/// the element type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalField {
    /// The Reference Model class both ends collect.
    pub element_rm_type: RmTypeName,
    /// The lower end.
    pub lower: FieldKind,
    /// The upper end.
    pub upper: FieldKind,
    /// Whether the lower end is itself admitted, where the template fixes it;
    /// `None` means the form collects the flag.
    pub lower_included: Option<bool>,
    /// Whether the upper end is itself admitted, where the template fixes it.
    pub upper_included: Option<bool>,
    /// Whether the interval has no lower end, where the template fixes it.
    pub lower_unbounded: Option<bool>,
    /// Whether the interval has no upper end, where the template fixes it.
    pub upper_unbounded: Option<bool>,
}

/// One transition out of a state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransitionOption {
    /// The event that fires the transition.
    pub event: String,
    /// The action the transition performs.
    pub action: Option<String>,
    /// The guard the transition is subject to.
    pub guard: Option<String>,
    /// The state the transition leads to.
    pub next_state: Option<String>,
}

/// One state a value may be in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateOption {
    /// The state's name.
    pub name: String,
    /// Whether the state machine ends here.
    pub is_terminal: bool,
    /// The transitions out of this state, which are the only states reachable
    /// once the value is in it.
    pub transitions: Vec<StateTransitionOption>,
}

/// A state of a state machine.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 4.2.3, `DV_STATE`. The
/// constrainer that expresses it, `C_DV_STATE`, is declared only in the
/// openEHR ITS-XML 2.0.0 `OpenehrProfile.xsd` and by no AOM prose in either
/// generation, so this field carries the states and their transitions and
/// nothing that prose does not define.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateField {
    /// The states, in the order the template lists them.
    pub states: Vec<StateOption>,
}

/// One alternative of a choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceAlternative {
    /// What names the alternative, so a composition builder knows which node
    /// a filled value belongs to.
    pub key: NodeKey,
    /// The Reference Model class the alternative collects.
    pub rm_type: RmTypeName,
    /// The text the alternative is labelled with.
    pub label: Localized,
    /// How many times the alternative may appear.
    pub occurrences: Occurrences,
    /// What the alternative collects.
    pub kind: FieldKind,
    /// The reference bands shown beside this alternative's value, where the
    /// template states any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_ranges: Option<Box<ReferenceRanges>>,
    /// The value the template prefills the alternative with.
    pub prefill: Option<Prefill>,
}

/// A field the clinician fills through exactly one of several alternatives.
///
/// openEHR AM Release-2.3.0 `AOM2.html` section 4.2.8.2 prescribes two sibling
/// nodes, one coded and one text-only, for a value that may be either; the
/// alternatives under any single-valued attribute are the general case, and
/// this variant is the general answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceField {
    /// The alternatives, in the order the template lists them.
    pub alternatives: Vec<ChoiceAlternative>,
}

/// The null flavours openEHR RM Release-1.1.0 `data_structures.html`
/// section 4.1 names, with the rubrics it prints.
#[must_use]
pub fn null_flavour_options() -> Vec<NullFlavourOption> {
    [
        ("253", "unknown"),
        ("271", "no information"),
        ("272", "masked"),
        ("273", "not applicable"),
    ]
    .into_iter()
    .map(|(code, rubric)| NullFlavourOption {
        code: Code::new(TerminologyName::new("openehr"), code),
        rubric: rubric.to_owned(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        BooleanField, FieldKind, ProportionKind, ReferenceRange, ReferenceRanges,
        null_flavour_options,
    };

    #[test]
    fn a_band_set_is_empty_only_while_the_template_states_no_band() {
        let mut bands = ReferenceRanges::default();
        assert!(bands.is_empty());
        bands.other.push(ReferenceRange {
            meaning: None,
            range: None,
        });
        assert!(!bands.is_empty());
    }

    #[test]
    fn the_four_null_flavours_are_the_ones_the_spec_names() {
        let codes: Vec<String> = null_flavour_options()
            .into_iter()
            .map(|option| option.code.code)
            .collect();
        assert_eq!(codes, ["253", "271", "272", "273"]);
    }

    #[test]
    fn a_field_kind_serializes_under_the_name_of_its_variant() {
        let kind = FieldKind::Boolean(BooleanField {
            true_allowed: true,
            false_allowed: true,
        });
        let json = serde_json::to_string(&kind).expect("a field kind serializes");
        assert_eq!(
            json,
            r#"{"boolean":{"true_allowed":true,"false_allowed":true}}"#
        );
        assert_eq!(kind.name(), "boolean");
    }

    #[test]
    fn the_proportion_kinds_carry_the_integers_the_spec_assigns() {
        assert_eq!(ProportionKind::from_code(0), ProportionKind::Ratio);
        assert_eq!(ProportionKind::from_code(1), ProportionKind::Unitary);
        assert_eq!(ProportionKind::from_code(2), ProportionKind::Percent);
        assert_eq!(ProportionKind::from_code(3), ProportionKind::Fraction);
        assert_eq!(
            ProportionKind::from_code(4),
            ProportionKind::IntegerFraction
        );
        assert_eq!(ProportionKind::from_code(9), ProportionKind::Other(9));
    }

    #[test]
    fn every_proportion_kind_round_trips_through_its_integer() {
        for code in [0, 1, 2, 3, 4, 9, -1] {
            assert_eq!(ProportionKind::from_code(code).code(), code);
        }
    }
}
