// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! One clinical constraint, written in both generations, read into one model.
//!
//! `docs/architecture.md` section 3 names this the narrow waist: both readers
//! normalize into the internal constraint model so the field derivation is
//! written once. No specification governs that decision; it is FerroCHART's
//! own design, and this module is the one place that says what it means for
//! the two readers to have met it.
//!
//! The two source artefacts are hand-authored twice over, once as an ADL 1.4
//! operational template and once as ADL 2 source. They are synthetic, and they
//! are the only matched-pair evidence this project has: the published
//! dual-dialect archetype library cannot supply the ADL 1.4 half in any
//! serialisation the ADL 1.4 reader reads, which `adl2_corpus.rs` records.
//!
//! # What "equivalent" means
//!
//! Two templates are equivalent when [`normalized_pair`] rewrites them into
//! two values that compare equal with `==`, node for node and field for field.
//! The comparison is a whole-value equality rather than a list of assertions,
//! so a field the internal model grows cannot be left out of it: every part
//! goes back through [`ConstraintNode::new`] and [`ConstraintTemplate::new`],
//! and a constructor that gains a parameter stops this file compiling.
//!
//! # The differences accepted as legitimate
//!
//! Each one is neutralized in [`normalized_pair`] and nowhere else. Nothing
//! else is neutralized, and a pair that differs anywhere else fails naming the
//! archetype and the node.
//!
//! 1. **A node's own code is spelled differently.** ADL 1.4 identifies a node
//!    with an `at`-code (openEHR AM Release-2.3.0 `ADL1.4.html` section 7.2);
//!    ADL 2 identifies it with an `id`-code and leaves `at`-codes for values
//!    (`ADL2.html` section 7.13.5.1). Every local code is replaced by the
//!    rubric its own terminology gives it, so the comparison is over clinical
//!    concepts rather than over code spellings.
//! 2. **A coded value's code is spelled differently**, for the same reason and
//!    with the same citation. A code from any terminology other than the
//!    archetype's own is compared verbatim, because both generations state an
//!    external code the same way (openEHR RM Release-1.1.0 `data_types.html`
//!    section 5.2.3, `CODE_PHRASE`).
//! 3. **A leaf may carry no code at all in ADL 1.4.** openEHR AM
//!    Release-2.3.0 `AOM1.4.html` section 4.2.3.1 lets a node with no siblings
//!    go unidentified, where an id-coded ADL 2 archetype identifies every
//!    object (`ADL2.html` section 7.13.5.1). Where the ADL 1.4 side states no
//!    code and the ADL 2 side states one, the ADL 2 code is dropped and the
//!    allowance is counted, so it cannot widen unnoticed. The reverse, a code
//!    on the ADL 1.4 side and none on the ADL 2 side, is a failure.
//! 4. **The artefact names itself differently.** The ADL 1.4 half is an
//!    operational template with a `TEMPLATE_ID`; the ADL 2 half is generated
//!    from an archetype, so its identifier is the archetype's. Both are
//!    replaced by one placeholder.
//! 5. **The template states occurrences in one generation and may omit them
//!    in the other.** ITS-XML 2.0.0 `Archetype.xsd` declares
//!    `C_OBJECT.occurrences` without `minOccurs="0"`, so an ADL 1.4
//!    operational template always states them, where openEHR AM Release-2.3.0
//!    `AOM2.html` section 4.5.2 makes them optional and inferred. The
//!    occurrences themselves are compared; only the flag recording whether the
//!    template said them out loud is dropped.
//! 6. **The owning attribute's existence and cardinality may be unstated in
//!    ADL 2.** openEHR AM Release-2.3.0 `AOM2.html` section 4.2.2 §Attribute
//!    Nodes: "Both existence and cardinality are optional in the model, since
//!    they are only needed to override the settings from the reference
//!    model." ITS-XML 2.0.0 `Archetype.xsd` declares
//!    `C_ATTRIBUTE.existence` without `minOccurs="0"`, so an ADL 1.4
//!    operational template always states it. Where one generation states one
//!    of the two and the other leaves it to the Reference Model, both are
//!    dropped and the allowance is counted; where both state it, it is
//!    compared. Neither reader resolves an unstated value against the
//!    Reference Model, so there is nothing else the comparison could hold
//!    them to.
//!
//! Three spellings are already normalized by the readers themselves, so this
//! comparison asserts nothing about them and says so rather than appearing to:
//! a `DV_QUANTITY` or `DV_ORDINAL` written as an AOM 2 tuple is folded into the
//! same payload as the ADL 1.4 domain type (openEHR AM Release-2.3.0
//! `AOM2.html` section 4.3 §Tuple Constraints), a regular expression keeps the
//! delimiters off in both generations, and a slot assertion is recorded as the
//! bare archetype-id expression in both.

