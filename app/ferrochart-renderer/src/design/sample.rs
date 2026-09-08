// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! A synthetic form that carries one field of every kind.
//!
//! No specification governs this: our own design. It is invented content for
//! the style guide and never patient data. It exists so the guide draws every
//! control the derivation table produces, which is what makes "eighteen
//! variants, eighteen controls" checkable by eye rather than by reading a
//! `match`.

use std::collections::BTreeSet;

use ferrochart_form::definition::{FORMAT_VERSION, FormDefinition};
use ferrochart_form::field::{
    BooleanField, ChoiceAlternative, ChoiceField, CodedField, ComponentValidity, CountField,
    DateField, DateTimeField, DurationComponent, DurationField, FieldKind, FormField,
    IdentifierField, IntervalField, MultimediaField, NullFlavour, OrdinalField, OrdinalOption,
    ParsableField, ProportionField, ProportionKind, QuantityField, QuantityUnitOption, RealField,
    StateField, StateOption, StateTransitionOption, TextField, TimeField, UriField,
};
use ferrochart_form::group::{
    FormGroup, FormItem, GroupShape, UndeterminedContent, UndeterminedReason,
};
use ferrochart_form::ids::{
    LanguageTag, RmAttributeName, RmTypeName, TemplateId, TerminologyName, local_terminology,
};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::occurrences::Occurrences;
use ferrochart_form::range::Range;
use ferrochart_form::text::Localized;
use ferrochart_form::value::{Code, CodedOption, EnumeratedSet, ValueSet};

/// The language the sample states its labels in.
pub(crate) fn english() -> LanguageTag {
    LanguageTag::new("en")
}

/// A one-step key under `items`, told apart by its node id.
fn key(node_id: &str, rm_type: &str) -> NodeKey {
    NodeKey::root().child(KeyStep {
        rm_attribute: RmAttributeName::new("items"),
        node_id: Some(ferrochart_form::ids::LocalCode::new(node_id)),
        archetype_id: None,
        rm_type: RmTypeName::new(rm_type),
        pinned_name: None,
        sibling_ordinal: 0,
    })
}

/// One field of the sample, optional and with a null flavour beside it.
fn field(node_id: &str, label: &str, rm_type: &str, kind: FieldKind) -> FormItem {
    FormItem::Field(Box::new(FormField {
        key: key(node_id, "ELEMENT"),
        rm_type: RmTypeName::new(rm_type),
        label: Localized::in_language(english(), label),
        help: Localized::empty(),
        occurrences: Occurrences::bounded(0, 1),
        kind,
        name_constraint: None,
        reference_ranges: None,
        is_ordered: false,
        is_unique: false,
        null_flavour: NullFlavour::offered(),
        prefill: None,
        is_deprecated: false,
        is_fixed: false,
    }))
}

/// One group of the sample.
fn group(node_id: &str, label: &str, occurrences: Occurrences, items: Vec<FormItem>) -> FormGroup {
    FormGroup {
        key: key(node_id, "CLUSTER"),
        rm_type: RmTypeName::new("CLUSTER"),
        archetype_id: None,
        label: Localized::in_language(english(), label),
        help: Localized::empty(),
        occurrences,
        shape: GroupShape::Cluster,
        name_constraint: None,
        is_ordered: false,
        is_unique: false,
        is_deprecated: false,
        items,
        undetermined: Vec::new(),
    }
}

/// A coded option of the sample's own terminology.
fn coded_option(code: &str, rubric: &str) -> CodedOption {
    CodedOption {
        code: Code::new(local_terminology(), code),
        label: Localized::in_language(english(), rubric),
        description: Localized::empty(),
    }
}

/// The eighteen kinds, one field each.
fn every_kind() -> Vec<FormItem> {
    let mut all = plain_kinds();
    all.extend(numeric_kinds());
    all.extend(temporal_kinds());
    all.extend(structured_kinds());
    all.extend(machine_kinds());
    all
}

/// The kinds that collect a word, a code, or a flag.
fn plain_kinds() -> Vec<FormItem> {
    vec![
        field(
            "at0001",
            "Boolean",
            "DV_BOOLEAN",
            FieldKind::Boolean(BooleanField {
                true_allowed: true,
                false_allowed: true,
            }),
        ),
        field(
            "at0002",
            "Text, from a closed list",
            "DV_TEXT",
            FieldKind::Text(TextField {
                patterns: Vec::new(),
                options: vec!["Sitting".to_owned(), "Standing".to_owned()],
                options_closed: true,
            }),
        ),
        field(
            "at0003",
            "URI",
            "DV_URI",
            FieldKind::Uri(UriField {
                patterns: Vec::new(),
                required_scheme: None,
            }),
        ),
        field(
            "at0004",
            "Coded",
            "DV_CODED_TEXT",
            FieldKind::Coded(CodedField {
                value_set: ValueSet::Enumerated(EnumeratedSet {
                    terminology: local_terminology(),
                    options: vec![
                        coded_option("at0041", "At rest"),
                        coded_option("at0042", "After exercise"),
                    ],
                    needs_display_lookup: false,
                }),
                strictness: None,
                rubric: None,
            }),
        ),
        field(
            "at0005",
            "Ordinal",
            "DV_ORDINAL",
            FieldKind::Ordinal(OrdinalField {
                options: vec![
                    OrdinalOption {
                        score: 0.0,
                        symbol: Code::new(local_terminology(), "at0051"),
                        label: Localized::in_language(english(), "None"),
                        description: Localized::empty(),
                    },
                    OrdinalOption {
                        score: 1.0,
                        symbol: Code::new(local_terminology(), "at0052"),
                        label: Localized::in_language(english(), "Mild"),
                        description: Localized::empty(),
                    },
                ],
            }),
        ),
    ]
}

