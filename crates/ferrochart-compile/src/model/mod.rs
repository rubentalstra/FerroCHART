// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The internal constraint model: the narrow waist both template readers fill.
//!
//! FerroCHART reads both operational template generations, and the two do not
//! share a constraint model. ADL 1.4 constrains a quantity with the
//! `C_DV_QUANTITY` domain type; AOM 2 constrains the same clinical fact with a
//! `C_COMPLEX_OBJECT` over `magnitude` and `units` tied by a
//! `C_PRIMITIVE_TUPLE` (openEHR AM Release-2.3.0 `AOM2.html` sections 4.3.1
//! and 4.5.25). Ordinals, coded text, value sets, temporal patterns and node
//! identity all differ the same way.
//!
//! No specification governs this model; it is FerroCHART's own design. Each
//! reader owes it, per node, a node identity, a Reference Model type, an
//! occurrences interval, a constraint payload, and the terminology in scope.
//! Everything above this point is generation-blind: nothing here records which
//! reader produced a node, and an absent fact means the template did not state
//! it rather than that the generation could not.

pub mod ids;
pub mod multiplicity;
pub mod node;
pub mod payload;
pub mod terminology;