use std::cell::RefCell;
use std::collections::BTreeMap;

use ferrochart_compile::model::ids::{
    ArchetypeId, CodeKind, LanguageTag, LocalCode, RmAttributeName, RmTypeName, TemplateId,
    TerminologyName,
};
use ferrochart_compile::model::multiplicity::{AttributeContext, Multiplicity};
use ferrochart_compile::model::node::{ConstraintNode, ConstraintTemplate, NodeIdentity};
use ferrochart_compile::model::payload::{
    CodeSource, CodedConstraint, CodedValue, ConstraintPayload, DefaultValue, ExternalSet,
    OrdinalConstraint, OrdinalOption, QuantityConstraint, TupleConstraint,
};
use ferrochart_compile::model::terminology::{ExternalTerm, TermDefinition, Terminology};
use ferrochart_compile::{adl2, adl14};

const AS_OPT14: &str = include_str!("../fixtures/ferro_test_observation.opt");
const AS_ADL2: &str = include_str!("../fixtures/ferro_test_observation.v1.0.0.adls");

/// The identifier both halves are given, because neither generation's own is
/// comparable (§The differences accepted as legitimate, item 4).
const ONE_ARTEFACT: &str = "the matched pair";

/// The terminology name an archetype's own codes are drawn from.
///
/// openEHR RM Release-1.1.0 `data_types.html` section 5.2.3,
/// `CODE_PHRASE.terminology_id`.
const LOCAL: &str = "local";

fn pair() -> (ConstraintTemplate, ConstraintTemplate) {
    (
        adl14::from_xml(AS_OPT14).expect("the ADL 1.4 fixture reads"),
        adl2::from_source(AS_ADL2, &[]).expect("the ADL 2 fixture reads"),
    )
}

/// How many times a legitimate difference was neutralized, so an allowance
/// cannot widen without the count moving.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Allowances {
    /// Nodes the ADL 1.4 half leaves unidentified where the ADL 2 half states
    /// an `id`-code (§The differences accepted as legitimate, item 3).
    pub(crate) unidentified_in_adl14: usize,
    /// Attribute slots where one generation states an existence and the other
    /// leaves it to the Reference Model (item 6).
    pub(crate) unstated_existence: usize,
    /// Attribute slots where one generation states a cardinality and the other
    /// leaves it to the Reference Model (item 6).
    pub(crate) unstated_cardinality: usize,
}

/// Rewrites both halves into the two values the equivalence compares, and
/// reports every allowance it spent doing so.
///
/// The pair is rewritten together rather than one side at a time, because item
/// 3 of the legitimate differences is a rule about the two sides at once.
///
/// # Panics
/// When a local code carries no rubric in the template's own language, naming
/// the archetype and the code: a code the model refers to and the terminology
/// does not define is a defect in the reader, not a difference to accept.
pub(crate) fn normalized_pair(
    from_opt14: &ConstraintTemplate,
    from_adl2: &ConstraintTemplate,
) -> (ConstraintTemplate, ConstraintTemplate, Allowances) {
    let mut allowances = Allowances::default();
    let left = Side::new(from_opt14);
    let right = Side::new(from_adl2);
    let (left_root, right_root) = normalize_nodes(
        &left,
        from_opt14.root(),
        &right,
        from_adl2.root(),
        "",
        &mut allowances,
    );
    (
        ConstraintTemplate::new(
            TemplateId::new(ONE_ARTEFACT),
            from_opt14.language().clone(),
            left_root,
            left.terminologies(),
        ),
        ConstraintTemplate::new(
            TemplateId::new(ONE_ARTEFACT),
            from_adl2.language().clone(),
            right_root,
            right.terminologies(),
        ),
        allowances,
    )
}

