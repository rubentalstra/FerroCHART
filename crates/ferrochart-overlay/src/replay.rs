// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The replay, and the report a template revision produces.
//!
//! No specification says an openEHR path is stable across template revisions.
//! AQL Release-1.1.0, openEHR BASE Release-1.2.0
//! `architecture_overview.html` sections 10.5, 10.6 and 11, and openEHR AM
//! Release-2.3.0 `AOM1.4.html` section 4.2.3.1 and `AOM2.html` section 4.2.3.2
//! are all silent, while AOM 2 section 7.2.1 makes `id25.1` a specialisation
//! of `id25`, so a revision provably changes codes. Recompiling and reporting
//! per entry is therefore the only correct design, and the report is the
//! product: no implementation surveyed for this project says what a recompile
//! did to hand-authored layout.
//!
//! Nothing here changes the overlay. An entry that resolves to nothing is
//! retained against its old key, so a later revision that restores the node
//! restores its layout, and a move is a suggestion a person accepts through
//! [`crate::store::Author::accept_move`].

use std::collections::BTreeSet;
use std::fmt;

use ferrochart_form::definition::FormDefinition;
use ferrochart_form::ids::{LanguageTag, RmTypeName, TemplateId};
use ferrochart_form::key::{KeyStep, NodeKey};
use ferrochart_form::text::Localized;

use crate::entry::OverlayEntry;
use crate::index::{Index, Target, TargetKind};
use crate::store::{Overlay, TemplateIdForm};

/// How many nodes a report lists one by one before it counts the rest.
const LISTED: usize = 10;

/// What became of one overlay entry.
///
/// The seven outcomes are the classification of `docs/architecture.md`
/// section 6.5. Four of them need a person: a move, an ambiguity, a
/// reordering and a retype are all reported and never applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// The key names one node of the same Reference Model type.
    Matched {
        /// The node, as the recompiled definition keys it.
        at: NodeKey,
    },
    /// The key names several nodes, and nothing in the template tells them
    /// apart.
    ///
    /// The five-part step tuple tied. Which node the layout belongs to is a
    /// question only a person can answer.
    Ambiguous {
        /// The nodes the key names, in key order.
        candidates: Vec<NodeKey>,
    },
    /// The entry is positionally keyed and the siblings it was anchored to
    /// changed.
    ///
    /// A position is what a reordering breaks without a trace, so the entry is
    /// reported rather than followed.
    Reordered {
        /// The nodes the key now names, in key order.
        candidates: Vec<NodeKey>,
    },
    /// The key names one node, and what it collects changed, so the layout
    /// may no longer mean anything.
    ///
    /// A widget chosen for a number says nothing about a coded field, so a
    /// class change is reported whether it shows in the key or only in what
    /// the node collects.
    Retyped {
        /// The node, as the recompiled definition keys it.
        at: NodeKey,
        /// The class the layout was authored against.
        was: RmTypeName,
        /// The class the node collects now.
        now: RmTypeName,
    },
    /// The key names nothing, and exactly one node elsewhere carries the same
    /// terminal node id and Reference Model type.
    ///
    /// A suggestion, never an automatic rebinding: rebinding layout onto a
    /// node a person did not choose is how layout ends up on the wrong
    /// clinical concept.
    Moved {
        /// The node the layout would move to.
        to: NodeKey,
    },
    /// The key names nothing, and nothing recognizable is anywhere else.
    ///
    /// The entry is retained against its old key.
    Disappeared,
}

impl Outcome {
    /// Whether the outcome is one a person has to decide.
    #[must_use]
    pub const fn needs_decision(&self) -> bool {
        matches!(
            *self,
            Self::Ambiguous { .. }
                | Self::Reordered { .. }
                | Self::Retyped { .. }
                | Self::Moved { .. }
        )
    }

    /// The one word a report labels the outcome with.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match *self {
            Self::Matched { .. } => "matched",
            Self::Ambiguous { .. } => "ambiguous",
            Self::Reordered { .. } => "reordered",
            Self::Retyped { .. } => "retyped",
            Self::Moved { .. } => "moved",
            Self::Disappeared => "disappeared",
        }
    }
}

