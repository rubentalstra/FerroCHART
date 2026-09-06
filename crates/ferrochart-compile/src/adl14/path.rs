// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Archetype paths, as an ADL 1.4 operational template writes them.
//!
//! openEHR BASE Release-1.2.0 `architecture_overview.html` section 11 defines
//! the path syntax: a sequence of `attribute[predicate]` steps, where the
//! predicate distinguishes siblings. An operational template uses two
//! predicate forms this reader has to resolve, the node identifier and the
//! archetype identifier, optionally conjoined with `and name/value='...'`.

use openehr_its::opt14::types as opt;

/// One step of an archetype path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathStep {
    /// The Reference Model attribute, absent only on a leading root
    /// predicate.
    pub(crate) attribute: Option<String>,
    /// The node identifier the predicate names.
    pub(crate) node_id: Option<String>,
    /// The archetype identifier the predicate names.
    pub(crate) archetype_id: Option<String>,
    /// The name the predicate pins.
    pub(crate) name: Option<String>,
}

/// Splits `path` into its steps, respecting brackets.
///
/// A name predicate contains `/` (`name/value='x'`), so a naive split on the
/// separator would cut a step in half.
pub(crate) fn parse(path: &str) -> Vec<PathStep> {
    let mut steps = Vec::new();
    let mut depth = 0_usize;
    let mut current = String::new();
    for ch in path.chars() {
        match ch {
            '[' => {
                depth = depth.saturating_add(1);
                current.push(ch);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            '/' if depth == 0 => {
                if !current.is_empty() {
                    steps.push(parse_step(&current));
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        steps.push(parse_step(&current));
    }
    steps
}

fn parse_step(text: &str) -> PathStep {
    let (attribute, predicate) = match (text.find('['), text.rfind(']')) {
        (Some(open), Some(close)) if close > open => (
            text.get(..open).unwrap_or_default(),
            text.get(open.saturating_add(1)..close).unwrap_or_default(),
        ),
        _ => (text, ""),
    };
    let mut step = PathStep {
        attribute: (!attribute.is_empty()).then(|| attribute.to_owned()),
        node_id: None,
        archetype_id: None,
        name: None,
    };
    for part in predicate.split(" and ") {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(value) = part.strip_prefix("name/value=") {
            step.name = Some(unquote(value));
        } else if is_local_code(part) {
            step.node_id = Some(part.to_owned());
        } else {
            step.archetype_id = Some(unquote(part));
        }
    }
    step
}

fn unquote(text: &str) -> String {
    let trimmed = text.trim();
    trimmed
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
        .unwrap_or(trimmed)
        .to_owned()
}

/// Whether `code` is one of the three ADL local-code forms.
fn is_local_code(code: &str) -> bool {
    ["at", "id", "ac"].iter().any(|prefix| {
        code.strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// Whether `object` satisfies `step`'s predicate.
fn matches(object: &opt::CObject, step: &PathStep) -> bool {
    if let Some(ref wanted) = step.node_id
        && node_id(object) != wanted
    {
        return false;
    }
    if let Some(ref wanted) = step.archetype_id {
        match *object {
            opt::CObject::CArchetypeRoot(ref root) if &root.archetype_id.value == wanted => {}
            _ => return false,
        }
    }
    if let Some(ref wanted) = step.name
        && super::name::pinned(object).as_deref() != Some(wanted.as_str())
    {
        return false;
    }
    true
}

/// The node identifier `object` carries.
pub(crate) fn node_id(object: &opt::CObject) -> &str {
    match *object {
        opt::CObject::ArchetypeInternalRef(ref o) => &o.node_id,
        opt::CObject::ArchetypeSlot(ref o) => &o.node_id,
        opt::CObject::ConstraintRef(ref o) => &o.node_id,
        opt::CObject::CArchetypeRoot(ref o) => &o.node_id,
        opt::CObject::CCodePhrase(ref o) => &o.node_id,
        opt::CObject::CCodeReference(ref o) => &o.node_id,
        opt::CObject::CComplexObject(ref o) => &o.node_id,
        opt::CObject::CDefinedObject(ref o) => &o.node_id,
        opt::CObject::CDvOrdinal(ref o) => &o.node_id,
        opt::CObject::CDvQuantity(ref o) => &o.node_id,
        opt::CObject::CDvState(ref o) => &o.node_id,
        opt::CObject::CPrimitiveObject(ref o) => &o.node_id,
        opt::CObject::TComplexObject(ref o) => &o.node_id,
    }
}

/// The Reference Model type `object` constrains.
pub(crate) fn rm_type(object: &opt::CObject) -> &str {
    match *object {
        opt::CObject::ArchetypeInternalRef(ref o) => &o.rm_type_name,
        opt::CObject::ArchetypeSlot(ref o) => &o.rm_type_name,
        opt::CObject::ConstraintRef(ref o) => &o.rm_type_name,
        opt::CObject::CArchetypeRoot(ref o) => &o.rm_type_name,
        opt::CObject::CCodePhrase(ref o) => &o.rm_type_name,
        opt::CObject::CCodeReference(ref o) => &o.rm_type_name,
        opt::CObject::CComplexObject(ref o) => &o.rm_type_name,
        opt::CObject::CDefinedObject(ref o) => &o.rm_type_name,
        opt::CObject::CDvOrdinal(ref o) => &o.rm_type_name,
        opt::CObject::CDvQuantity(ref o) => &o.rm_type_name,
        opt::CObject::CDvState(ref o) => &o.rm_type_name,
        opt::CObject::CPrimitiveObject(ref o) => &o.rm_type_name,
        opt::CObject::TComplexObject(ref o) => &o.rm_type_name,
    }
}

/// The occurrences `object` states.
pub(crate) fn occurrences(object: &opt::CObject) -> &opt::Intervalofinteger {
    match *object {
        opt::CObject::ArchetypeInternalRef(ref o) => &o.occurrences,
        opt::CObject::ArchetypeSlot(ref o) => &o.occurrences,
        opt::CObject::ConstraintRef(ref o) => &o.occurrences,
        opt::CObject::CArchetypeRoot(ref o) => &o.occurrences,
        opt::CObject::CCodePhrase(ref o) => &o.occurrences,
        opt::CObject::CCodeReference(ref o) => &o.occurrences,
        opt::CObject::CComplexObject(ref o) => &o.occurrences,
        opt::CObject::CDefinedObject(ref o) => &o.occurrences,
        opt::CObject::CDvOrdinal(ref o) => &o.occurrences,
        opt::CObject::CDvQuantity(ref o) => &o.occurrences,
        opt::CObject::CDvState(ref o) => &o.occurrences,
        opt::CObject::CPrimitiveObject(ref o) => &o.occurrences,
        opt::CObject::TComplexObject(ref o) => &o.occurrences,
    }
}

/// The attributes `object` constrains, where it has any.
pub(crate) fn attributes(object: &opt::CObject) -> &[opt::CAttribute] {
    match *object {
        opt::CObject::CArchetypeRoot(ref o) => &o.attributes,
        opt::CObject::CComplexObject(ref o) => &o.attributes,
        opt::CObject::TComplexObject(ref o) => &o.attributes,
        _ => &[],
    }
}

/// The name and children of a `C_ATTRIBUTE`.
pub(crate) fn attribute_parts(attribute: &opt::CAttribute) -> (&str, &[opt::CObject]) {
    match *attribute {
        opt::CAttribute::CSingleAttribute(ref a) => (&a.rm_attribute_name, &a.children),
        opt::CAttribute::CMultipleAttribute(ref a) => (&a.rm_attribute_name, &a.children),
    }
}

/// Resolves `steps` against the subtree rooted at `root`, ignoring any leading
/// step that carries no attribute (the root predicate).
pub(crate) fn resolve<'a>(root: &'a opt::CObject, steps: &[PathStep]) -> Option<&'a opt::CObject> {
    let mut current = root;
    for step in steps {
        let Some(ref attribute_name) = step.attribute else {
            // A leading root predicate names the node the walk starts at.
            if !matches(current, step) {
                return None;
            }
            continue;
        };
        let mut next = None;
        for attribute in attributes(current) {
            let (name, children) = attribute_parts(attribute);
            if name != attribute_name {
                continue;
            }
            if let Some(child) = children.iter().find(|child| matches(child, step)) {
                next = Some(child);
                break;
            }
        }
        current = next?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::{PathStep, parse};

    #[test]
    fn a_name_predicate_does_not_split_the_step() {
        let steps = parse(
            "[openEHR-EHR-COMPOSITION.report.v1]\
             /content[openEHR-EHR-SECTION.adhoc.v1 and name/value='Reporting']\
             /items[at0005]",
        );
        assert_eq!(steps.len(), 3);
        assert_eq!(
            steps[1],
            PathStep {
                attribute: Some("content".to_owned()),
                node_id: None,
                archetype_id: Some("openEHR-EHR-SECTION.adhoc.v1".to_owned()),
                name: Some("Reporting".to_owned()),
            }
        );
        assert_eq!(steps[2].node_id.as_deref(), Some("at0005"));
    }

    #[test]
    fn a_specialised_at_code_is_a_node_id() {
        let steps = parse("/items[at0001.1]");
        assert_eq!(steps[0].node_id.as_deref(), Some("at0001.1"));
        assert!(steps[0].archetype_id.is_none());
    }
}