/// The codes one half's compared model refers to, per archetype.
type Mentions = BTreeMap<ArchetypeId, Vec<LocalCode>>;

/// One half of the pair, with the lookups the rewrite needs.
struct Side<'a> {
    template: &'a ConstraintTemplate,
    language: LanguageTag,
    /// Every code the rebuilt model refers to, filled as the rebuild goes.
    mentions: RefCell<Mentions>,
}

impl<'a> Side<'a> {
    fn new(template: &'a ConstraintTemplate) -> Self {
        Self {
            template,
            language: template.language().clone(),
            mentions: RefCell::new(Mentions::new()),
        }
    }

    /// Records that the rebuilt model refers to `codes` in the archetype
    /// `scope` names.
    fn mention(&self, scope: &ArchetypeId, codes: Vec<LocalCode>) {
        self.mentions
            .borrow_mut()
            .entry(scope.clone())
            .or_default()
            .extend(codes);
    }

    /// The rubric `code` carries in the archetype `scope` names.
    fn rubric(&self, scope: &ArchetypeId, code: &LocalCode) -> String {
        self.term(scope, code)
            .unwrap_or_else(|| panic!("{scope} defines no rubric for {code} in {}", self.language))
            .text()
            .to_owned()
    }

    fn term(&self, scope: &ArchetypeId, code: &LocalCode) -> Option<&'a TermDefinition> {
        self.template
            .terminology(scope)?
            .rubric(&self.language, code)
    }

    /// The rubric a code is rewritten to, or the code itself where it is not
    /// one the archetype defines.
    fn concept(&self, scope: &ArchetypeId, code: &LocalCode) -> LocalCode {
        match code.kind() {
            CodeKind::Other => code.clone(),
            CodeKind::Node | CodeKind::Term | CodeKind::ValueSet => {
                LocalCode::new(self.rubric(scope, code))
            }
            _ => panic!("the equivalence has no rule for the code kind of {code}"),
        }
    }

    /// The terminology content the tree refers to, keyed by concept rather
    /// than by code.
    ///
    /// The rebuild is driven by the codes the tree mentions, because that is
    /// what the two generations can be held to: a definition neither side's
    /// nodes reach is not a constraint the form derivation ever sees.
    fn terminologies(&self) -> BTreeMap<ArchetypeId, Terminology> {
        let mut built = BTreeMap::new();
        for (scope, codes) in &*self.mentions.borrow() {
            let scope = scope.clone();
            let mut codes = codes.clone();
            codes.sort();
            codes.dedup();
            let mut terminology = Terminology::new();
            for code in &codes {
                if code.kind() == CodeKind::Other {
                    continue;
                }
                let key = self.concept(&scope, code);
                if let Some(term) = self.term(&scope, code) {
                    terminology.insert_definition(self.language.clone(), key.clone(), term.clone());
                }
                self.copy_bindings(&scope, code, &key, &mut terminology);
            }
            built.insert(scope, terminology);
        }
        built
    }

    fn copy_bindings(
        &self,
        scope: &ArchetypeId,
        code: &LocalCode,
        key: &LocalCode,
        into: &mut Terminology,
    ) {
        let Some(source) = self.template.terminology(scope) else {
            return;
        };
        for target in source.bindings(code).into_iter().flatten() {
            into.insert_binding(key.clone(), clone_term(target.1));
        }
        for target in source.constraint_bindings(code).into_iter().flatten() {
            into.insert_constraint_binding(key.clone(), clone_term(target.1));
        }
        if let Some(members) = source.value_set(code) {
            let members = members.iter().map(|m| self.concept(scope, m)).collect();
            into.insert_value_set(key.clone(), members);
        }
    }
}

fn clone_term(term: &ExternalTerm) -> ExternalTerm {
    ExternalTerm::new(term.terminology().clone(), term.value().to_owned())
}

