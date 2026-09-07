// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The two path languages a validation message arrives in, and the one they
//! are normalized into.
//!
//! openEHR BASE Release-1.2.0 `architecture_overview.html` section 11 defines
//! the path syntax as a sequence of `attribute[predicate]` steps. Both
//! languages here are that syntax with a different predicate:
//!
//! - the archetype path, whose predicate is the node identifier the archetype
//!   states, optionally with the name the template pins;
//! - the Reference Model instance path, whose predicate is the position of the
//!   node in the document.
//!
//! A position says nothing about which template node the value came from, so
//! the second is normalized into the first by reading the reached node's
//! `archetype_node_id` out of the instance. openEHR RM Release-1.1.0
//! `common.html` section 3.2.2 fixes that attribute's value: the archetype
//! identifier in string form at an archetype root, and the node's own code
//! everywhere else, which is exactly the predicate the archetype path carries.

use serde_json::Value;

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The Reference Model attribute.
    pub attribute: String,
    /// The predicate, verbatim and without its brackets, where the step has
    /// one.
    pub predicate: Option<String>,
}

impl Step {
    /// The identifier the predicate names, dropping any pinned name.
    ///
    /// The web template writes a predicate as `[node_id]` or
    /// `[node_id,'pinned name']` (`openehr_its::flat::webtemplate::builder`),
    /// so the identifier is everything before the first comma.
    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        let predicate = self.predicate.as_deref()?;
        Some(match predicate.find(',') {
            Some(comma) => predicate.get(..comma).unwrap_or(predicate),
            None => predicate,
        })
    }

    /// Whether the predicate is a position rather than an identifier.
    #[must_use]
    pub fn is_positional(&self) -> bool {
        self.predicate.as_deref().is_some_and(|predicate| {
            !predicate.is_empty() && predicate.bytes().all(|b| b.is_ascii_digit())
        })
    }
}

/// Splits `path` into its steps.
///
/// A pinned name may hold a `/` (`items[at0004,'mmol/l']`), so the split
/// respects brackets rather than cutting on every separator.
#[must_use]
pub fn steps(path: &str) -> Vec<Step> {
    let mut out = Vec::new();
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
                    out.push(step(&current));
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        out.push(step(&current));
    }
    out
}

fn step(text: &str) -> Step {
    match (text.find('['), text.rfind(']')) {
        (Some(open), Some(close)) if close > open => Step {
            attribute: text.get(..open).unwrap_or_default().to_owned(),
            predicate: text
                .get(open.saturating_add(1)..close)
                .map(str::to_owned)
                .filter(|predicate| !predicate.is_empty()),
        },
        _ => Step {
            attribute: text.to_owned(),
            predicate: None,
        },
    }
}

/// Writes `steps` back as a path, keeping only the identifier of each
/// predicate.
///
/// The pinned name is dropped because the two producers disagree about when
/// to write one: the web template writes it wherever the template fixes a
/// single `name/value`, and the instance path never writes one at all.
/// Comparing on the identifier alone is what lets the two languages meet.
#[must_use]
pub fn write(steps: &[Step]) -> String {
    let mut out = String::new();
    for step in steps {
        out.push('/');
        out.push_str(&step.attribute);
        if let Some(identifier) = step.identifier() {
            out.push('[');
            out.push_str(identifier);
            out.push(']');
        }
    }
    out
}

/// Rewrites the positional predicates of `path` as the identifiers the
/// instance carries.
///
/// `instance` is the node `path` is relative to. A step whose node cannot be
/// reached, or whose node carries no `archetype_node_id`, keeps whatever
/// predicate it arrived with: an honest unresolvable path is better than an
/// invented one.
#[must_use]
pub fn against_instance(path: &str, instance: &Value) -> String {
    let mut out = Vec::new();
    let mut current = Some(instance);
    for step in steps(path) {
        let reached = current.and_then(|node| descend(node, &step));
        let predicate = reached
            .and_then(node_identifier)
            .map(str::to_owned)
            .or_else(|| {
                // A step that reached nothing keeps its own predicate, unless
                // that predicate is a position, which names no template node
                // and would resolve onto the wrong one.
                step.predicate.clone().filter(|_| !step.is_positional())
            });
        out.push(Step {
            attribute: step.attribute,
            predicate,
        });
        current = reached;
    }
    write(&out)
}

/// The node one step reaches from `node`.
fn descend<'v>(node: &'v Value, step: &Step) -> Option<&'v Value> {
    let attribute = node.get(&step.attribute)?;
    let Some(members) = attribute.as_array() else {
        return Some(attribute);
    };
    match step.predicate.as_deref() {
        Some(predicate) if step.is_positional() => members.get(predicate.parse::<usize>().ok()?),
        Some(_) => {
            let wanted = step.identifier()?;
            members
                .iter()
                .find(|member| node_identifier(member) == Some(wanted))
        }
        None => members.first(),
    }
}

/// The `archetype_node_id` a node carries, where it carries a non-empty one.
fn node_identifier(node: &Value) -> Option<&str> {
    node.get("archetype_node_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{against_instance, steps, write};

    #[test]
    fn a_pinned_name_holding_a_separator_does_not_split_a_step() {
        let parsed = steps("/data[at0001]/items[at0004,'mmol/l']/value");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[1].attribute, "items");
        assert_eq!(parsed[1].identifier(), Some("at0004"));
        assert_eq!(parsed[2].identifier(), None);
    }

    #[test]
    fn writing_back_keeps_the_identifier_and_drops_the_pinned_name() {
        assert_eq!(
            write(&steps(
                "/content[openEHR-EHR-OBSERVATION.x.v1,'Weight']/data[at0001]"
            )),
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]"
        );
    }

    #[test]
    fn a_positional_predicate_becomes_the_identifier_the_instance_carries() {
        // `common.html` section 3.2.2: at an archetype root the value is the
        // archetype identifier in string form, and the node's own code below
        // it. Reading it is what turns a position into a template node.
        let instance = json!({
            "content": [
                { "archetype_node_id": "openEHR-EHR-OBSERVATION.x.v1",
                  "data": { "archetype_node_id": "at0001" } }
            ]
        });
        assert_eq!(
            against_instance("/content[0]/data", &instance),
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]"
        );
    }

    #[test]
    fn a_step_that_reaches_nothing_keeps_an_identifier_and_loses_a_position() {
        let instance = json!({ "content": [] });
        assert_eq!(against_instance("/content[0]", &instance), "/content");
        assert_eq!(
            against_instance("/content[at0002]", &instance),
            "/content[at0002]"
        );
    }

    #[test]
    fn a_node_with_no_archetype_node_id_keeps_the_path_unpredicated() {
        // An `EVENT_CONTEXT` carries no `archetype_node_id` of its own, so
        // there is nothing to write and nothing to invent.
        let instance = json!({ "context": { "setting": {} } });
        assert_eq!(
            against_instance("/context/setting", &instance),
            "/context/setting"
        );
    }
}