/// What became of one node a layout reads.
///
/// A visibility rule reads another node's answer, and that reference is a key
/// like any other. A rule reading a node the revision removed cannot be
/// evaluated, so it is reported beside the entry that carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceOutcome {
    /// The reference names one node.
    Resolved {
        /// The node, as the recompiled definition keys it.
        at: NodeKey,
    },
    /// The reference names several nodes.
    Ambiguous {
        /// How many nodes it names.
        count: usize,
    },
    /// The reference names nothing.
    Disappeared,
}

/// One node a layout reads, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceReport {
    /// The node the layout reads.
    pub node: NodeKey,
    /// What became of it.
    pub outcome: ReferenceOutcome,
}

/// What became of one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryReport {
    /// The key the entry is stored against.
    pub key: NodeKey,
    /// The text a person knows the node by.
    ///
    /// The authored label where the entry carries one, and the label the
    /// definition derived from the archetype otherwise.
    pub label: Localized,
    /// What became of the entry.
    pub outcome: Outcome,
    /// What became of every node the layout reads.
    pub references: Vec<ReferenceReport>,
}

/// Whether a node is a group or a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeKind {
    /// A group of the form.
    Group,
    /// A field a clinician fills.
    Field,
}

/// A node of the recompiled definition that no entry decorates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNode {
    /// What names the node.
    pub key: NodeKey,
    /// The text the definition labels it with.
    pub label: Localized,
    /// Whether it is a group or a field.
    pub kind: NodeKind,
}

/// How many entries fell into each outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplaySummary {
    /// Entries whose key still names one node of the same class.
    pub matched: usize,
    /// Entries whose key names several nodes.
    pub ambiguous: usize,
    /// Entries whose anchored siblings changed.
    pub reordered: usize,
    /// Entries whose node changed class.
    pub retyped: usize,
    /// Entries with one move suggestion.
    pub moved: usize,
    /// Entries whose node is nowhere in the recompiled definition.
    pub disappeared: usize,
    /// Nodes of the recompiled definition that no entry decorates.
    pub appeared: usize,
}

impl ReplaySummary {
    /// How many entries need a person.
    #[must_use]
    pub const fn needing_decision(&self) -> usize {
        self.ambiguous
            .saturating_add(self.reordered)
            .saturating_add(self.retyped)
            .saturating_add(self.moved)
    }

    /// How many entries were replayed.
    #[must_use]
    pub const fn entries(&self) -> usize {
        self.matched
            .saturating_add(self.needing_decision())
            .saturating_add(self.disappeared)
    }
}

/// What a recompile did to one overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayReport {
    template_was: TemplateId,
    template_now: TemplateId,
    template_id_form: TemplateIdForm,
    language: LanguageTag,
    entries: Vec<EntryReport>,
    appeared: Vec<NewNode>,
}

impl ReplayReport {
    /// The template the overlay was keyed against.
    #[must_use]
    pub fn template_was(&self) -> &TemplateId {
        &self.template_was
    }

    /// The template the definition was compiled from.
    #[must_use]
    pub fn template_now(&self) -> &TemplateId {
        &self.template_now
    }

    /// The form the overlay's template identifier takes.
    #[must_use]
    pub const fn template_id_form(&self) -> TemplateIdForm {
        self.template_id_form
    }

    /// Whether the overlay and the definition name one template.
    #[must_use]
    pub fn is_same_template(&self) -> bool {
        self.template_was == self.template_now
    }

    /// Every entry, in key order.
    #[must_use]
    pub fn entries(&self) -> &[EntryReport] {
        &self.entries
    }

    /// Every node no entry decorates, in key order.
    #[must_use]
    pub fn appeared(&self) -> &[NewNode] {
        &self.appeared
    }

    /// Every entry a person has to decide, in key order.
    pub fn needing_decision(&self) -> impl Iterator<Item = &EntryReport> {
        self.entries
            .iter()
            .filter(|entry| entry.outcome.needs_decision())
    }