/// Rewrites one node of each half and, on a difference, fails naming the
/// archetype and the node.
fn normalize_nodes(
    left: &Side<'_>,
    left_node: &ConstraintNode,
    right: &Side<'_>,
    right_node: &ConstraintNode,
    path: &str,
    allowances: &mut Allowances,
) -> (ConstraintNode, ConstraintNode) {
    let (left_id, right_id) = surviving_codes(left_node, right_node, allowances);
    let (left_attribute, right_attribute) =
        normalize_attributes(left_node.attribute(), right_node.attribute(), allowances);
    let left_shallow = rebuild(left, left_node, left_id, left_attribute);
    let right_shallow = rebuild(right, right_node, right_id, right_attribute);
    assert_eq!(
        left_shallow,
        right_shallow,
        "at {} {path}: the two generations state different constraints",
        left_node.terminology_scope()
    );
    assert_eq!(
        left_node.children().len(),
        right_node.children().len(),
        "at {} {path}: the two generations state a different number of children",
        left_node.terminology_scope()
    );

    let mut left_children = Vec::with_capacity(left_node.children().len());
    let mut right_children = Vec::with_capacity(right_node.children().len());
    for (index, (left_child, right_child)) in left_node
        .children()
        .iter()
        .zip(right_node.children().iter())
        .enumerate()
    {
        let child_path = format!("{path}/{}[{index}]", left_child.identity().rm_attribute());
        let (built_left, built_right) = normalize_nodes(
            left,
            left_child,
            right,
            right_child,
            &child_path,
            allowances,
        );
        left_children.push(built_left);
        right_children.push(built_right);
    }
    (
        with_children(&left_shallow, left_children),
        with_children(&right_shallow, right_children),
    )
}

/// Applies items 1 and 3 to one pair of nodes, returning the code each side
/// keeps, still spelled as its own generation spells it.
fn surviving_codes(
    left: &ConstraintNode,
    right: &ConstraintNode,
    allowances: &mut Allowances,
) -> (Option<LocalCode>, Option<LocalCode>) {
    let left_code = left.identity().node_id().cloned();
    let right_code = right.identity().node_id().cloned();
    if left_code.is_none() && right_code.is_some() {
        allowances.unidentified_in_adl14 += 1;
        return (None, None);
    }
    (left_code, right_code)
}

/// Applies item 6 to one pair of attribute slots.
fn normalize_attributes(
    left: AttributeContext,
    right: AttributeContext,
    allowances: &mut Allowances,
) -> (AttributeContext, AttributeContext) {
    let (left_existence, right_existence) =
        if left.existence().is_some() == right.existence().is_some() {
            (left.existence(), right.existence())
        } else {
            allowances.unstated_existence += 1;
            (None, None)
        };
    let (left_cardinality, right_cardinality) =
        if left.cardinality().is_some() == right.cardinality().is_some() {
            (left.cardinality(), right.cardinality())
        } else {
            allowances.unstated_cardinality += 1;
            (None, None)
        };
    (
        context(left.is_container(), left_existence, left_cardinality),
        context(right.is_container(), right_existence, right_cardinality),
    )
}

fn context(
    is_container: bool,
    existence: Option<Multiplicity>,
    cardinality: Option<ferrochart_compile::model::multiplicity::Cardinality>,
) -> AttributeContext {
    if is_container {
        AttributeContext::container(existence, cardinality)
    } else {
        AttributeContext::single(existence)
    }
}

