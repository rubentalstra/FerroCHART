# Write your own renderer

<!-- toc -->

FerroCHART publishes the form definition as a document, and everything the
renderer in this repository does with it, you can do. This page is the
contract: what the document contains, what a client owes it, and where the
line between the two sits.

No specification governs any of it. openEHR defines the operational template
the definition is compiled from and the COMPOSITION it produces, and says
nothing about the shape in between. That shape is FerroCHART's own design, so
it is published, versioned, and described here rather than left to be read out
of the source.

## Why the format exists at all

A form builder that only its own front end can read is a front end with a
storage format. The reason this one is a document is that a hospital already
running a user interface should be able to keep it, and a form is not worth
rewriting a clinical system over.

So the engine crates never enter a browser. `scripts/checks/crate-closure.sh`
reads the resolved dependency graph and fails when the renderer links anything
of this tree but `ferrochart-form`, which is the crate that holds the
published types and no I/O. That check exists to keep this promise honest
rather than aspirational: if the renderer needed the compiler, so would you.

## The three documents

| Document | Direction | Rust type |
|---|---|---|
| the form definition | the server writes, a client reads | `ferrochart_form::definition::FormDefinition` |
| the entered values | a client writes, the server reads | `ferrochart_form::values::FormValues` |
| the validation report | the server writes, a client reads | `ferrochart_form::validation::ValidationReport` |

All three state `format_version`, and all three move together. A client reads
the number first and refuses a document it does not know, rather than guessing
at a shape that may have changed underneath it.

**The current version is 2.** Version 1 tagged its enums internally; version 2
tags them externally, which is the shape described below.

## Reading a definition

A definition is a tree. `root` is a group, a group holds `items`, and an item
is either a nested group or a field:

```json
{
  "format_version": 2,
  "template_id": "Ferro wire probe",
  "default_language": "en",
  "languages": ["en"],
  "root": { "key": { … }, "items": [ { "group": { … } }, { "field": { … } } ] }
}
```

Every enum is **externally tagged**: the variant is the member name and its
payload is the value. So an item is `{"group": {…}}` or `{"field": {…}}`, and
a field's kind is `{"quantity": {…}}` rather than a `kind` member beside the
payload. There are eighteen field kinds, one per row of the derivation table,
and a client that meets one it does not know should say so on the screen
rather than skip the field.

A field carries what it collects and what it admits:

```json
{
  "rm_type": "DV_QUANTITY",
  "label": { "en": "Body temperature" },
  "occurrences": { "minimum": 0, "maximum": 1 },
  "kind": { "quantity": { "property": …, "units": [ … ] } }
}
```

**The constraint is the point.** A quantity states its permitted units, and
each unit states its own magnitude range and its own decimal precision, so
changing the unit changes both. A coded field states its value set. A text
field states whether its list of options is the whole permitted set or a
suggestion. A client that admits more than the constraint states will build a
COMPOSITION the CDR rejects, and that rejection is the client's defect.

`label` and `help` are maps from language tag to text. Where a template states
nothing, the map is empty, and a client falls back to something a reader can
act on rather than drawing a blank heading.

## The key, and why it looks like that

Every group and field carries a `key`: a chain of steps from the template
root, each naming the Reference Model attribute, the node id, the archetype
id, the Reference Model class and the pinned name.

That is more than an identifier needs, and the reason is measured rather than
theoretical. Walking 102 real operational templates showed an id-only path is
not unique inside one template, so a key built from ids alone binds the wrong
node. `is_positional` marks a key that a reordering of the template would
break, which is the case a layout overlay has to report rather than silently
rebind.

## Writing values

`FormValues` is a JSON array, not an object, because its entries are keyed by
a structure rather than a string. Each entry addresses one control:

```json
[{
  "key": { "steps": [ … ], "is_positional": false },
  "group_path": [1],
  "occurrence": 0,
  "entered": { "value": { "quantity": { "magnitude": 37.2, "units": "Cel" } } }
}]
```

`group_path` is the occurrence of each **repeating** group above the field,
outermost first, and `occurrence` is which repeat of the field itself. Both
are needed: a repeating group produces several data nodes sharing one
`archetype_node_id`, and without the path a value entered in the second
occurrence is indistinguishable from one entered in the first. 67 of the 121
templates in the committed corpus have at least one repeating group, so this
is the common case rather than an edge.

`entered` is `{"value": …}` or `{"null": {"code": …, "reason": …}}` and never
both. openEHR RM Release-1.1.0 `data_structures.html` section 5.2.3 gives
`ELEMENT` the invariant `Inv_null_flavour_indicated`, so an element carries
exactly one of a value and a null flavour. The type is that invariant.

## Placing a refusal

A `ValidationReport` is a list of failures. Each carries the `key` of the item
it belongs to, so a client draws it beside that control with no path
arithmetic of its own.

Two members decide where exactly:

- `at` states the occurrence, where the judgement knows one. A refusal from
  the composition builder carries it, because the builder walks the form with
  that address in hand. Draw such a failure on that control alone.
- `at` is absent where the judgement does not know. A refusal from the
  operational template carries a Reference Model path whose positional
  predicates do not translate to a form's occurrence path, so it states none
  rather than inventing one. Draw such a failure on every repeat of its field.

A failure whose `key` is absent resolved to no item of the form. **Show it
anyway.** Losing a refusal is worse than showing it in the wrong place, and a
client that drops one will eventually drop the one that mattered.

## What the server does that you should not reimplement

Validation. A client may check what it can as a courtesy, and the server
judges the entered values against the operational template before any
COMPOSITION is built, because that is where the template actually is. The
routes are in [Configuration](../operate/configuration.md).

Building the COMPOSITION. The mapping from entered values to Reference Model
instances is `ferrochart-compose`, it is the part openEHR governs in detail,
and a second implementation of it is a second place for a clinical document to
go wrong.

## Where the contract can change

`FORMAT_VERSION` covers the bytes you parse: member names, the tag and variant
names of every enum, the shape of every value, and which members a document is
guaranteed to carry. Two changes are deliberately not a version bump, because
a client written against the old bytes still reads what it read before: a new
member you may ignore, and a new variant of an enum the crate already marks
`#[non_exhaustive]`.

So write a client that ignores members it does not recognise, and that says
something visible when it meets a variant it does not know. Both will happen.
