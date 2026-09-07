// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Validates a COMPOSITION against its operational template, before any
//! request reaches a CDR (`docs/architecture.md` section 9).
//!
//! # Why the gate exists
//!
//! openEHR ITS-REST Release-1.1.0 `overview.html` makes a CDR's error detail
//! optional ("services MAY return additional error details"), conditional on
//! the client having sent `Prefer: return=representation`, and shapes its
//! `errors` array as `DV_CODED_TEXT` entries whose codes are in
//! `terminology_id: "local"`. `DV_CODED_TEXT` has no path attribute, so the
//! wire carries no pointer to the node that failed and the codes are
//! vendor-defined. A clinician cannot be shown the field they got wrong from
//! that, so FerroCHART judges the document itself and owns the field-level
//! error.
//!
//! # What it does not do
//!
//! It never posts. The gate is composed by the caller that holds both a
//! validator and a client, and `ferrochart-server` is the crate that does so
//! for the product. Keeping the two apart is what lets `ferrochart-cdr` go on
//! linking `ferrochart-form` and nothing else
//! (`scripts/checks/crate-closure.sh`).
//!
//! # The path claim of section 9, corrected
//!
//! `docs/architecture.md` section 9 said `openehr_its::rm_instance` returns
//! messages "keyed by the same path string the compiler emitted, which is a
//! field-level error with no adapter in between". Measured against the
//! committed template pack, that is not so, and [`mod@resolve`] is the adapter
//! it said was unnecessary. Three path languages meet here:
//!
//! 1. FerroCHART's [`ferrochart_form::key::NodeKey`], whose step carries the
//!    Reference Model attribute, the node id, the archetype id, the Reference
//!    Model type and the pinned name;
//! 2. the archetype `aqlPath` the template-driven pass reports, which carries
//!    the attribute, one identifier and an optional pinned name, starts below
//!    the template root, and descends past an `ELEMENT` into the attribute
//!    holding its value;
//! 3. the Reference Model instance path the instance passes report, whose
//!    predicates are numeric positions in the document.
//!
//! The adapter normalizes 3 into 2 by reading each reached node's
//! `archetype_node_id` out of the instance, then resolves 2 onto 1 through an
//! index built from the form definition. What cannot be resolved keeps its
//! reported path and is reported unplaced, never dropped.
//!
//! The renderer's side of the claim does hold: what reaches a renderer is a
//! [`ferrochart_form::validation::ValidationReport`] keyed by `NodeKey`, which
//! it resolves against the form definition it is already holding.

pub mod error;
pub mod index;
pub mod path;
pub mod resolve;
pub mod template;
pub mod vendor;