/// Rebuilds one node with its codes rewritten, and records every code the
/// rebuilt node refers to.
///
/// Every part of [`ConstraintNode`] passes through here, so a node that grows
/// a part stops this file compiling until the equivalence says what to do with
/// it. The children are attached afterwards by [`with_children`], so a
/// difference is reported at the node that carries it rather than at the root.
fn rebuild(
    side: &Side<'_>,
    node: &ConstraintNode,
    node_id: Option<LocalCode>,
    attribute: AttributeContext,
) -> ConstraintNode {
    let scope = node.terminology_scope();
    let mut mentioned = Vec::new();
    if let Some(ref code) = node_id {
        mentioned.push(code.clone());
    }
    local_codes(node.payload(), &mut mentioned);
    if let Some(value) = node.default_value() {
        local_codes_of_default(value, &mut mentioned);
    }
    side.mention(scope, mentioned);
    let identity = NodeIdentity::new(
        node.identity().rm_attribute().clone(),
        node_id.map(|code| side.concept(scope, &code)),
        node.identity().archetype_id().cloned(),
        node.identity().rm_type().clone(),
        node.identity().pinned_name().map(str::to_owned),
        node.identity().sibling_ordinal(),
    );
    ConstraintNode::new(
        identity,
        node.occurrences(),
        // Item 5: whether the template said the occurrences out loud is a
        // generational fact, not a constraint.
        false,
        attribute,
        normalize_payload(side, scope, node.payload()),
        scope.clone(),
        node.is_deprecated(),
        node.default_value()
            .map(|value| normalize_default(side, scope, value)),
        Vec::new(),
    )
}

fn with_children(node: &ConstraintNode, children: Vec<ConstraintNode>) -> ConstraintNode {
    ConstraintNode::new(
        node.identity().clone(),
        node.occurrences(),
        node.occurrences_stated(),
        node.attribute(),
        node.payload().clone(),
        node.terminology_scope().clone(),
        node.is_deprecated(),
        node.default_value().cloned(),
        children,
    )
}

/// Rewrites the local codes a payload carries.
///
/// # Panics
/// On a payload variant this equivalence has never been told about:
/// [`ConstraintPayload`] is `#[non_exhaustive]`, so a new variant would
/// otherwise pass through unexamined and a local code inside it would be
/// compared as a code spelling.
fn normalize_payload(
    side: &Side<'_>,
    scope: &ArchetypeId,
    payload: &ConstraintPayload,
) -> ConstraintPayload {
    match *payload {
        ConstraintPayload::Coded(ref coded) => {
            ConstraintPayload::Coded(normalize_coded(side, scope, coded))
        }
        ConstraintPayload::Ordinal(ref ordinal) => ConstraintPayload::Ordinal(OrdinalConstraint {
            options: ordinal
                .options
                .iter()
                .map(|option| normalize_option(side, scope, option))
                .collect(),
            assumed_value: ordinal
                .assumed_value
                .as_ref()
                .map(|option| normalize_option(side, scope, option)),
        }),
        ConstraintPayload::Quantity(ref quantity) => {
            ConstraintPayload::Quantity(QuantityConstraint {
                property: quantity
                    .property
                    .as_ref()
                    .map(|code| normalize_coded_value(side, scope, code)),
                units: quantity.units.clone(),
                assumed_value: quantity.assumed_value.clone(),
            })
        }
        ConstraintPayload::Tuple(ref tuple) => ConstraintPayload::Tuple(TupleConstraint {
            members: tuple.members.clone(),
            rows: tuple
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| normalize_payload(side, scope, cell))
                        .collect()
                })
                .collect(),
        }),
        ConstraintPayload::Structure
        | ConstraintPayload::Boolean(_)
        | ConstraintPayload::Integer(_)
        | ConstraintPayload::Real(_)
        | ConstraintPayload::Text(_)
        | ConstraintPayload::Date(_)
        | ConstraintPayload::Time(_)
        | ConstraintPayload::DateTime(_)
        | ConstraintPayload::Duration(_)
        | ConstraintPayload::State(_)
        | ConstraintPayload::OpenSlot(_) => payload.clone(),
        _ => panic!(
            "the equivalence has no rule for a {} payload",
            payload.kind()
        ),
    }
}

fn normalize_coded(
    side: &Side<'_>,
    scope: &ArchetypeId,
    coded: &CodedConstraint,
) -> CodedConstraint {
    CodedConstraint {
        source: normalize_source(side, scope, &coded.source),
        assumed_value: coded
            .assumed_value
            .as_ref()
            .map(|code| normalize_coded_value(side, scope, code)),
        status: coded.status,
    }
}

