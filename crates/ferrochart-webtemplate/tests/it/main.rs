// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The web template compatibility surface, tested against real documents.
//!
//! Every web template a test reads is built from the committed CKM
//! operational template pack, so nothing here is asserted against an invented
//! document. The web template is a compatibility target and not a specified
//! format, so a test that pins its shape pins what the published
//! implementations write, never a conformance requirement.

mod corpus;
mod from_form;
mod preservation;
mod refusals;
mod support;
mod version;
