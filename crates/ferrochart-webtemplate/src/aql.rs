// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The last step of a node's `aqlPath`, which is where its Reference Model
//! attribute is stated.
//!
//! A web template node states its path from the template root and nothing
//! else about where it sits, so the attribute a node hangs under is read off
//! the end of that path. The spelling is the AQL path predicate of openEHR
//! QUERY Release-1.1.0 section 3.6.3, `attribute[predicate]`, whose predicate
//! carries an archetype node identifier and may carry a name.
//!
//! A predicate is skipped rather than parsed: the node's own `nodeId` and
//! `name` members state the same two facts, and they are the members this
//! crate models.

/// The Reference Model attribute the node at `aql_path` hangs under.
///
/// The empty string for the tree root, which a web template states with an
/// empty `aqlPath` and which hangs under no attribute of anything.
pub(crate) fn last_attribute(aql_path: &str) -> &str {
    let Some(last) = segments(aql_path).last() else {
        return "";
    };
    match last.find('[') {
        Some(open) => last.get(..open).unwrap_or(last),
        None => last,
    }
}

/// The `/`-separated segments of an AQL path, with a `/` inside a predicate
/// left alone.
///
/// A predicate holds an archetype identifier and a quoted name, both of which
/// may carry a `/`, so the split counts brackets and quotes rather than
/// splitting on every slash.
fn segments(aql_path: &str) -> impl Iterator<Item = &str> {
    let mut depth = 0_usize;
    let mut quoted = false;
    let mut start = 0_usize;
    let mut out = Vec::new();
    for (index, character) in aql_path.char_indices() {
        match character {
            '\'' => quoted = !quoted,
            '[' if !quoted => depth = depth.saturating_add(1),
            ']' if !quoted => depth = depth.saturating_sub(1),
            '/' if !quoted && depth == 0 => {
                if let Some(segment) = aql_path.get(start..index)
                    && !segment.is_empty()
                {
                    out.push(segment);
                }
                start = index.saturating_add(1);
            }
            _ => {}
        }
    }
    if let Some(segment) = aql_path.get(start..)
        && !segment.is_empty()
    {
        out.push(segment);
    }
    out.into_iter()
}

#[cfg(test)]
mod tests {
    use super::last_attribute;

    #[test]
    fn the_root_hangs_under_no_attribute() {
        assert_eq!(last_attribute(""), "");
        assert_eq!(last_attribute("/"), "");
    }

    #[test]
    fn a_leaf_hangs_under_the_attribute_its_path_ends_with() {
        assert_eq!(
            last_attribute(
                "/content[openEHR-EHR-EVALUATION.x.v1]/data[at0001]/items[at0002]/value"
            ),
            "value"
        );
        assert_eq!(last_attribute("/context/start_time"), "start_time");
    }

    #[test]
    fn a_slash_inside_a_predicate_does_not_start_a_segment() {
        // A pinned name is quoted in an AQL node predicate (openEHR QUERY
        // Release-1.1.0 section 3.6.3), and clinical names carry slashes.
        assert_eq!(
            last_attribute("/items[openEHR-EHR-CLUSTER.x.v1,'Height/length']"),
            "items"
        );
    }
}