/// # Panics
/// On a [`CodeSource`] or [`ExternalSet`] variant this equivalence has never
/// been told about; both are `#[non_exhaustive]`.
fn normalize_source(side: &Side<'_>, scope: &ArchetypeId, source: &CodeSource) -> CodeSource {
    match *source {
        CodeSource::Enumerated {
            ref terminology,
            ref codes,
        } if terminology.as_str() == LOCAL => CodeSource::Enumerated {
            terminology: terminology.clone(),
            codes: codes
                .iter()
                .map(|code| side.rubric(scope, &LocalCode::new(code.clone())))
                .collect(),
        },
        CodeSource::Enumerated { .. }
        | CodeSource::OpenTerminology { .. }
        | CodeSource::Unconstrained
        | CodeSource::External(ExternalSet::ReferenceSet { .. }) => source.clone(),
        CodeSource::External(ExternalSet::ConstraintCode {
            ref code,
            ref bindings,
            ref operational_terminology,
        }) => CodeSource::External(ExternalSet::ConstraintCode {
            code: side.concept(scope, code),
            bindings: bindings.clone(),
            operational_terminology: operational_terminology.clone(),
        }),
        CodeSource::External(ref other) => {
            panic!("the equivalence has no rule for the external set {other:?}")
        }
        _ => panic!("the equivalence has no rule for the code source {source:?}"),
    }
}

fn normalize_option(side: &Side<'_>, scope: &ArchetypeId, option: &OrdinalOption) -> OrdinalOption {
    OrdinalOption {
        value: option.value,
        symbol: normalize_coded_value(side, scope, &option.symbol),
    }
}

fn normalize_coded_value(side: &Side<'_>, scope: &ArchetypeId, value: &CodedValue) -> CodedValue {
    if value.terminology().as_str() != LOCAL {
        return CodedValue::new(
            value.terminology().clone(),
            value.code().to_owned(),
            value.preferred_term().map(str::to_owned),
        );
    }
    CodedValue::new(
        value.terminology().clone(),
        side.concept(scope, &LocalCode::new(value.code().to_owned()))
            .into_inner(),
        value.preferred_term().map(str::to_owned),
    )
}

/// # Panics
/// On a [`DefaultValue`] variant this equivalence has never been told about;
/// the type is `#[non_exhaustive]`.
fn normalize_default(side: &Side<'_>, scope: &ArchetypeId, value: &DefaultValue) -> DefaultValue {
    match *value {
        DefaultValue::Coded {
            ref code,
            ref rubric,
        } => DefaultValue::Coded {
            code: normalize_coded_value(side, scope, code),
            rubric: rubric.clone(),
        },
        DefaultValue::Ordinal {
            value: ordinal,
            ref symbol,
        } => DefaultValue::Ordinal {
            value: ordinal,
            symbol: normalize_coded_value(side, scope, symbol),
        },
        DefaultValue::Boolean(_)
        | DefaultValue::Integer(_)
        | DefaultValue::Real(_)
        | DefaultValue::Text(_)
        | DefaultValue::Quantity { .. }
        | DefaultValue::Temporal(_)
        | DefaultValue::Opaque(_) => value.clone(),
        _ => panic!("the equivalence has no rule for the default value {value:?}"),
    }
}

/// Collects every code from the archetype's own terminology that a payload
/// refers to.
fn local_codes(payload: &ConstraintPayload, out: &mut Vec<LocalCode>) {
    match *payload {
        ConstraintPayload::Coded(ref coded) => {
            match coded.source {
                CodeSource::Enumerated {
                    ref terminology,
                    ref codes,
                } if terminology.as_str() == LOCAL => {
                    out.extend(codes.iter().map(|code| LocalCode::new(code.clone())));
                }
                CodeSource::External(ExternalSet::ConstraintCode { ref code, .. }) => {
                    out.push(code.clone());
                }
                _ => {}
            }
            if let Some(ref assumed) = coded.assumed_value {
                push_local(assumed, out);
            }
        }
        ConstraintPayload::Ordinal(ref ordinal) => {
            for option in &ordinal.options {
                push_local(&option.symbol, out);
            }
            if let Some(ref assumed) = ordinal.assumed_value {
                push_local(&assumed.symbol, out);
            }
        }
        ConstraintPayload::Quantity(ref quantity) => {
            if let Some(ref property) = quantity.property {
                push_local(property, out);
            }
        }
        ConstraintPayload::Tuple(ref tuple) => {
            for cell in tuple.rows.iter().flatten() {
                local_codes(cell, out);
            }
        }
        _ => {}
    }
}