    /// How many entries fell into each outcome.
    #[must_use]
    pub fn summary(&self) -> ReplaySummary {
        let mut summary = ReplaySummary {
            appeared: self.appeared.len(),
            ..ReplaySummary::default()
        };
        for entry in &self.entries {
            let counter = match entry.outcome {
                Outcome::Matched { .. } => &mut summary.matched,
                Outcome::Ambiguous { .. } => &mut summary.ambiguous,
                Outcome::Reordered { .. } => &mut summary.reordered,
                Outcome::Retyped { .. } => &mut summary.retyped,
                Outcome::Moved { .. } => &mut summary.moved,
                Outcome::Disappeared => &mut summary.disappeared,
            };
            *counter = counter.saturating_add(1);
        }
        summary
    }

    /// The text of one label, in the form's own language.
    ///
    /// A node the recompiled definition no longer holds has no label to read,
    /// and its key is what a person has left to recognize it by.
    fn read(&self, label: &Localized, key: &NodeKey) -> String {
        label
            .get(&self.language)
            .or_else(|| label.languages().next().and_then(|tag| label.get(tag)))
            .map_or_else(|| key.to_string(), ToOwned::to_owned)
    }

    /// One listed node: its label and its key, or its key alone where there is
    /// no label to read.
    fn named(&self, label: &Localized, key: &NodeKey) -> String {
        let text = self.read(label, key);
        let path = key.to_string();
        if text == path {
            return path;
        }
        format!("{text} ({path})")
    }
}

impl fmt::Display for ReplayReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self.summary();
        writeln!(f, "Layout replay for template \"{}\"", self.template_now)?;
        if !self.is_same_template() {
            writeln!(
                f,
                "The overlay was keyed against \"{}\", which is a different template.",
                self.template_was
            )?;
        }
        if !self.template_id_form.pins_one_release() {
            writeln!(
                f,
                "The template identifier names no release, so which revision this is \
                 depends on what the server held."
            )?;
        }
        writeln!(
            f,
            "{}: {} kept, {} needing a decision, {} no longer in the form.",
            plural(summary.entries(), "authored entry", "authored entries"),
            summary.matched,
            summary.needing_decision(),
            summary.disappeared
        )?;
        writeln!(
            f,
            "{} with no layout yet.",
            plural(summary.appeared, "node", "nodes")
        )?;
        self.decisions(f)?;
        self.gone(f)?;
        self.kept(f)?;
        self.fresh(f)
    }
}

impl ReplayReport {
    fn decisions(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pending: Vec<&EntryReport> = self.needing_decision().collect();
        if pending.is_empty() {
            return Ok(());
        }
        writeln!(f, "\nNeeds a decision ({})", pending.len())?;
        for entry in pending {
            let label = self.read(&entry.label, &entry.key);
            writeln!(f, "  {} {label}", entry.outcome.name())?;
            match entry.outcome {
                Outcome::Moved { ref to } => {
                    writeln!(f, "      was {}", entry.key)?;
                    writeln!(f, "      now {to}")?;
                    writeln!(
                        f,
                        "      One node elsewhere carries the same node id and class. \
                         Accept the move to take the layout there."
                    )?;
                }
                Outcome::Retyped {
                    ref at,
                    ref was,
                    ref now,
                } => {
                    writeln!(f, "      at {at}")?;
                    writeln!(
                        f,
                        "      The node collects {now} where it collected {was}, \
                         so this layout may no longer mean anything."
                    )?;
                }
                Outcome::Ambiguous { ref candidates } => {
                    writeln!(f, "      at {}", entry.key)?;
                    writeln!(
                        f,
                        "      {} nodes carry this key and the template tells them apart \
                         by nothing. Choose the one this layout belongs to.",
                        candidates.len()
                    )?;
                }
                Outcome::Reordered { ref candidates } => {
                    writeln!(f, "      at {}", entry.key)?;
                    writeln!(
                        f,
                        "      This layout was placed by position among {} lookalike nodes, \
                         and they are not the ones it was placed against. Confirm where it goes.",
                        candidates.len()
                    )?;
                }
                Outcome::Matched { .. } | Outcome::Disappeared => {}
            }
            Self::dangling(f, entry)?;
        }
        Ok(())
    }

