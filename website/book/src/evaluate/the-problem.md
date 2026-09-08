# The problem

<!-- toc -->

## A template is not a screen

An openEHR operational template says what a valid COMPOSITION contains: which
nodes exist, what type each carries, how many times a node may repeat, and
which codes a coded field admits. It says nothing about what a person sees.

Deriving the mechanical half is deterministic. A `DV_QUANTITY` becomes a number
with its permitted units, a `DV_CODED_TEXT` becomes a selection over its value
set, a `DV_DATE_TIME` becomes a date field at the right precision, and a
`CLUSTER` whose upper occurrence exceeds one becomes a repeatable group. No
human is needed for any of it. [The renderer](../operate/renderer.md) shows
what one template becomes.

## The half no specification governs

Field order, grouping, labels, help text, defaults, conditional visibility and
widget choice are not in the Reference Model, not in the Archetype Object
Model, and not in the operational template specifications. Twelve such cases
are listed in the architecture document, each one a decision a form builder
must make with no specification behind it.

A person spends hours on that work. Then the template is revised.

## What happens next is the whole product

Every openEHR form tool surveyed for this project loses that work, hides it, or
puts it somewhere it does not belong:

- Storing layout in the rendered artefact means nothing replays and nothing is
  reported when the template changes.
- Storing layout in the template's annotations means a form decision travels to
  every other consumer of that clinical model.
- Regenerating from the definition, as HL7 FHIR Structured Data Capture does
  through `$assemble`, gives no replay and no report of what went missing.

None of them tells you what a recompile did to the layout you authored.

FerroCHART stores layout in a separate overlay keyed by node identity. A
recompile from the revised template replays the overlay and reports, per entry,
what matched, what disappeared, what moved, what became ambiguous and what
changed type. Nothing is discarded, and nothing is silently rebound.

That report is the product.

It is built. A layout authored on all 4,447 nodes of the 121 templates this
repository vendors replays with everything matched and nothing lost, and the
same corpus carries the collision the key was designed for: 102 nodes across
four templates where siblings share a node id and differ in nothing a key can
see, one of them two identically named branches under a single event.
