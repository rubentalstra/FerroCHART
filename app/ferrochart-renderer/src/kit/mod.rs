// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The design system, one module per concern.
//!
//! No specification governs this: our own design. A look is a class constant
//! and a behaviour is a component, and each affordance has exactly ONE
//! definition here. A screen composes these; a screen that hand-rolls the
//! markup instead is how two buttons start disagreeing.
//!
//! Every constant styles against the semantic tokens of
//! `style/tailwind.css`. None of them names a focus ring: one
//! `:focus-visible` rule in that stylesheet's base layer draws every
//! indicator, so a control cannot ship without one.

pub(crate) mod field;
pub(crate) mod page_header;
pub(crate) mod surface;