fn local_codes_of_default(value: &DefaultValue, out: &mut Vec<LocalCode>) {
    match *value {
        DefaultValue::Coded { ref code, .. } => push_local(code, out),
        DefaultValue::Ordinal { ref symbol, .. } => push_local(symbol, out),
        _ => {}
    }
}

fn push_local(value: &CodedValue, out: &mut Vec<LocalCode>) {
    if value.terminology().as_str() == LOCAL {
        out.push(LocalCode::new(value.code().to_owned()));
    }
}

/// The rubric a node's own terminology gives its code, which is the name of
/// the clinical concept rather than the code that happens to spell it.
fn rubric<'a>(template: &'a ConstraintTemplate, node: &ConstraintNode) -> Option<&'a str> {
    let code = node.identity().node_id()?;
    template
        .terminology(node.terminology_scope())?
        .rubric(template.language(), code)
        .map(TermDefinition::text)
}

#[test]
fn both_generations_produce_the_same_internal_model() {
    let (opt14, from_adl2) = pair();
    let (left, right, allowances) = normalized_pair(&opt14, &from_adl2);
    assert_eq!(left, right);
    // The three leaves the ADL 1.4 half leaves unidentified, its quantity, its
    // coded text and its text (item 3), and the six attribute slots where the
    // ADL 2 half leaves existence to the Reference Model (item 6).
    assert_eq!(
        allowances,
        Allowances {
            unidentified_in_adl14: 3,
            unstated_existence: 6,
            unstated_cardinality: 0,
        }
    );
}

#[test]
fn the_equivalence_reads_every_node_of_the_pair() {
    // The rewrite walks the two trees in lockstep, so a comparison that
    // silently stopped early would still pass. This pins the size it covered.
    let (opt14, from_adl2) = pair();
    let (left, right, _) = normalized_pair(&opt14, &from_adl2);
    assert_eq!(left.walk().count(), 10);
    assert_eq!(right.walk().count(), 10);
}

#[test]
fn a_difference_the_equivalence_does_not_accept_fails_naming_the_node() {
    let (opt14, from_adl2) = pair();
    let mutated = ConstraintTemplate::new(
        from_adl2.id().clone(),
        from_adl2.language().clone(),
        widen_first_leaf(from_adl2.root()),
        from_adl2.terminologies().clone(),
    );
    let failure = std::panic::catch_unwind(|| normalized_pair(&opt14, &mutated))
        .expect_err("a widened occurrence is not an accepted difference");
    let message = failure
        .downcast_ref::<String>()
        .expect("the assertion carries its message");
    assert!(
        message.contains("openEHR-EHR-OBSERVATION.ferro_test.v1.0.0"),
        "{message}"
    );
    assert!(message.contains("/data[0]/items[0]"), "{message}");
}

/// Widens the occurrences of the first leaf under `items`, which is the
/// smallest change that is not an accepted difference.
fn widen_first_leaf(root: &ConstraintNode) -> ConstraintNode {
    fn rewrite(node: &ConstraintNode, done: &mut bool) -> ConstraintNode {
        let occurrences = if !*done && node.identity().rm_attribute().as_str() == "items" {
            *done = true;
            Multiplicity::unbounded_from(0)
        } else {
            node.occurrences()
        };
        ConstraintNode::new(
            node.identity().clone(),
            occurrences,
            node.occurrences_stated(),
            node.attribute(),
            node.payload().clone(),
            node.terminology_scope().clone(),
            node.is_deprecated(),
            node.default_value().cloned(),
            node.children()
                .iter()
                .map(|child| rewrite(child, done))
                .collect(),
        )
    }
    rewrite(root, &mut false)
}

