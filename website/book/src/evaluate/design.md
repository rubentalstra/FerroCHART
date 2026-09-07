# The design

<!-- toc -->

The design of record is [`docs/architecture.md`][arch] in the repository. It
carries a decision per question with a citation to a primary source or the
explicit label that no specification governs it, a pin table, a build order,
and a decision register. This page is the shape of it, not a second copy.

## The model comes from published crates

The openEHR Reference Model, the archetype object model and the ADL parsers
come from the `openehr-*` crates on crates.io, which are generated from the
openEHR BMM schemas. The Reference Model BMM is complete enough to generate
from, and the ADL 1.4 constraint model is not, so the ADL 1.4 reader answers to
the published XML schemas instead.

## Both template generations, one internal model

FerroCHART reads ADL 1.4 operational templates and ADL 2 sources. The two
generations express the same clinical constraint through different classes, so
both readers normalize into one internal constraint model and the field
derivation is written once against that. Writing it twice would double the
surface where a form can admit what a template refuses.

## Layout lives in an overlay, keyed by node identity

The key is a chain of steps, each carrying the RM attribute name, the node id,
the archetype id where the child is an archetype root, the RM type, and the
pinned name where the template states one. Where all five tie, the step carries
a sibling ordinal and the entry is marked as positionally keyed.

That shape was measured rather than chosen. Walking 102 operational templates
found 427 sibling groups sharing one node id under one attribute, across 14 of
them. The pinned name is the only discriminator for 41.0% of those, the RM type
for 29.5%, the archetype id for 17.8%, and nothing at all for 7.0%.

Real openEHR tooling agrees. Nine of the 123 CKM templates FerroCHART vendors
carry a vendor layout section, and every one of its 42 entries is keyed by a
path carrying archetype ids, three of them by a name predicate too. A shipped
product keying layout to nodes reached the same conclusion from the other
direction.

## The overlay says what a person authored, and no more

Order, section, label, help text, default value, conditional visibility, the
widget asked for, and geometry. Nothing else, and nothing that belongs to the
template.

Geometry is a column grid. A section declares how many columns it has, an item
carries a span, an optional break, and an optional width in character units.
Nothing stores a row or a column index: placement comes from the order the
overlay already carries, plus the span, plus the break. That makes what a
reader sees and what a screen reader announces the same thing by construction,
which is what WCAG 2.2 asks for and what a stored coordinate breaks.

The reason it is a grid rather than free positioning is evidence rather than
taste. Of 31 form products surveyed, 25 store a device-independent layout, and
of the six that store coordinates, one is a design tool, two are deprecated,
two document the cost in their own documentation, and one has been redesigned
twice to escape it. Clinicians meet a form on a computer on wheels and on a
tablet in the same shift, so an authored form has to survive both.

## A recompile reports what happened to that work

Replay the overlay against the recompiled definition and every entry comes back
classified: matched, moved, disappeared, ambiguous, reordered, retyped, or a
node with no layout yet. A move is a suggestion a person accepts, never applied
silently. Nothing is discarded, so a revision that restores a node restores its
layout.

The report reads as prose rather than a diff: how many entries were kept, how
many need a decision, and for each of those what changed and what to do about
it.

## The compiler runs on the server

The server compiles the template and serves the form definition. The renderer
reads that definition and links no engine crate, so a third party can write
their own renderer against the published format.

## FerroCHART owns the field-level error

A clinician gets an error on the field they got wrong, and no CDR can supply
it: the openEHR ITS-REST error body is optional, conditional on a request
header, and carries no path to the node that failed. So FerroCHART validates
the composition against its operational template before it posts. A CDR
rejecting a composition FerroCHART built and validated is a FerroCHART defect.

[arch]: https://github.com/rubentalstra/FerroCHART/blob/main/docs/architecture.md
