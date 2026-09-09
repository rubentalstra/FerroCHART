<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Maintenance rule: every pull request that changes user-visible behaviour adds
an entry under **[Unreleased]** in the same PR. Cutting a release renames
[Unreleased] to the version and date, and adds a fresh link reference.

The 0.0.x line carries the design of record, the repository and its gates, and
the engine as it is built. `docs/architecture.md` is the design, and
`docs/architecture.md` section 14 is the build order each milestone follows.
A release publishes binaries for four Linux targets; what they do grows with
the build order.

## [Unreleased]

### Added

- The browser battery drives the published container image (#166).
  `scripts/ui-e2e.sh --image REF` pulls it, starts it over the same two
  committed templates and the same committed layout, and drives it with the
  same browser, building nothing from the tree. An image that answers
  `/health` and serves no `/ui` fails the run naming it, which is the one
  thing the mode exists to catch. It runs only the journeys that hold for
  every release, because the image is usually the last release and the rest
  assert what this tree does. The `released` workflow runs it weekly.

- `scripts/checks/site.sh`, which fails when the published site disagrees with
  the tree it describes (#196). It checks that every version the landing page
  states is the version this tree releases, and that every corpus figure the
  site quotes is the figure the test suite asserts. The landing page said 2658
  fields across 1789 groups against a suite asserting 2621 and 1775, and its
  own meta description, which is what a search result shows, said "Nothing
  renders a form to a clinician yet". Prose stays review-enforced: a sentence
  saying a shipped capability is unbuilt reads exactly like a true one.

- A partial date is collected one component at a time (#197). Where the
  template pins a precision the field keeps its native picker; where it admits
  several, no native control collects that, and the fallback used to be one
  text box whose placeholder was the ADL pattern `YYYY[-MM[-DD]]`. It is now a
  labelled box per component, on the precedent of the GOV.UK Design System's
  date input. A number typed short is padded, so 9 for September is written
  09, and a component to the right of an empty one is refused rather than
  closed up: openEHR RM Release-1.1.0 `data_types.html` section 7.1.2.2 drops
  components from the right and has no shape for a hole in the middle.

- A group lays its items on a two-column grid from the medium breakpoint up,
  so a form of dates and counts reads across as well as down (#190). Most
  kinds draw one input and share a line; an attachment, a parsable body, a
  choice, an interval and an identifier keep the row, because each draws a
  control inside a control or four boxes of its own. Measured from the
  capture, the two committed corpus forms are 1568 and 2143 pixels tall,
  against 4190 and 3934 when the issue was filed.

- The renderer reads the layout a person authored, which is half the product
  and until now the browser read none of it (#201). A form is drawn in the
  order they put its items in, under the names and the help text they wrote
  over the archetype's, starting from the values they chose, and an item
  behind a rule appears only while that rule holds. So a form grows as it is
  answered instead of asking every question at once: the family history form
  asks whether a family member has died, and the date and the age at death
  arrive when the answer is yes. No specification governs any of that, and it
  is what the overlay exists to carry.

  The server reads the layouts from `FERROCHART_OVERLAYS`, a directory of at
  most one overlay per template, and serves each as `FormLayout` at
  `GET /api/templates/{template_id}/layout`. A template nobody laid out
  answers with a layout that decides nothing, so a renderer has no absent case
  to tell apart from an unknown template. An overlay that will not read fails
  the startup, as a template that will not compile does, and so do two
  overlays over one template. `e2e/overlays/` carries the one authored layout
  this repository ships, and the browser battery drives it.

- The browser battery opens a section the template says may be absent and
  proves the fields inside it arrive (#202). A form now opens with those
  sections closed, so a battery that stopped at the first screen would never
  see what is in one, and the book would show a form nobody had touched. The
  capture runs the same proof, which is why the picture shows an opened
  section.

- `scripts/checks/changelog.sh`, which refuses a release section with two
  headings of one change type (#184). Every pull request adds an entry under
  `[Unreleased]`, and the quick way to do that is to paste a fresh
  `### Changed` above the old one; three of those and one release has three
  Changed sections and a reader can no longer find anything. It also checks
  the headings are the six Keep a Changelog names, in the order that
  specification lists them, and that none is empty. Every section in the file
  is normalised to that shape, with no entry reworded.

- **The round trip against a real CDR runs on every pull request** (#28). The
  live cases of `ferrochart-cdr` and `ferrochart-server` commit a COMPOSITION
  FerroCHART built and validated to a FerroEHR started from the release's own
  `compose.yaml`, read it back, and compare. They existed, and ran only when
  somebody remembered `scripts/test-cdr.sh`, so the one lane that can falsify
  the product's central promise was the one lane nothing enforced. A failure
  now prints what the CDR logged, because the container is gone by the time
  anybody looks.
- The browser battery drives and photographs **every** screen, not three of
  eight (#192). The form screen with no template named, the three frames on
  the rail whose content is still to come, and an address no route owns are
  all covered, each with a proof of its own: the way out of a dead end, the
  heading and the line under it, and the way back. The first run of the new
  coverage found #191.

### Changed

- The published site says what is built (#196). Every page in `SUMMARY.md`
  was read against the code: the landing page no longer says the product
  renders nothing, the operations card no longer says the routes that do the
  work have not arrived, the status list says the browser reads a layout, the
  build order carries the current figures and names the milestones after it,
  and the renderer page no longer says the browser honours no overlay. The
  book gains the route table the "write your own renderer" page was already
  pointing at, and names the layout as the fourth published document with a
  version line of its own.

- **The frame stops spending a fifth of the window on nothing** (#193). A
  320-pixel Inspector panel reading "Select a node to edit its layout" was
  drawn on every screen, including four that have no node to select and
  Layout itself, which says the authoring surface is not built. It arrives
  with the selection that drives it (#27). A breadcrumb of one entry is no
  longer drawn: it named the screen its own heading names, so Layout read
  "Layout" twice. And the three frames say what their screen will do rather
  than citing a tracker issue number at the reader.
- **Everything the project says about itself says what is built** (#196). It
  was three releases behind and it understated the product to the people it is
  for. The README said the round trip had never met a real CDR and to treat
  the commit path as untested. The book's front page said the same, and its
  renderer page said no published artefact serves the screens, when the
  release binary embeds the bundle and the image copies that binary. The
  landing page told a reader "the ITS-REST client, the composition builder,
  and the FHIR terminology client are decided and scheduled for v0.0.5; they
  are not written yet", when all three shipped in that release, and its
  machine-readable version said `0.0.3`. The build order read as a plan for
  work that has shipped, a health check example answered `"version":"0.0.2"`,
  and an attestation example named a `v0.0.5` asset.

  None of it was a version bump: every claim was read against the code. What
  the pages say is missing now is what is missing, which is the layout overlay
  reaching the browser and a form that can commit.

- **An open archetype slot reads as a place to add content, not a warning**
  (#180). openEHR AM Release-2.3.0 `AOM2.html` section 4.5.8 gives
  `ARCHETYPE_SLOT` the attribute `is_closed`, "closed to further filling
  either in further specialisations or at runtime", and defaults it to false,
  so a slot that survived into an operational template is one the template
  meant to leave open. It was drawn as a yellow warning, and counted in a
  banner reading "the template left content undetermined", on 116 of the 121
  templates the committed pack compiles. It is a neutral notice now and the
  banner counts only what is genuinely undetermined, which across the whole
  pack is four nodes rather than 1194.
- **The Reference Model's own container is not drawn as a group** (#190). RM
  Release-1.1.0 `data_structures.html` section 4.3 makes `ITEM_SINGLE`,
  `ITEM_LIST` and `ITEM_TREE` the structure an ENTRY attribute holds its
  content in; they exist because the model needs a container, not because
  anybody grouped anything, and the CKM archetypes head them "Tree" and
  describe them "@ internal @". Their contents are drawn where they were, so
  a form loses a nesting level and two meaningless headings. An `ITEM_TABLE`
  keeps its card because a grid is a shape a person reads, a `CLUSTER` keeps
  its card because a cluster is the clinical grouping, and a container that
  roots an archetype or repeats keeps its card because it carries something
  of its own. No key, value or composition node changes.

- **A timezone is chosen from the list the platform carries, and resolved to
  the offset at the instant entered** (#197). It was a free-text field with a
  regular expression, so a person had to know their UTC offset and today's
  daylight-saving state and type `+02:00` in a shape the pattern accepted.
  The list is the IANA Time Zone Database, whose registry RFC 6557 puts with
  IANA and which every current browser already carries: ECMA-402 publishes the
  identifiers as `Intl.supportedValuesOf('timeZone')`. Nothing is vendored,
  so the bundle carries no table. openEHR RM Release-1.1.0 `data_types.html`
  section 7.2.4 types `DV_DATE_TIME` on `Iso8601_date_time`, so the value
  carries an offset rather than a zone name, and `Europe/Amsterdam` is
  `+01:00` in January and `+02:00` in July; the offset is resolved for the
  instant the reader entered rather than for today. A platform that does not
  publish the list still gets the offset field.
- A field that admits several precisions asked for `YYYY[-MM[-DD]]`, which is
  ADL notation printed at a clinician (#197). It now asks for "A year, and a
  month and day if you have them". The pattern still gates what the browser
  accepts; only what a person reads changed.


- **A duration is a number and a unit** (#198). "Age at death" drew Years,
  Months, Weeks, Days, Hours, Minutes and Seconds as seven number boxes plus a
  checkbox, for a question a person answers with "82". The family history form
  carried fourteen of those boxes, seven for the age at death and seven for
  the age at onset. It is now one number and one unit, and where the template
  admits a single unit there is no picker at all. A compound duration stays
  reachable behind "and…", so the rare case works and the common case costs
  nothing. The sign is offered only where a range could admit a negative
  duration, so an age bounded at zero no longer asks whether it counts
  backwards.
- Two parts of a duration naming one unit are refused. `assemble` writes the
  first match per unit, so the second was being dropped without a word.


- **A form is a list of questions with answers, and most of what it drew was
  neither** (#190). Four of the five lines a field drew were not the value.
  - A field carries a value **or** a reason there is none, never both. The
    null-flavour select was drawn full width under every field that offered
    one, which in the committed corpus is nearly every field, so a
    fifteen-field form carried fifteen extra dropdowns reading "No value,
    because: A value is entered". It showed a state openEHR RM Release-1.1.0
    `data_structures.html` section 5.2.3 forbids, since `Inv_is_null_valid`
    gives `ELEMENT` no state carrying the two together. A quiet "No value"
    at the end of the value row now replaces the control with the flavour and
    a way back.
  - A field name is 14px solid ink where it was 12px grey, the same weight as
    the help text under it. The name is what a person scans a form by.
  - The help text sits with the name it describes. Under the control it read
    as a caption for the next field.
  - A value sits in one box, not three. The per-field fieldset keeps the
    accessible name its `legend` carries and draws no border.
  - A date, a count and a unit stop at 28rem. Prose, an attachment and a
    choice keep the width, because their answers use it.
  - A choice picker is labelled "Kind of value" where it said "Which of
    these", and a repeat is headed "Entry 2" where it said "Occurrence 2".

  Measured in the browser, over the two committed corpus forms: the family
  history summary fell from 4190 to 3086 pixels and the alcohol consumption
  summary from 3934 to 2828, both a little over a quarter shorter. The capture
  prints the height of every screen it photographs, so the figure is read from
  a run rather than estimated.


- **The living style guide is no longer in a clinician's download**, and the
  renderer bundle fell from 410995 to 359486 gzipped, a saving of 51506 bytes
  or 12.5% (#154). The design system draws every affordance the kit defines,
  which instantiates every control a second time, and it is a surface for
  whoever works on the kit rather than for a person filling a form. It is
  behind the `design` cargo feature now, which is on by default: `trunk serve`
  brings it up at `/ui/design` and the browser battery photographs it, while
  the release bundle is built `--no-default-features`. That build answers
  `/ui/design` the way it answers any address it does not serve, and the rail
  carries no entry for it. The feature is on by default so that an affordance
  the kit defines and nothing draws is still a dead-code error (#186).
- **The renderer says what a field collects instead of naming a Reference
  Model class** (#178). A choice between unlabelled alternatives offered
  `DV_COUNT` and `DV_QUANTITY` as its options and now offers "a whole number"
  and "a measurement"; an undetermined hole read "ITEM_TREE is left
  undetermined." and now reads "The template leaves a group of fields here
  undetermined."; the line under it says what the template does and does not
  admit without spelling `ELEMENT` or `DV_INTERVAL`. The class name is a fact
  about the specification, and the audience came to build a form. The
  refusal message built in `ferrochart-compose` and the unlabelled-field
  fallback are the rest of #178 and are not fixed here.
- **A field the template never labelled says what it collects, and an
  unlabelled archetype root says its concept** (#178). The fallback was the
  node identifier, so a form drew a heading reading `at0004` or
  `openEHR-EHR-OBSERVATION.blood_pressure.v2`. An archetype identifier names a
  concept (openEHR BASE Release-1.2.0 `base_types.html` section 5.4.10), so a
  root is now called "Blood pressure"; below one the name leads with the value
  and keeps the code, as "A whole number (at0004)", because the code is what
  tells two siblings apart and dropping it would give a group several fields
  with one name. A browser journey over every screen the battery drives fails
  the build on an openEHR class name or a node code in the text a person
  reads, and skips the monospaced surface where the path deliberately stays.
- A refusal on a link field read "The Reference Model class requires another
  URI scheme." and now reads "The link has to start with the scheme this field
  takes." (#178).

### Fixed

- A timezone picked before the date was resolved against the wrong instant
  (#197). A zone is not an offset, and the offset was resolved once, when the
  zone was chosen. A reader who picked Europe/Amsterdam and then typed a
  January date kept whatever offset was in force at the moment they picked. It
  is re-resolved whenever the zone or the instant changes. A browser journey
  pins both halves of the year.

- A date field spent half its width on a timezone it does not collect (#190).
  The temporal control drew a two-column grid whatever the field admits, so a
  date got one column of it and its placeholder read "Year, then month and day
  if kn". The grid is one column where there is no zone to put in the second.

- A field the template fixed drew the sentence "The template fixed this
  value" instead of the value, and one the template also said may be absent
  could not be answered at all (#207). A template can narrow a field to one
  value and still leave the element optional, which is how the family history
  template asks whether a family member has died: the question is whether the
  element is recorded, and yes is the only answer it admits. The value the
  constraint names is now read in one place, `FieldKind::only_admitted`, which
  the derivation and the renderer both use. A fixed field shows the value it
  fixed; an optional one draws a checkbox that records it or leaves the
  element out; a required one is written into the document, so a COMPOSITION
  cannot go out missing a node the template requires.

- A repeating group inside an occurrence a reader added opened with one
  entry, whatever its template said (#202). The count of occurrences a form is
  showing is seeded from the definition, and a group inside a section nobody
  had opened yet was never seeded, so the lookup answered with a hard-coded 1.
  It answers with what the template requires now. The browser battery found
  this the moment it started opening a section rather than photographing a
  closed one.

- **A section the template says may be absent opens with none of it** (#202).
  A repeatable group opened showing `max(1)` occurrences, so a group whose
  template says `0..*` opened one, expanded, with every field inside it drawn.
  openEHR AM Release-2.3.0 `AOM1.4.html` section 4.3.6 makes `occurrences` the
  count a node may appear in the data, so a lower bound of zero is the
  template saying none of that section is a complete answer, and the renderer
  was discarding the only statement a template makes about how much of itself
  applies. The family history form met a reader with a family member they had
  not said existed and a clinical history for them. It now opens as a heading,
  a description and "Add one". The floor on removing followed the same rule
  and used to be one whatever the template said, so an optional section, once
  added, could never be put back.

- A field the template fixed showed no value and offered "No value" beside it
  (#190). "Deceased?" asked a question, answered "The template fixed this
  value, so nothing is entered", and then invited the reader to say there was
  no value. It shows what the template fixed it to now, and a fixed field
  offers no null flavour, because a value the template gave is not one a
  person can decline to give.

- An address no route owns lost the whole application (#193). It was drawn
  outside the shell, so a reader who mistyped one had no rail, no theme
  control and one link. It is a route inside the shell now, and it points at
  the template library, which is somewhere a reader can act.
- The form screen asked the server for a template with no name (#191). Landing
  on `/ui/forms` from the rail sent `GET /api/templates//definition` and took a
  404, because the guard sat on what the screen draws and not on the request.
  The screen looked right, which is why it went unnoticed until the battery
  started photographing it.

## [0.1.0] - 2026-09-08

### Added

- "Write your own renderer" in the book, which is the page that makes the
  published form definition a contract rather than an internal type (#26). It
  describes the three documents and their version, the externally tagged
  shape, why a key carries five parts, how an entered value is addressed
  inside a repeating group, where a refusal is drawn and where it is not, and
  which two changes are deliberately not a version bump.


- The server serves the renderer at `/ui/`, and `/ui` redirects onto it
  (#166). The bundle Trunk builds rides inside the `ferrochart` binary as a
  table of files the build script writes, so a release archive and the
  container image behave the same and a request path never reaches the
  filesystem. A path under `/ui` that names a file type the bundle does not
  hold answers `404`, and any other path answers `index.html`, which is how a
  client-side route deep-links. Content-hashed assets are served
  `public, max-age=31536000, immutable` and `index.html` `no-cache`, each with
  its own media type and `X-Content-Type-Options: nosniff`. `FERROCHART_UI=off`
  drops the routes, and a binary built without the bundle serves no `/ui`
  route. With the quickstart `compose.yaml` the address is
  <http://127.0.0.1:8080/ui/>.
- Every release carries a second CycloneDX document,
  `ferrochart-renderer-<tag>-<target>.cdx.json`, attested against the same
  archive (#166). The server's own document reaches none of `leptos`,
  `wasm-bindgen` or `web-sys`, because the cargo feature that compiles the
  bundle in is empty and there is no dependency edge to follow, so the
  WebAssembly the binary serves to every reader was absent from the document
  that claims to describe what shipped. The lane refuses to publish a renderer
  document that lists none of those three.
- `scripts/checks/palette-utilities.sh` refuses a control that draws its own
  focus ring or sets `outline-none` (#157). The stylesheet's base layer holds
  one `:focus-visible` rule for the whole application, and that rule is what
  guarantees every control has an indicator; a class that overrides it removes
  one silently. The assertion existed in one kit module of eleven and now
  covers every source file, including the ones nobody has written yet.


- The book shows the renderer, and a test takes the pictures (#93).
  `scripts/ui-e2e.sh` stands up the form surface over two committed CKM
  templates, serves the renderer bundle against it, and drives a pinned
  headless Chromium through the template library, both forms and the design
  system over WebDriver. `--docs-shots` walks the same screens again on both
  grounds and writes one PNG each into
  `website/book/src/operate/img/renderer`, which the new
  `website/book/src/operate/renderer.md` page and the book's introduction
  embed. The journeys and the capture share one definition of what a screen
  must have drawn (`e2e/tests/it/screens.rs`), so a screenshot cannot outlive
  the journey that proves it, and an ordinary run rewrites no tracked file.
  A `ui-e2e` CI job runs the battery on every pull request, uploads what a
  failed journey was looking at, and fails when a run changed a tracked file.
  `scripts/checks/docs-shots.sh` refuses an image that is not a PNG, is small
  enough to be a blank page, carries a name the capture does not write, or is
  committed with no page embedding it.
- The form screen draws the controls (#26). `/ui/forms/{template_id}` fetched
  a definition and rendered its group tree as headings and labels, because the
  request module and the controls were built in parallel and nothing joined
  them. It now renders `control::group::FormBody`, so a clinician sees the
  form rather than its outline: inputs, the null-flavour affordance beside
  each field, a repeatable group carrying its occurrence bounds, and a field
  the template fixed saying so instead of offering an entry.
- `trunk serve` proxies `/api` as well as `/health`, so a local session runs
  the whole surface rather than half of it.


- The renderer's conversation with the server (#138). One module,
  `app/ferrochart-renderer/src/api`, owns every request the browser makes over
  `gloo-net`, and a test scans the crate's own sources and fails when any other
  module names the transport. Its `ApiError` carries the status the server
  answered with and the bytes it answered with, reads the surface's error
  document so the stable code and the message arrive as fields, and passes a
  CDR's own `cdr_status` and `cdr_body` through. A request that never reached
  an answer reports no status rather than an invented one.
- The template library at `/ui/templates` lists what the server holds, and
  `/ui/forms/{template_id}` draws the group tree of one form definition as
  headings with the field labels beneath. A read renders its failure inline,
  with the status, the server's code, the CDR's answer and every link of the
  cause chain, and never as a toast.
- `app/ferrochart-renderer/src/placement.rs`, the lookup a control calls to
  find the validation failures that belong to it. A failure lands on every
  control its node key names, and one about how many nodes there are is drawn
  once rather than under every repeat. A failure that names no field of the
  form stays reachable through `unplaced` and is shown beside the form.
- A control for every one of the eighteen `FieldKind` variants (#137), each
  rendering from a pure admission function that refuses what the template
  refuses: a boolean whose two flags are independent, a text field whose
  closed list is the whole permitted set, a coded field that says plainly when
  a value set needs a terminology server rather than inventing an expansion,
  an ordinal that stores the symbol and shows the rubric, a quantity whose
  magnitude range and decimal precision follow the chosen unit, a proportion
  that holds the `DV_PROPORTION` invariants of openEHR RM Release-1.1.0
  `data_types.html` section 6.2.10, and a date, time and date-time at the
  precision the template states and no finer.
- The null-flavour affordance beside a field, and the add and remove pair a
  repeatable field or group carries, disabled at the template's ceiling and
  floor. Content the template left undetermined is drawn as a visible hole
  carrying its reason and captures nothing.
- Ten design-system affordances beside the buttons and surfaces already there:
  the table shell, tab pills, the segmented control, badges and status pills,
  the stat tile, the empty state, the loading skeleton, the confirm dialog,
  the toast queue, and the inline notice family. Each has one definition, and
  the living style guide at `/ui/design` draws every one of them beside a
  synthetic form carrying one field of every kind.
- The HTTP form surface (#146): `GET /api/templates`,
  `GET /api/templates/{template_id}/definition`,
  `POST /api/templates/{template_id}/validation`,
  `POST /api/ehrs/{ehr_id}/templates/{template_id}/compositions` and
  `GET /api/ehrs/{ehr_id}/compositions/{uid}/values?template={template_id}`.
  No specification governs a route of it: openEHR ITS-REST defines the CDR's
  API and says nothing about a form server in front of one, and
  `docs/architecture.md` section 11.1 is the design of record. An unknown
  template is 404, a body the route cannot read is 400, values the template
  refuses are 422 with the report, and a CDR that refused or never answered is
  502 carrying the CDR's own status and body.
- `FERROCHART_TEMPLATES`, a directory of `.opt` operational templates the
  server compiles at startup into a form definition and a flattened validator
  per template. A template that will not compile fails the startup naming the
  file, and so do two files stating one template identifier. The variable is
  optional, and unset means the server holds no template.
- The entered-value types (`FormValues`, `Slot`, `Entered`, `Datum`) serialise.
  A `FormValues` is a JSON array of entries, each spelling out the node key,
  the occurrence path and the occurrence beside the value, so the occurrence
  path survives the wire. `ferrochart-form` carries a round trip and a golden
  wire snapshot for it, the pair the form definition already had.
- `ValidationFailure` can name the composition builder as its source, so a
  required field left empty reaches a renderer keyed onto that field rather
  than as prose it cannot place.

- The design system (#101): the renderer is a Leptos client-side binary built
  by Trunk, and it lands with its frame before its content. Every screen
  styles against semantic tokens, dark mode is the same token names redefined
  under `.dark`, and the accent is the Rose & Iron palette
  (`assets/brand/README.md`). The shell is three columns, a 56px topbar over a
  208px rail, a canvas, and a persistent inspector, with the rail becoming a
  focus-trapped overlay below the breakpoint. A living style guide at
  `/ui/design` draws every affordance the kit defines.
- The token contrast gate: the stylesheet is the one place a colour is
  written, and the renderer's tests parse it and re-measure every pair the
  design depends on, in both themes, against WCAG 2.2 success criteria 1.4.3
  and 1.4.11. Three values differ from the sibling viewer as a result: the
  dark accent is blush rather than rose-light, a control boundary is darkened
  until it clears 3:1, and a placeholder takes the muted step rather than the
  faint one.
- `scripts/checks/palette-utilities.sh`: a raw palette utility, a per-element
  `dark:` override, or a hex colour outside the stylesheet fails the build.
  Neither sibling enforces this rule, and both state it in prose.
- `scripts/checks/bundle-size.sh` and `app/ferrochart-renderer/bundle-size.json`:
  a ceiling and a per-change budget per asset. A pull request is charged
  against the baseline recorded in the merge base rather than in the branch,
  so a change cannot pass itself by editing the number.
- The `renderer` job in CI: `cargo fmt` plus `leptosfmt`, clippy on
  `wasm32-unknown-unknown`, the tests, the palette check, a release bundle
  build, and the bundle budget.

### Changed

- **The published documents are externally tagged, and `FORMAT_VERSION` is 2**
  (#154). A variant is spelled `{"quantity": {…}}` where version 1 wrote
  `{"kind": "quantity", …}`. The reason is measured rather than aesthetic: an
  internally or adjacently tagged enum cannot be deserialised in one pass, so
  serde buffers the input through `Content` and monomorphises the whole subtree
  twice. Over the renderer that cost **33269 gzipped bytes, 7.6% of the bundle
  a reader downloads**, confirmed by building both ways. The same buffer is
  what #103 broke against, when a dependency enabling
  `serde_json/arbitrary_precision` stopped these documents deserialising with
  no line changed in this tree. An externally tagged enum never enters that
  code path, so the change removes a hazard class as well as the bytes.


- The tied-sibling fold cites ADL 1.4 section 5.3.4.2 rule VCOC rather than
  AOM 2's VSONCO (#142). Four places in this repository said VSONCO was "the
  only definition of collective sibling occurrences openEHR publishes; AOM 1.4
  defines none", and that is false: ADL 1.4 publishes VCOC, in the generation
  these templates are written in, and it states the sum of upper bounds, which
  is the bound that was in dispute. The arithmetic is unchanged, because it
  was already what VCOC requires. The upstream-report register loses a
  consequence it should never have carried.


- The envelope (`Envelope`, `Composer`, `Subject`, `Setting`) moved from
  `ferrochart-compose` to `ferrochart-form` (#151), so the renderer can build
  the value it has to submit. openEHR RM Release-1.1.0 `ehr.html` section
  5.2.2 makes `COMPOSITION.composer` mandatory and nothing authorises a server
  to invent it, and the renderer links `ferrochart-form` alone. The code that
  turns one into a Reference Model `CODE_PHRASE` stays with the builder. The
  serialised shape is unchanged, and the surface tests prove it.
- `FormValues::remove_in` is `#[must_use]`. A caller that discards what was
  forgotten now says so, and the attribute caught two sites that did not.

- The entered-value types (`FormValues`, `Slot`, `Entered`, `Datum`) moved from
  `ferrochart-compose` to `ferrochart-form` (#139). A renderer collects values
  and links `ferrochart-form` alone, so leaving them behind the composition
  builder left the browser unable to name the type it exists to fill in. The
  types carried no serialisation at the time of the move, so nothing on any
  wire changed with it.
- `deny.toml` ignores two RustSec unmaintained advisories, `RUSTSEC-2024-0436`
  (`paste`) and `RUSTSEC-2026-0173` (`proc-macro-error2`). Both crates are
  build-time proc-macro dependencies of Leptos, neither is a vulnerability,
  and no code from either reaches the bundle a reader downloads. Tracked as
  #135.

- `FormValues::remove_in` and `FormValues::remove_under`: a set of entered
  values could be written to and never unset, so a form had no way to say a
  clinician removed an occurrence. `remove_under` forgets everything inside
  one instance of one group, which is what removing a repeat means.

### Fixed

- A refusal from the composition builder is drawn on the repeat it came from
  (#152). A validation failure carried no occurrence, so a form with a
  repeating group showed "this value is out of range" under every repeat of
  the field rather than the one that was wrong, and 67 of the 121 forms the
  committed pack derives to carry a repeating group. The builder walks a form
  with the occurrence of every repeating group above each field, so it knows
  the address and now states it.
- A refusal from the operational template still states no occurrence and is
  still drawn on every repeat. Its Reference Model path carries positional
  predicates into attribute arrays, and nothing establishes their
  correspondence to a form's occurrence path, so an address there would
  sometimes be invented. The type says which judgements know and which do not.


- A COMPOSITION built around a template rooted below COMPOSITION states its
  template identifier where the template is active, and a real CDR now accepts
  it (#163). It was written at the wrapper COMPOSITION, a document FerroCHART
  supplies from configuration and where no template is active, so FerroEHR
  4.1.1 refused **66 of 66** such templates with `expected RM type conforming
  to OBSERVATION but found COMPOSITION`. Every one had passed FerroCHART's own
  gate first, which is the failure this project is least allowed to have.
  openEHR RM Release-1.1.0 `common.html` section 3.2.3 makes the identifier
  conditional: it is stated "if a template was active at this point in the
  structure".
- The read-back looks for the template identifier wherever it legitimately
  sits, rather than at the document root alone, so the guard that refuses a
  composition from another template keeps working for the 113 templates whose
  identifier moved.


- The release lane builds and describes the binary it ships rather than every
  binary the workspace declares (#166). `cargo auditable build --workspace
  --bins` tried to link the renderer, which is a browser binary, for
  `x86_64-unknown-linux-musl`, the Package step would have tarred it into the
  archive, and the SBOM guard refused the whole lane because the workspace
  declares a bin set it did not describe. The lane now builds `-p ferrochart`
  with the renderer feature on, packages the released package's own bins, and
  the guard names both bins and says which document covers which.

- A repeatable group builds one instance per occurrence a clinician entered,
  and reads back into the instance it came from (#141). The composition
  builder used to build exactly one instance of every group whatever the
  template admitted, so a second entry in a repeating group was dropped
  without a word; 67 of the 121 forms the committed pack derives to carry at
  least one repeating group, and 1868 fields sit under one. A value is now
  addressed by the occurrence of every repeating group above it as well as by
  its field and its own occurrence, the builder appends an index at each such
  group, and the read-back assigns the same indices in document order.

## [0.0.5] - 2026-09-08

### Added

- The web template compatibility surface (#54): `ferrochart-webtemplate` reads
  a web template another tool produced into a form definition, and writes a
  form definition back out as one. The web template is a compatibility target
  and no openEHR specification defines it, so the crate sits beside the
  compiler rather than under it and links `ferrochart-form` and nothing else
  of this tree. A member the form definition does not model, a third party's
  annotations among them, survives a round trip verbatim: a read keeps each
  node's source object beside the definition, and a write starts from it, so
  a node the definition still states as it was read goes back out untouched.
  Over the whole vendored CKM pack, every document built from it reads and
  writes back as the document it came from, with annotations, per-option
  terminology bindings and members this crate has never seen put into it
  first. `min` and `max` are read as the flattened integers they are, and
  template version detection reads `templateId` and the root archetype
  identifier, never `semVer`, which not one document of the pack states.
- A COMPOSITION reads back into the form it was built from (#23), completing
  the pair. The reader walks the data rather than the form, so a node the form
  does not cover is reported instead of silently dropped, which is what an
  editing round trip needs if it is not to delete a colleague's content. Over
  the committed pack every value entered comes back against its own field, a
  null flavour and its reason survive both directions, and a repeated field
  reads back into the right occurrence. Where several form nodes share an
  identity and the data cannot fill them all, the assignment is a guess the
  reader reports as one rather than presenting as certain.
- The pre-post validation gate (#24), in the new `ferrochart-validate` crate
  and `ferrochart_server::commit`. A COMPOSITION FerroCHART built is judged
  against its own operational template before any request reaches a CDR, and
  every failure it finds is keyed onto the form definition, so a renderer puts
  it on the field a clinician got wrong. The gate is the only path in this
  workspace from entered values to a CDR write, and the ordering is tested by
  construction: the client is pointed at a port nothing listens on, so a
  refusal proves the gate ran first. A negative case per constraint kind
  covers the value set, the numeric range, the string pattern, the data value
  class, the occurrences, the cardinality, a mandatory node left out, and a
  code outside its openEHR terminology group. A live case commits through the
  gate to a real CDR and reads the document back.
- A validation report a renderer reads, in `ferrochart-form`. It carries one
  failure per problem, each with the node key it belongs to, the path it was
  found at, its message and its category, so a renderer still links one crate
  of this tree. A failure that names no field of the form keeps its path and
  is reported unplaced rather than dropped.
- A best-effort reader for a CDR's own error body, marked vendor-specific.
  openEHR ITS-REST Release-1.1.0 publishes no path on an error entry, so the
  reader recognises one inside the prose where a CDR writes one and reports
  the refusal verbatim where it does not.

- Three layers that guard the two documents FerroCHART publishes, after a
  dependency's cargo feature took one of them off the wire with no change to
  this tree (#107). `serde_json/arbitrary_precision`, enabled for the whole
  workspace by one new dependency, sends a number to serde's content buffer as
  a map, and the internally tagged enums of the form definition stopped
  deserializing. Each layer catches something the others do not: the form
  definition and the layout overlay each round-trip a document carrying every
  number-bearing shape they publish, so a document that no longer reads back
  fails; each also carries a golden snapshot of its exact bytes, so a document
  that still reads back and no longer means the same thing to a consumer
  written against the old bytes is a diff a person reads and accepts; and
  `scripts/checks/serde-json-features.sh` compares the resolved `serde_json`
  feature set against the reviewed one, failing on a refused or an unreviewed
  feature and naming what turned it on, which is the cause the two tests can
  only report the effect of. The form definition's `FORMAT_VERSION` now says
  what the number covers, so a byte change is a decision rather than an
  accident.
- The composition builder (part of #23), in `ferrochart-compose`. It turns
  what a clinician entered into a COMPOSITION, working from the form
  definition alone and never from the template, which keeps that definition a
  contract a third party can commit against as well as render against. 78 of
  the 121 committed templates that read build a document; the 36 rooted at
  `CLUSTER` are refused by name, because a fragment is not a document.
  `docs/architecture.md` sections 5.3 and 5.4 record where every Reference
  Model field a form never shows comes from, which of them FerroCHART invents,
  and what a round trip may assert.
- The terminology client (#25), in `ferrochart-term`. It resolves the codes
  behind a coded field, and the default path needs no network: an
  archetype-local `at`-code list comes out of the template's own terminology
  with its rubrics in the requested language, and openEHR's own terminology
  groups come from the `openehr-term` crate. A test proves the local path
  issues zero HTTP requests. Where the template names a target, the client
  calls a FHIR R4 4.0.1 terminology server: `ValueSet/$expand` to fill a
  picker, `CodeSystem/$lookup` for the display text of an enumerated external
  code, and `ValueSet/$validate-code` to confirm a chosen one. Every request
  is formatted against the generated `fhir-types` operation contracts, and no
  FHIR resource is hand-written. A refused expansion, a value set the server
  does not hold, a timeout and a binding no FHIR request can name are each a
  typed error carrying the upstream status, the `OperationOutcome` diagnostics
  where the body carries one, and the raw body regardless. None of them is an
  empty picker. Expansions are cached per template, language and request, and
  a recompile of one template drops that template's entries.
- FerroCHART does not parse the SNOMED CT expression constraint language, and
  passes an expression through to the server as the implicit value set of FHIR
  R4 `snomedct.html` section 4.3.1.0.9. The reason is in
  `docs/architecture.md` section 7.3 and its decision register: executing an
  expression needs a SNOMED CT release, a concept store and a closure index,
  which is a terminology server's work, and `sct-ecl` would add a BUSL-1.1
  licence exception for a check the server repeats anyway.
- `scripts/test-term.sh` runs the terminology client's wire tests against a
  real server, HL7's public R4 service by default and the compose demo profile
  with `--ferroterm`. The cases are `#[ignore]`d, so a run without a server
  reports them as ignored rather than as passes.
- Every chapter that asks for a table of contents is checked to have one
  (#98). `scripts/checks/book-toc.sh` reads the rendered pages, and the docs
  lane runs it after the site is assembled, because mdBook reports a
  preprocessor that ran and produced nothing as a warning at most and would
  otherwise publish a chapter with a missing table of contents and a green
  build.
- The terminology of an archetype can be enumerated, where it could only be
  asked about a code the caller already held (#91). `Terminology::definitions`
  returns the rubrics one language states, `all_bindings` and
  `all_constraint_bindings` return every binding as a code and one target, and
  `value_sets` returns every enumerated value set with its members. All four
  borrow the terminology and come back in code order, so a caller that wants
  one code still pays for one lookup. The terminology client needs this to
  emit an archetype's value sets as FHIR `CodeSystem` and `ValueSet`
  resources, which a lookup-only interface cannot do.
- The openEHR ITS-REST client (#22), in `ferrochart-cdr`. It creates, reads
  and updates a COMPOSITION, uploads and retrieves a template in both ADL
  generations, and creates the EHR a first run needs. Every identifier the
  wire carries is its own type, because the three composition operations spell
  `uid_based_id` alike and accept different forms of it: the read takes either
  a versioned object uid or a full version uid, and the update takes the
  object uid only. `If-Match` is sent quoted and unweakened on every update,
  `Prefer` is always explicit, and a 412 is a concurrent-edit outcome the form
  surfaces rather than a retry. Every status openEHR ITS-REST Release-1.1.0
  publishes has its own error variant, plus a catch-all, because the same
  section permits statuses beyond that set. Nothing absorbs an upstream
  failure into an empty value.
- The FHIR model comes from the published `fhir-types` crate (Apache-2.0,
  pinned at 0.1.85), which `ferrochart-term` now depends on (#95). It is
  generated from the HL7 FHIR packages, so `ValueSet`, `CodeSystem`,
  `ConceptMap`, `Parameters` and `OperationOutcome` are read from a generated
  model the way the openEHR types already are, and no FHIR resource is
  hand-written here. The crate also carries `$expand`, `$lookup` and
  `$validate-code` as typed request and response shapes that convert to and
  from `Parameters`, which is why the terminology engine of the same release
  line is left alone: FerroCHART is a client of a terminology server rather
  than one. `docs/architecture.md` section 7.3 records both decisions.

### Changed

- The hard rule about the non-canonical formats now names the two of them
  separately, because openEHR only specified one (#104). ITS-REST
  Release-1.1.0 publishes `simplified_formats.html`, "Simplified Formats for
  openEHR Data", in the STABLE state, so that document is the authority for
  the FLAT and structured formats: their media types (section 2.3), their
  field identifiers (section 4.2), level removal (section 4.6), the `|other`
  suffix (section 4.7), and the Reference Model mapping class by class
  (section 5). The web template stays a compatibility target, on the same
  document's word: section 2.2 puts "Web Template itself as a resource" under
  what the specification does not cover. Where the Reference Model and
  `simplified_formats.html` disagree, no specification settles it, so the two
  divide by subject and a real contradiction is filed upstream.
  `CLAUDE.md`, `.claude/rules/spec-adherence.md`, `.claude/rules/testing.md`,
  the `spec-researcher` agent, the `/spec-lookup` skill, the contributor book
  and the `compat` label all carry the corrected split.

### Fixed

- The last hand-built archetype identifier in the three-part `template_id`
  form, in `ferrochart-form`'s wire test. #127 corrected the `.opt` fixtures
  and its check covers those only, so this one sat in a hand-built key and in
  the committed wire snapshot, disagreeing with `key.rs`'s own example.
- The synthetic fixtures stated two shapes the Reference Model does not admit
  (#127), both invisible until the commit gate landed something that judges an
  instance. Ten `.opt` fixtures gave an `archetype_id` a three-part version,
  which is the template identifier's form: openEHR BASE Release-1.2.0
  `base_types.html` section 5.5 gives an `archetype_id` a single version
  number, and section 5.4.11 leaves `TEMPLATE_ID` with no lexical form at all.
  Twelve `OBSERVATION`-rooted fixtures put an `ITEM_STRUCTURE`, a `CLUSTER` or
  an `ELEMENT` straight under `OBSERVATION.data`, where RM Release-1.1.0
  `ehr.html` section 8.3.4 types that attribute `HISTORY<ITEM_STRUCTURE>`, so
  a `HISTORY` and an `EVENT` belong between; all 118 `OBSERVATION.data`
  attributes of the vendored CKM pack already state it that way. Anything
  those fixtures proved about an `OBSERVATION` was proved about a structure no
  CDR would accept. A new check over every committed `.opt` fixture holds all
  three shapes, so the next one fails at the fixture rather than at a CDR.

- `docs/architecture.md` section 6.2 recorded a corpus split that does not
  sum: "87 single-purpose conformance templates plus 16 CKM clinical
  templates" for a total of 102. The total is consistent everywhere and is
  what every measurement rests on; the split is not re-derivable, because that
  corpus has since grown past 400 templates, so the document now says so
  rather than repeating an arithmetic that cannot be right. Section 16 gains
  the two upstream-report candidates found since it was written (#111, #129).
- An operational template can declare two sibling constraints sharing the node
  id, the archetype id, the Reference Model type and the pinned name, which no
  instance can tell apart, and a form offering a field per member built a
  document its own template refuses (#122). AOM 1.4 permits the shape: it says
  a node id distinguishes siblings but states no invariant that requires it,
  and the ITS-XML schemas carry no identity constraint either. The compiler now
  folds such a group before it computes a key. Members that state the same
  constraint become one node, carrying the collective occurrences under a
  container attribute, which is lossless because the members are the same
  constraint. Members that differ under a container attribute are refused by
  name, because no instance could say which one it satisfies; that refuses no
  template of the committed pack. Folding before the key is what keeps the
  layout overlay stable when a template is republished with the duplicate
  gone. Over the pack, two more templates now build a document their own
  template accepts, and the gate reports no occurrences failure at all.
- A composition wrapped around a template rooted below COMPOSITION named the
  entry's archetype as its own, so the document claimed a COMPOSITION was an
  OBSERVATION archetype root. That was 113 of the 123 committed templates.
  `ehr.html` section 5.4.1 makes `Is_archetype_root` an invariant of
  COMPOSITION and `common.html` section 3.2.2 requires a root's
  `archetype_node_id` to be the stringified archetype id, so the wrapper needs
  an archetype of its own. Configuration names it, the way it names the
  territory, and a build with none refuses rather than inventing one. Found by
  the commit gate of #24.
- A COMPOSITION FerroCHART built put no `ARCHETYPED` on any node below its
  root, so every ENTRY it wrote violated `Is_archetype_root`
  (openEHR RM Release-1.1.0 `ehr.html` section 8.3.1, with
  `LOCATABLE.Archetyped_valid` in `common.html` section 3.1.2). Every
  archetype root below the document root now carries its own archetype
  identifier and the Reference Model release, and the template identifier
  stays at the document root alone, which is what `common.html` section 3.2.3
  says. The pre-post gate of #24 is what found it.


- `docs/VERSIONS.md` said "Nothing is vendored yet" twelve lines above the
  table of what is vendored, and carried two headings for one subject (#112).
  It now has one section that says what is vendored, what is not, and that no
  specification text is, which is the fact a reader of a citation needs. The
  same file no longer claims there is no Cargo workspace, and
  `.claude/rules/vendored-inputs.md` no longer says nothing is vendored.
- The datatype grid fixture carries the two field kinds it claimed to and did
  not (#114). `ferro_datatype_grid.opt` now states a `DV_ORDINAL` with three
  graded symbols and a `DV_STATE` with a two-state machine, so the derivation
  of both is snapshotted rather than asserted by a comment. `DV_SCALE` stays
  the exception the comment now names: `OpenehrProfile.xsd` declares no
  `C_DV_SCALE`, so only the ADL 2 reader reaches it.

## [0.0.4] - 2026-09-07

### Added

- Every icon the site should carry, from the one authority. `/favicon.ico` is
  served at the site root for a client that asks for it directly, the landing
  page links the brand palette instead of restating its hex values, and the
  book's tab icons are staged from `assets/brand/` at assembly time rather than
  committed as a second copy. `assets/brand/README.md` records where each file
  is used, and why five of them are deliberately used by nothing (#94).
- A field carries the reference bands its template states beside the value
  (#52). openEHR RM Release-1.1.0 `data_types.html` section 6.2.1 gives every
  `DV_ORDERED` a `normal_status`, a `normal_range` and
  `other_reference_ranges`, none of them entered by a clinician, and the
  derivation used to drop all three. They now sit in
  `field::ReferenceRanges`, reachable only from a field or a choice
  alternative and never from a field's kind or a group's items, so a renderer
  shows them without reading a Reference Model class name to tell metadata
  from an entry field. A band reuses the value shape of the class it is stated
  over: a status carries the value set of a coded field, and a range carries
  the interval shape of the class the value collects. The three attributes are
  no longer refused as unentered, and a band the derivation cannot represent
  is refused by name rather than dropped.
- What "equivalent" means for the two ADL readers is defined in one place,
  cited, with the six differences accepted as legitimate and a count for each
  allowance it spends, and the matched pair is compared as a whole value so a
  field the internal constraint model grows cannot be left out of the
  comparison (#59). The published dual-dialect archetype library cannot supply
  the ADL 1.4 half of that comparison: the twins are ADL 1.4 text, the ADL 1.4
  reader takes an ITS-XML operational template, nothing in the pinned crates
  turns one into the other, and every one of the 321 ADL 2 halves declares
  itself `generated` from its 1.4 twin. What the library does prove is
  asserted over all 321 pairs: for the 300 whose ADL 2 half the reader reads,
  the internal model names the same concept, in the same language, over the
  same Reference Model class as the published ADL 1.4 twin.
- `docs/architecture.md` section 6.5 records the five things the replay's
  outcome classes left underdetermined, decided while implementing #19 and
  #20. The load-bearing one is that a `retyped` outcome is unreachable from
  the key alone, because a field's key step says `ELEMENT` while the field
  collects a `DV_*` class. Section 11 states where the line between
  `ferrochart-form` and `ferrochart-overlay` falls and why (#72).
- `docs/architecture.md` section 6.5 records why one document state is a
  refusal at authoring time and an advisory afterwards, and section 11 stops
  reading as though `ferrochart-form` were dependency-free. Two doc comments
  that had gone stale with the type move are corrected, and the hint map's
  ordering is asserted beside the type that has to keep it rather than in
  another crate's test suite.
- The book covers what the overlay carries, why its geometry is a column grid,
  and what a replay reports, and says which releases are published and what
  the compiler measurably does.
- The citation rule says that `docs/architecture.md` is citable, because it is
  the design of record and permanent, while a plan document is not. The rule
  banned "an internal markdown file" while justifying the ban with plan
  documents, which left every architecture citation in the overlay crate
  arguably against the rules. Also fixes a `codegen.md` reference to a
  `CLAUDE.md` heading that does not exist (#77) and a prose deferral where a
  `TODO` belongs (#78).
- The replay runs against the largest real difference the corpus can produce
  (#21): an overlay authored on every node of one CKM entry, replayed against
  a second entry for the same clinical concept. Nothing survives, and that is
  the point: the two differ in the pinned name of nearly every node, so 0 of
  171 entries match and the replay reports 7 moved, 164 disappeared and 181
  appeared rather than rebinding layout onto nodes that merely look similar.
  Every entry is still accounted for and every key still held.
- A printed node key carries every part that decides whether it matches (#83):
  the identifier, the Reference Model class, the pinned name and, where the
  step is not the first of its tied siblings, the ordinal. The shape follows
  the AQL node predicate. Before this, 40 distinct keys printed alike across
  two real CKM entries while none of them matched, so a replay report could
  show a person the same string twice and ask them to decide between them. A
  test over the whole corpus asserts that two keys printing alike are the same
  key.
- The layout overlay and its store (#19), in `ferrochart-overlay`. An overlay
  carries what no specification governs: field order, authored sections,
  labels, help text, defaults, conditional visibility and widget choice. It is
  stored apart from the compiled definition and keyed by the node key of
  `docs/architecture.md` section 6.2, whose every step carries the Reference
  Model attribute, the node id, the archetype id, the Reference Model type and
  the pinned name, with a sibling ordinal only where all five tie. The store
  normalizes a key against the definition rather than trusting it: an entry
  that decorates no node of the form is refused, and an entry whose key names
  several nodes is reported as positionally keyed instead of being resolved
  silently. It records which form the template identifier takes, because
  openEHR ITS-REST Release-1.1.0 `definition.html` resolves a partial
  identifier to the latest major version, and none of the 123 committed CKM
  templates pins a release in its own identifier. The serialisation is
  byte-deterministic and round-trips.
- The replay and the differential report (#20). A replay against a recompiled
  definition classifies every entry as matched, disappeared, moved, ambiguous,
  reordered or retyped, and names every node of the form that carries no
  layout yet. Nothing is discarded and nothing is rebound: an unmatched entry
  is retained against its old key so a later revision that restores the node
  restores its layout, and a move is a suggestion a person accepts. The report
  prints as text a form author can read, saying which entries survived, which
  need a decision and what changed about each. Authoring a layout on every one
  of the 4,447 nodes of the committed pack and replaying it comes back fully
  matched, and the 102 nodes in 4 templates that no key can tell apart are
  reported rather than guessed at.
- The overlay's geometry (#69), a column grid that stores no coordinate. A
  section declares a column count of 1 to 12, and an item carries a span, a
  break-before flag and a width hint in character units. Nothing stores a row
  or a column index: an item's place is the sibling order the overlay already
  carries plus its span and break, so the visual order of a form equals its
  source order by construction, which is what W3C WCAG 2.2 success criterion
  1.3.2 asks for. A span wider than its section is refused naming both
  numbers, a section wider than four columns is stored and advised against,
  and a renderer clamps a span to a narrower grid rather than the overlay
  dropping it, so one authored form serves a ward-round tablet and a desk. A
  per-item hint map carries what the grid cannot express, uninterpreted and
  outside the clean-replay guarantee. A replay reports what became of the
  geometry beside what became of the key: a span that no longer fits the
  section it lands in, and an entry whose section was dropped. The overlay
  format version is 2, and version 1 is refused rather than read as a form
  whose author chose one column.
- A landing page at the site root, with the book under `/docs/` (#92).
  `scripts/site/assemble.sh` puts `website/landing/` at `/`, the mark and the
  favicons from `assets/brand/` under `/assets/brand/`, and the built book
  under `/docs/`; the Docs workflow uploads that assembled directory. The
  book's `site-url` is `/docs/`, so its asset, search and 404 links resolve
  under the prefix, and its 404 page is promoted to the site root where
  GitHub Pages serves it for a miss anywhere. The page draws only on the
  "Rose & Iron" tokens and the contrast ratios `assets/brand/README.md`
  records, and it separates what is built from what is not: 121 of the 123
  vendored CKM templates derive a form, and nothing yet renders a form to a
  clinician, builds a COMPOSITION, or offers a surface for authoring layout.

### Changed

- The overlay's layout types moved from `ferrochart-overlay` to
  `ferrochart-form`, where `docs/architecture.md` section 11 puts them (#72).
  `Layout`, `Section`, `Visibility`, `Condition`, `WidgetName` and the column
  grid now live beside the definition they decorate, because a renderer applies
  a layout and has no business with the store, the authoring session or the
  replay, which read and write files and compare two definitions. The stored
  document is unchanged, byte for byte. `ferrochart-overlay` keeps the store,
  the key normalization, the replay and the report, and re-exports nothing it
  does not own, so a caller that needs a layout type names `ferrochart-form`.
  `scripts/checks/crate-closure.sh` is the new CI lane that holds the boundary:
  it walks the resolved dependency closure and fails when `ferrochart-form`
  links any other crate of this tree, or when `ferrochart-renderer` links
  anything but `ferrochart-form`.

### Fixed

- The compiler refuses a quantity magnitude or precision it cannot represent,
  naming the units and what it found, instead of reading part of the
  constraint and leaving the field wider than the template (#86).
- The compiler refuses a reference band stated on the end of an interval
  instead of dropping it (#87).
- The book at ferrochart.eu serves the brand favicon. It served mdBook's
  default while `assets/brand/favicon.svg` sat unused in the repository (#94).
- Every "Suggest an edit" link in the book. `edit-url-template` ended in
  `src/{path}` and mdBook expands `{path}` to a path that already begins with
  `src/`, so each link pointed at `website/book/src/src/…` and returned a 404.
- The book's introduction said there is no binary and listed the architecture
  and the gates as what exists. Three releases publish binaries for four Linux
  targets, and the compiler derives a form from 121 of the 123 committed
  templates. It now says what works and what does not.

## [0.0.3] - 2026-09-07

### Added

- Snapshots of the derived form over the whole vendored corpus (#18). An
  inventory carries one line per template with its group and field counts and
  the kinds it derives, so a change shows as one changed line naming the
  template rather than as a moved total; four whole documents are snapshotted
  beside it, from a 4-field form to a 303-field one. A test also derives twenty
  templates twice and compares the bytes, so the format's determinism claim is
  checked rather than trusted.
- The ADL 2 reader runs over the published archetype library (#49): 322
  archetypes parse, 306 flatten into an operational template, and 301 read.
  The 5 the reader refuses are mandatory unfilled slots, which is the
  adjudication on #13; the 16 it never sees are refused by the upstream
  flattener because a differential path in a specialised child does not
  resolve in its parent, in a 2013 export that predates the AOM 2 validity
  rules. The two families are counted apart, because they belong to different
  owners. CI fetches the library and sets `FERROCHART_REQUIRE_ADL2`, so a
  fetch that stops working is a red build rather than a skipped suite.
- A fetch script for the ADL 2 archetype library (#49), pinned to a commit of
  `openEHR/adl-archetypes`. It brings 322 ADL 2 archetypes and 330 ADL 1.4
  twins, 321 of them the same archetype in both dialects, which is the first
  real input for the property that both readers fill one internal constraint
  model. Nothing from it is committed: one of the 652 files states a licence
  and the rest state none, so the tree is fetched into a directory
  `.gitignore` refuses and `PROVENANCE.md` records the omission.
- The published form definition type (#17), in `ferrochart-form`. A definition
  is a tree of groups rooted at the template root; a group holds items, an item
  is a nested group or a field, and a field carries one of eighteen field
  kinds, one per thing a clinician can enter. Labels and help text are resolved
  per language from the archetype terminology, occurrences carry repeatability
  and optionality, an `ELEMENT` the template permits to be omitted carries the
  four null flavours openEHR RM Release-1.1.0 `data_structures.html`
  section 4.1 names, and a `default_value` prefills the field. The crate
  performs no I/O and depends on nothing else in the tree, so a third party can
  write a renderer against it. Every map in the format is ordered and every
  list keeps template order, so recompiling an unchanged template produces an
  identical document.
- The field derivation (#16), in `ferrochart-compile`: `docs/architecture.md`
  section 5 implemented against the internal constraint model, so it is written
  once for both ADL generations. Each permitted unit of a quantity carries its
  own magnitude bounds and decimal precision, so changing the unit changes
  both; a coded field's value set is enumerated with its rubrics where the
  archetype lists them and marked for expansion at render time where it only
  names them; an ordinal keeps list order, scores by its integer and stores its
  symbol's code; a date, time, date-and-time or duration pattern is honoured
  component by component; an occurrences upper bound above one makes an item
  repeatable; an `assumed_value` never reaches the data; and an open slot is
  recorded as undetermined content rather than rendered. A constraint the
  derivation does not understand is a typed error naming the node, never a
  permissive field. Each of the twelve cases `docs/architecture.md`
  section 5.2 lists as ungoverned carries a decision in code labelled as
  FerroCHART's own design. All 121 committed CKM templates the reader reads
  derive a form.

## [0.0.2] - 2026-09-07

### Added

- A `./.github/actions/setup-rust` composite action (#33). The six gating jobs
  in `ci.yml` and the coverage job in `sonar.yml` call it instead of each
  carrying its own pinned toolchain step. The release lane keeps its own,
  because a publishing lane restores no cache, and now says so at the step.
- The internal constraint model and the two template readers that fill it
  (#13, #14), in `ferrochart-compile`. Both ADL generations normalize into one
  model, so the field derivation is written once and nothing above that point
  can tell which reader produced a node: each node carries a node identity, a
  Reference Model type, an occurrences interval, a constraint payload, the
  terminology in scope, and the five parts an overlay key step needs
  (`docs/architecture.md` sections 3 and 6.2). The ADL 1.4 reader walks a
  parsed `.opt`, resolving `use_node` internal references the vendor flattener
  left in place and reading the constrainers only the ITS-XML family declares
  (`C_DV_STATE`, `C_CODE_REFERENCE`, `T_COMPLEX_OBJECT`). The ADL 2 reader
  starts from source archetypes, flattens them and generates the operational
  form itself, folding the `C_PRIMITIVE_TUPLE` shapes for quantity and ordinal
  into the same payloads the ADL 1.4 domain types produce, and carrying
  `constraint_status`, which ADL 1.4 cannot state. A node with
  `occurrences matches {0}` is absent from the model, and a template that
  requires content at a slot it never filled is refused with a typed error
  naming the slot rather than rendered as a guess. 121 of the 123 committed CKM
  templates read; the two refusals are required unfilled slots.
- A crate-scoped `deny.toml` licence exception for `openehr-term`, which
  embeds the openEHR support terminology under CC-BY-SA-3.0 and reaches this
  tree through `openehr-its` and `openehr-adl`.
- The release supply chain (#39, #34): a `v*` tag now publishes a container
  image, SBOMs, checksums and Sigstore attestations beside the binaries, and a
  quickstart `compose.yaml` a downloader can run without a clone. The binaries
  and the image are built in reusable workflows called by jobs that carry no
  steps of their own, which is what the SLSA Build Level 3 claim rests on;
  `cargo auditable` writes the dependency list into every shipped binary,
  CycloneDX describes it beside the archive, and syft describes each image
  platform from that embedded list. A consumer verifies with
  `gh attestation verify --signer-workflow`, and `finalize-release` refuses to
  publish a draft missing any of the 27 assets a four-target release promises.
  The compose file's `demo` profile starts FerroEHR and FerroTERM alongside, and
  its header says plainly that those are separately licensed products and that
  the profile is for evaluation.
- The server's runtime surface (#41): one `FERROCHART_` environment namespace
  read in one place, a loopback default bind that only the container image
  widens, a refusal to start when either upstream endpoint is missing that
  names the variable, an unauthenticated `GET /health`, and orderly shutdown
  on SIGTERM. The book gains an Operate section describing all of it.
- The documentation lane (#4): a book under `website/book` rendered by the
  pinned mdBook toolchain that `.github/actions/docs-toolchain` installs, and
  `docs.yml`, which verifies it on every pull request and publishes it to
  GitHub Pages from `main`. `docs/VERSIONS.md` carries the three tool pins and
  `scripts/checks/versions.sh` stops skipping that check.
- The vendored openEHR CKM template corpus, licence by licence (#12).

### Changed

- `.claude/rules/ci-cd.md` and `docs/ci-cd.md` describe the pipeline that
  exists rather than the one that was waiting for a workspace, and drop a
  stale issue reference carried over from another repository.

### Fixed

- Status text that had gone stale as the repository gained code: the README no
  longer says there is no code and no binary, `docs/ci-cd.md` no longer calls
  the Rust tier gated off, `scripts/checks/versions.sh` and four `.claude`
  rules no longer describe a design phase that closed, and a reference to
  issue #20 that meant a different repository's issue now names #33.

## [0.0.1] - 2026-09-06

### Added

- The Rust CI tier, live now that the workspace exists (#15): `rustfmt`,
  `clippy` with `-D warnings`, `cargo nextest`, doctests, `rustdoc`, an MSRV
  check through `cargo-hack`, and `cargo deny`, all under the one required
  `conclusion` check, with SonarQube importing Rust coverage from an
  instrumented run.
- The Cargo workspace (#11): the eight crates of `docs/architecture.md` §11,
  the workspace lint table from `.claude/rules/reliability.md` with
  `unsafe_code` forbidden, a release profile that keeps `panic = "unwind"` and
  `overflow-checks`, and a separate `wasm-release` profile for the renderer.
  `deny.toml`'s licence exceptions now name the real crates, including the
  three pinned openEHR crates that are BUSL-1.1.
- How a coded field gets its codes, decided against measured evidence (#6):
  97.6% of the 843 coded fields in 102 operational templates take their
  membership from the template, an enumerated external code carries no rubric
  there, and one code path serves every binding kind by resolving locally and
  asking a terminology server only for what is missing. Archetype value sets
  are emitted as FHIR `CodeSystem`, `ValueSet` and `ConceptMap` under a minted
  URL, because no openEHR specification defines one.
- The overlay key, decided against measured evidence (#8): a step chain
  carrying the RM attribute, the `node_id`, the archetype id, the RM type and
  the pinned name, with a sibling ordinal only where those tie. Walking 102
  operational templates showed an id-only path collides in 14 of them, and
  that the pinned name is the only discriminator for 41% of the 427 colliding
  sibling groups, which reversed the earlier decision to keep the name out of
  the key.
- The design of record, `docs/architecture.md` (#1): the version pins, both
  template generations normalized into one internal constraint model, the
  field derivation table from the Reference Model and its constraints, the
  layout overlay with its key normalization and its replay report, the
  terminology split between local and network resolution, the ITS-REST client
  rules, the workspace layout, the build order, and a decision register.
  Every decision carries a citation or an explicit note that no specification
  governs it.
- The specification and model-crate pins in `docs/VERSIONS.md`, which now
  carry values instead of `pending #1`, and `scripts/checks/versions.sh` also
  checks `openehr-am` and `openehr-adl` (#9).
- The brand: the mark, the "Rose & Iron" palette, and the lockup, favicon and
  social-card set (#3). Rose was chosen by measuring worst-case CIEDE2000
  distance from the other three product hues under normal, deuteranope and
  protanope vision, and `assets/brand/README.md` records the ratio for every
  token against both grounds.
- The repository: the working discipline in `.claude/` (rules, hooks, skills,
  agents, memory), the community and governance documents, the pinned version
  matrix in `docs/VERSIONS.md`, the committed guards under `scripts/checks/`,
  the tracker helpers under `scripts/gh/`, and five workflows that work on a
  repository with no code (CI, CodeQL, Scorecard, SonarQube Cloud, Release).
  The configuration is FerroBRIDGE's, adapted from the FHIR and OMOP oracles to
  the openEHR Reference Model, the Archetype Object Model, ITS-REST, and AQL.

[Unreleased]: https://github.com/rubentalstra/FerroCHART/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.5...v0.1.0
[0.0.5]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/rubentalstra/FerroCHART/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/rubentalstra/FerroCHART/releases/tag/v0.0.1