/// The kinds that collect a number.
fn numeric_kinds() -> Vec<FormItem> {
    vec![
        field(
            "at0006",
            "Count",
            "DV_COUNT",
            FieldKind::Count(CountField {
                options: Vec::new(),
                ranges: vec![Range::new(Some(0), Some(20), true, true)],
            }),
        ),
        field(
            "at0007",
            "Quantity",
            "DV_QUANTITY",
            FieldKind::Quantity(QuantityField {
                property: None,
                units: vec![
                    QuantityUnitOption {
                        units: "mm[Hg]".to_owned(),
                        magnitude: Some(Range::new(Some(0.0), Some(300.0), true, true)),
                        decimals: Some(Range::new(Some(0), Some(0), true, true)),
                    },
                    QuantityUnitOption {
                        units: "kPa".to_owned(),
                        magnitude: Some(Range::new(Some(0.0), Some(40.0), true, true)),
                        decimals: Some(Range::new(Some(0), Some(1), true, true)),
                    },
                ],
            }),
        ),
        field(
            "at0008",
            "Proportion",
            "DV_PROPORTION",
            FieldKind::Proportion(ProportionField {
                kinds: vec![ProportionKind::Percent, ProportionKind::Fraction],
                numerator: RealField::default(),
                denominator: RealField::default(),
                decimals: CountField::default(),
                is_integral: None,
            }),
        ),
    ]
}

/// The kinds that collect a point or a length of time.
fn temporal_kinds() -> Vec<FormItem> {
    vec![
        field(
            "at0009",
            "Date, year and month only",
            "DV_DATE",
            FieldKind::Date(DateField {
                month: ComponentValidity::Mandatory,
                day: ComponentValidity::Prohibited,
                ranges: Vec::new(),
            }),
        ),
        field(
            "at0010",
            "Time",
            "DV_TIME",
            FieldKind::Time(TimeField {
                minute: ComponentValidity::Mandatory,
                second: ComponentValidity::Prohibited,
                timezone: ComponentValidity::Optional,
                ranges: Vec::new(),
            }),
        ),
        field(
            "at0011",
            "Date and time",
            "DV_DATE_TIME",
            FieldKind::DateTime(DateTimeField {
                month: ComponentValidity::Mandatory,
                day: ComponentValidity::Mandatory,
                hour: ComponentValidity::Mandatory,
                minute: ComponentValidity::Mandatory,
                second: ComponentValidity::Prohibited,
                timezone: ComponentValidity::Optional,
                ranges: Vec::new(),
            }),
        ),
        field(
            "at0012",
            "Duration",
            "DV_DURATION",
            FieldKind::Duration(DurationField {
                components: [
                    DurationComponent::Days,
                    DurationComponent::Hours,
                    DurationComponent::Minutes,
                ]
                .into_iter()
                .collect::<BTreeSet<_>>(),
                ranges: Vec::new(),
            }),
        ),
    ]
}

/// The kinds that collect more than one part.
fn structured_kinds() -> Vec<FormItem> {
    vec![
        field(
            "at0013",
            "Identifier",
            "DV_IDENTIFIER",
            FieldKind::Identifier(IdentifierField::default()),
        ),
        field(
            "at0014",
            "Multimedia",
            "DV_MULTIMEDIA",
            FieldKind::Multimedia(MultimediaField {
                media_types: ValueSet::Enumerated(EnumeratedSet {
                    terminology: TerminologyName::new("IANA_media-types"),
                    options: vec![CodedOption {
                        code: Code::new(TerminologyName::new("IANA_media-types"), "image/png"),
                        label: Localized::empty(),
                        description: Localized::empty(),
                    }],
                    needs_display_lookup: false,
                }),
                uri: None,
            }),
        ),
        field(
            "at0015",
            "Parsable",
            "DV_PARSABLE",
            FieldKind::Parsable(ParsableField::default()),
        ),
        field(
            "at0016",
            "Interval",
            "DV_INTERVAL",
            FieldKind::Interval(Box::new(IntervalField {
                element_rm_type: RmTypeName::new("DV_COUNT"),
                lower: FieldKind::Count(CountField::default()),
                upper: FieldKind::Count(CountField::default()),
                lower_included: Some(true),
                upper_included: None,
                lower_unbounded: None,
                upper_unbounded: None,
            })),
        ),
    ]
}