    fn dangling(f: &mut fmt::Formatter<'_>, entry: &EntryReport) -> fmt::Result {
        for reference in &entry.references {
            match reference.outcome {
                ReferenceOutcome::Resolved { .. } => {}
                ReferenceOutcome::Ambiguous { count } => writeln!(
                    f,
                    "      Its visibility rule reads {}, which now names {count} nodes.",
                    reference.node
                )?,
                ReferenceOutcome::Disappeared => writeln!(
                    f,
                    "      Its visibility rule reads {}, which is no longer in the form.",
                    reference.node
                )?,
            }
        }
        Ok(())
    }

    fn gone(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let gone: Vec<&EntryReport> = self
            .entries
            .iter()
            .filter(|entry| entry.outcome == Outcome::Disappeared)
            .collect();
        if gone.is_empty() {
            return Ok(());
        }
        writeln!(
            f,
            "\nNo longer in the form ({}), kept in case a later revision brings them back",
            gone.len()
        )?;
        for entry in gone.iter().take(LISTED) {
            writeln!(f, "  {}", self.named(&entry.label, &entry.key))?;
        }
        remainder(f, gone.len(), "entries")
    }

    fn kept(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kept = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.outcome, Outcome::Matched { .. }))
            .count();
        if kept == 0 {
            return Ok(());
        }
        writeln!(f, "\nKept, nothing to do ({kept})")
    }

    fn fresh(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.appeared.is_empty() {
            return Ok(());
        }
        writeln!(f, "\nNo layout yet ({})", self.appeared.len())?;
        for node in self.appeared.iter().take(LISTED) {
            writeln!(f, "  {}", self.named(&node.label, &node.key))?;
        }
        remainder(f, self.appeared.len(), "nodes")
    }
}

/// A count with the noun that agrees with it.
fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        return format!("1 {one}");
    }
    format!("{count} {many}")
}

/// The tail of a list the report only counted.
fn remainder(f: &mut fmt::Formatter<'_>, total: usize, noun: &str) -> fmt::Result {
    let rest = total.saturating_sub(LISTED);
    if rest == 0 {
        return Ok(());
    }
    writeln!(f, "  and {rest} more {noun}")
}

/// Classifies every entry of `overlay` against `definition`.
///
/// The overlay is not changed. Nothing is discarded, a move is a suggestion,
/// and replaying the same overlay against the same definition twice gives the
/// same report.
#[must_use]
pub fn replay(overlay: &Overlay, definition: &FormDefinition) -> ReplayReport {
    let index = Index::new(definition);
    let mut entries = Vec::with_capacity(overlay.entries().len());
    let mut decorated: BTreeSet<Vec<KeyStep>> = BTreeSet::new();
    for entry in overlay.entries() {
        let outcome = classify(&index, entry);
        match outcome {
            Outcome::Matched { ref at } | Outcome::Retyped { ref at, .. } => {
                decorated.insert(at.steps.clone());
            }
            Outcome::Moved { ref to } => {
                decorated.insert(to.steps.clone());
            }
            Outcome::Ambiguous { ref candidates } | Outcome::Reordered { ref candidates } => {
                decorated.extend(candidates.iter().map(|key| key.steps.clone()));
            }
            Outcome::Disappeared => {}
        }
        let label = if entry.layout.label.is_empty() {
            label_of(&index, &outcome, &entry.key)
        } else {
            entry.layout.label.clone()
        };
        entries.push(EntryReport {
            key: entry.key.clone(),
            label,
            outcome,
            references: references(&index, entry.layout.referenced_nodes()),
        });
    }
    let appeared = appeared(&index, &decorated);
    ReplayReport {
        template_was: overlay.template().id.clone(),
        template_now: definition.template_id.clone(),
        template_id_form: overlay.template().id_form,
        language: definition.default_language.clone(),
        entries,
        appeared,
    }
}