#[test]
fn the_node_identity_of_each_generation_carries_its_own_code_kind() {
    // openEHR AM Release-2.3.0 ADL1.4.html section 7.2 identifies a node with
    // an `at`-code; `ADL2.html` section 7.13.5.1 identifies it with an
    // `id`-code and leaves `at`-codes for values. Both occupy the same slot of
    // the internal model, which is the whole mapping.
    let (opt14, from_adl2) = pair();
    for node in opt14.walk() {
        if let Some(code) = node.identity().node_id() {
            assert_eq!(code.kind(), CodeKind::Term, "{code} in the ADL 1.4 model");
        }
    }
    for node in from_adl2.walk() {
        if let Some(code) = node.identity().node_id() {
            assert_eq!(code.kind(), CodeKind::Node, "{code} in the ADL 2 model");
        }
    }
}

#[test]
fn the_same_concept_carries_the_same_rubric_in_both_generations() {
    let (opt14, from_adl2) = pair();
    assert_eq!(
        rubric(&opt14, opt14.root()),
        rubric(&from_adl2, from_adl2.root()),
        "the two roots name different concepts"
    );
}

#[test]
fn neither_model_records_which_reader_produced_it() {
    // The model carries no generation marker, so the only honest test is that
    // the two trees compare equal wherever they state the same constraint.
    let (opt14, from_adl2) = pair();
    assert_eq!(opt14.walk().count(), from_adl2.walk().count());
    for (left, right) in opt14.walk().zip(from_adl2.walk()) {
        assert_eq!(left.payload().kind(), right.payload().kind());
        assert_eq!(left.is_deprecated(), right.is_deprecated());
    }
}

#[test]
fn a_slot_assertion_reads_the_same_in_both_generations() {
    // ADL 1.4 states the bare archetype-id expression in the assertion's
    // `C_STRING.pattern`; ADL 2 writes a contained regexp with delimiters.
    // Both readers record the bare expression, so a slot compares.
    let from_opt14 = adl14::from_xml(include_str!("../fixtures/ferro_slots_and_refs.opt"))
        .expect("the ADL 1.4 fixture reads");
    let from_adl2 = adl2::from_source(
        include_str!("../fixtures/ferro_adl2_extras.v1.0.0.adls"),
        &[],
    )
    .expect("the ADL 2 fixture reads");
    let includes = |template: &ConstraintTemplate| {
        template
            .walk()
            .find_map(|node| match *node.payload() {
                ConstraintPayload::OpenSlot(ref slot) => Some(slot.includes.clone()),
                _ => None,
            })
            .expect("the fixture carries an open slot")
    };
    assert_eq!(includes(&from_opt14), includes(&from_adl2));
}

#[test]
fn an_external_code_is_compared_verbatim_rather_than_by_rubric() {
    // Item 2: only the archetype's own codes are rewritten, because both
    // generations spell an external code the same way (openEHR RM
    // Release-1.1.0 `data_types.html` section 5.2.3).
    let (opt14, _) = pair();
    let side = Side::new(&opt14);
    let scope = ArchetypeId::new("openEHR-EHR-OBSERVATION.ferro_test.v1.0.0");
    let snomed = CodedValue::new(TerminologyName::new("SNOMED-CT"), "27113001", None);
    assert_eq!(
        normalize_coded_value(&side, &scope, &snomed).code(),
        "27113001"
    );
}

#[test]
fn a_node_with_no_local_code_keeps_its_identity_parts() {
    // A node the archetype leaves unidentified still carries its attribute,
    // its type and its ordinal, so the rewrite cannot flatten two siblings
    // into one.
    let (opt14, _) = pair();
    let side = Side::new(&opt14);
    let node = ConstraintNode::new(
        NodeIdentity::new(
            RmAttributeName::new("value"),
            None,
            None,
            RmTypeName::new("DV_TEXT"),
            None,
            3,
        ),
        Multiplicity::exactly(1),
        true,
        AttributeContext::single(None),
        ConstraintPayload::Structure,
        ArchetypeId::new("openEHR-EHR-OBSERVATION.ferro_test.v1.0.0"),
        false,
        None,
        Vec::new(),
    );
    let rebuilt = rebuild(&side, &node, None, node.attribute());
    assert_eq!(rebuilt.identity().sibling_ordinal(), 3);
    assert_eq!(rebuilt.identity().rm_type().as_str(), "DV_TEXT");
}