/// The kinds whose value is a machine or a set of alternatives.
fn machine_kinds() -> Vec<FormItem> {
    vec![
        field(
            "at0017",
            "State",
            "DV_STATE",
            FieldKind::State(StateField {
                states: vec![
                    StateOption {
                        name: "planned".to_owned(),
                        is_terminal: false,
                        transitions: vec![StateTransitionOption {
                            event: "start".to_owned(),
                            action: None,
                            guard: None,
                            next_state: Some("active".to_owned()),
                        }],
                    },
                    StateOption {
                        name: "active".to_owned(),
                        is_terminal: false,
                        transitions: vec![StateTransitionOption {
                            event: "finish".to_owned(),
                            action: None,
                            guard: None,
                            next_state: Some("completed".to_owned()),
                        }],
                    },
                    StateOption {
                        name: "completed".to_owned(),
                        is_terminal: true,
                        transitions: Vec::new(),
                    },
                ],
            }),
        ),
        field(
            "at0018",
            "Choice",
            "DV_TEXT",
            FieldKind::Choice(ChoiceField {
                alternatives: vec![
                    ChoiceAlternative {
                        key: key("at0181", "ELEMENT"),
                        rm_type: RmTypeName::new("DV_CODED_TEXT"),
                        label: Localized::in_language(english(), "A coded answer"),
                        occurrences: Occurrences::bounded(0, 1),
                        kind: FieldKind::Coded(CodedField {
                            value_set: ValueSet::Enumerated(EnumeratedSet {
                                terminology: local_terminology(),
                                options: vec![coded_option("at0183", "Yes, coded")],
                                needs_display_lookup: false,
                            }),
                            strictness: None,
                            rubric: None,
                        }),
                        reference_ranges: None,
                        prefill: None,
                    },
                    ChoiceAlternative {
                        key: key("at0182", "ELEMENT"),
                        rm_type: RmTypeName::new("DV_TEXT"),
                        label: Localized::in_language(english(), "Free text instead"),
                        occurrences: Occurrences::bounded(0, 1),
                        kind: FieldKind::Text(TextField::default()),
                        reference_ranges: None,
                        prefill: None,
                    },
                ],
            }),
        ),
    ]
}

/// The sample form the style guide draws.
pub(crate) fn form() -> FormDefinition {
    let mut repeats = group(
        "at0100",
        "A repeatable group, one to three times",
        Occurrences::bounded(1, 3),
        vec![field(
            "at0101",
            "A reading inside the repeat",
            "DV_COUNT",
            FieldKind::Count(CountField::default()),
        )],
    );
    repeats.undetermined.push(UndeterminedContent {
        key: key("at0102", "ELEMENT"),
        rm_type: RmTypeName::new("ELEMENT"),
        label: Localized::in_language(english(), "An element the template left open"),
        occurrences: Occurrences::bounded(0, 1),
        reason: UndeterminedReason::UnconstrainedValue,
    });

    let root = FormGroup {
        items: vec![
            FormItem::Group(Box::new(group(
                "at0000",
                "One field of every kind",
                Occurrences::bounded(1, 1),
                every_kind(),
            ))),
            FormItem::Group(Box::new(repeats)),
        ],
        ..group(
            "root",
            "A sample form",
            Occurrences::bounded(1, 1),
            Vec::new(),
        )
    };

    FormDefinition {
        format_version: FORMAT_VERSION,
        template_id: TemplateId::new("ferrochart.style_guide.v0"),
        default_language: english(),
        languages: vec![english()],
        root,
    }
}

#[cfg(test)]
mod tests {
    use ferrochart_form::field::FieldKind;

    use super::{every_kind, form};

    #[test]
    fn the_sample_draws_one_field_of_every_kind_the_table_produces() {
        let kinds: Vec<&str> = form()
            .fields()
            .map(|field| field.kind.name())
            .filter(|name| *name != "count")
            .collect();
        assert!(kinds.len() >= 17, "{kinds:?}");
        assert_eq!(every_kind().len(), 18);
    }

    #[test]
    fn no_two_fields_of_the_sample_share_a_key() {
        let mut keys: Vec<String> = form().fields().map(|field| field.key.to_string()).collect();
        let count = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), count, "two sample fields share one key");
    }

    #[test]
    fn the_sample_carries_a_visible_hole_the_template_left() {
        assert_eq!(form().undetermined().count(), 1);
    }

    #[test]
    fn every_kind_the_sample_lists_is_a_distinct_variant() {
        let mut names: Vec<&str> = every_kind()
            .iter()
            .filter_map(|item| match *item {
                ferrochart_form::group::FormItem::Field(ref field) => Some(field.kind.name()),
                _ => None,
            })
            .collect();
        assert_eq!(names.len(), 18);
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 18, "two sample fields share one kind");
        assert!(
            names.contains(
                &FieldKind::Boolean(ferrochart_form::field::BooleanField {
                    true_allowed: true,
                    false_allowed: true,
                })
                .name()
            )
        );
    }
}