/// What became of one entry.
fn classify(index: &Index<'_>, entry: &OverlayEntry) -> Outcome {
    let key = &entry.key;
    let candidates = index.resolve(key);
    if let Some(anchor) = entry.anchor.as_ref() {
        let siblings: Vec<String> = candidates.iter().map(Target::signature).collect();
        let reordered = || Outcome::Reordered {
            candidates: keys(&candidates),
        };
        if siblings != anchor.tied_siblings {
            return reordered();
        }
        // NOTE: no specification governs this: our own design. The siblings
        // are the ones the person saw, so the ordinal still names the node it
        // named then.
        return candidates
            .iter()
            .find(|target| target.key().steps == key.steps)
            .map_or_else(reordered, |target| resolved(&entry.rm_type, *target));
    }
    match candidates.as_slice() {
        [] => unresolved(index, entry),
        [only] => resolved(&entry.rm_type, *only),
        several => Outcome::Ambiguous {
            candidates: keys(several),
        },
    }
}

/// What became of an entry whose key names exactly one node.
fn resolved(was: &RmTypeName, target: Target<'_>) -> Outcome {
    if *was == *target.rm_type() {
        return Outcome::Matched {
            at: target.key().clone(),
        };
    }
    Outcome::Retyped {
        at: target.key().clone(),
        was: was.clone(),
        now: target.rm_type().clone(),
    }
}

/// What became of an entry whose key names no node at all.
fn unresolved(index: &Index<'_>, entry: &OverlayEntry) -> Outcome {
    let retyped = index.resolve_retyped(&entry.key);
    if let [only] = retyped.as_slice() {
        return Outcome::Retyped {
            at: only.key().clone(),
            was: entry.rm_type.clone(),
            now: only.rm_type().clone(),
        };
    }
    if retyped.len() > 1 {
        return Outcome::Ambiguous {
            candidates: keys(&retyped),
        };
    }
    let moved = index.resolve_moved(&entry.key);
    if let [only] = moved.as_slice() {
        return Outcome::Moved {
            to: only.key().clone(),
        };
    }
    Outcome::Disappeared
}

/// What became of every node a layout reads.
fn references(index: &Index<'_>, nodes: Vec<&NodeKey>) -> Vec<ReferenceReport> {
    nodes
        .into_iter()
        .map(|node| {
            let found = index.resolve(node);
            let outcome = match found.as_slice() {
                [] => ReferenceOutcome::Disappeared,
                [only] => ReferenceOutcome::Resolved {
                    at: only.key().clone(),
                },
                several => ReferenceOutcome::Ambiguous {
                    count: several.len(),
                },
            };
            ReferenceReport {
                node: node.clone(),
                outcome,
            }
        })
        .collect()
}

/// The label the definition carries for whatever an outcome resolved to.
fn label_of(index: &Index<'_>, outcome: &Outcome, key: &NodeKey) -> Localized {
    let at = match *outcome {
        Outcome::Matched { ref at } | Outcome::Retyped { ref at, .. } => at,
        Outcome::Moved { ref to } => to,
        Outcome::Ambiguous { .. } | Outcome::Reordered { .. } | Outcome::Disappeared => key,
    };
    index
        .resolve(at)
        .first()
        .map_or_else(Localized::empty, |target| target.label().clone())
}

/// Every node of the definition no entry accounted for.
fn appeared(index: &Index<'_>, decorated: &BTreeSet<Vec<KeyStep>>) -> Vec<NewNode> {
    index
        .targets()
        .iter()
        .filter(|target| !decorated.contains(&target.key().steps))
        .map(|target| NewNode {
            key: target.key().clone(),
            label: target.label().clone(),
            kind: match target.kind() {
                TargetKind::Group => NodeKind::Group,
                TargetKind::Field => NodeKind::Field,
            },
        })
        .collect()
}

/// The keys of a set of nodes, in the order the index holds them.
fn keys(targets: &[Target<'_>]) -> Vec<NodeKey> {
    targets.iter().map(|target| target.key().clone()).collect()
}
