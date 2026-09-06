// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The assertions on an ADL 1.4 archetype slot.
//!
//! openEHR AM Release-2.3.0 `AOM1.4.html` section 4.4 models a slot assertion
//! as an expression tree over `EXPR_ITEM`. The one shape an operational
//! template writes constrains `archetype_id/value` against a `C_STRING`, so
//! that shape is read into a typed assertion and every other shape is kept as
//! text rather than dropped.

use openehr_its::opt14::types as opt;

use crate::model::payload::SlotAssertion;

/// Reads one slot assertion.
pub(crate) fn assertion(assertion: &opt::Assertion) -> Vec<SlotAssertion> {
    if let Some(read) = archetype_id_match(&assertion.expression) {
        return read;
    }
    vec![SlotAssertion::Opaque(
        assertion
            .string_expression
            .clone()
            .unwrap_or_else(|| render(&assertion.expression)),
    )]
}

/// The `archetype_id/value matches {C_STRING}` shape, where the expression has
/// it.
fn archetype_id_match(expression: &opt::ExprItem) -> Option<Vec<SlotAssertion>> {
    let opt::ExprItem::ExprBinaryOperator(ref binary) = *expression else {
        return None;
    };
    if binary.operator != opt::OperatorKind::Matches {
        return None;
    }
    let opt::ExprItem::ExprLeaf(ref left) = *binary.left_operand else {
        return None;
    };
    if left.item.text().trim() != "archetype_id/value" {
        return None;
    }
    let opt::ExprItem::ExprLeaf(ref right) = *binary.right_operand else {
        return None;
    };
    let constraint = &right.item;
    let mut read = Vec::new();
    for pattern in constraint.children_named("pattern") {
        read.push(SlotAssertion::ArchetypeIdPattern(pattern.text()));
    }
    for listed in constraint.children_named("list") {
        read.push(SlotAssertion::ArchetypeId(listed.text()));
    }
    (!read.is_empty()).then_some(read)
}

/// A compact rendering of an expression this reader does not interpret, so an
/// assertion is never silently dropped.
fn render(expression: &opt::ExprItem) -> String {
    match *expression {
        opt::ExprItem::ExprLeaf(ref leaf) => {
            let text = leaf.item.text();
            let text = text.trim();
            if text.is_empty() {
                format!("<{}>", leaf.reference_type)
            } else {
                text.to_owned()
            }
        }
        opt::ExprItem::ExprUnaryOperator(ref unary) => {
            format!("{} {}", unary.operator, render(&unary.operand))
        }
        opt::ExprItem::ExprBinaryOperator(ref binary) => format!(
            "({} {} {})",
            render(&binary.left_operand),
            binary.operator,
            render(&binary.right_operand)
        ),
    }
}
