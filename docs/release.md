<!-- SPDX-FileCopyrightText: Ruben Talstra -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Cutting a release

A milestone is a delivery promise, and a release is cut when its milestone
reaches zero open issues. This page is the checklist the cut follows, what a
release publishes, how to verify one as a consumer, and what a published
release is and is not protected against. It is written from the workflows as
they are: `.github/workflows/release.yml` and the two reusable lanes it calls,
`release-build.yml` and `release-image.yml`.

No specification governs the lane itself; it is FerroCHART's own design. The
discipline every workflow here obeys is `.claude/rules/ci-cd.md`.

## What the lane does

`release.yml` is dormant until a `v*` tag is pushed. It does not run on a push
to `main`, on a pull request, or in a merge group. `workflow_dispatch` re-runs
it for a tag that already exists and has to be dispatched at that tag.

```text
plan ─ github-release (draft) ─ build-binaries ─ build-image ─ finalize-release
```

- **plan** validates the tag shape, refuses a dispatch that is not at the tag
  it names, checks the tag against every file that declares the product
  version, and extracts the `## [X.Y.Z]` section of `CHANGELOG.md` as the
  release notes. A missing or empty section fails the release, so a cut can
  never ship with notes generated from the commit range standing in for the
  changelog. It also emits the target matrix, once, which both the build
  matrix and the asset check read.
- **github-release** creates the release as a draft carrying those notes. A
  draft is mutable and invisible to anyone browsing releases, which is the
  window the asset uploads need.
- **build-binaries** is a call to `release-build.yml`, once per target, and
  carries no steps of its own. Four Linux targets, each on a runner of its own
  architecture, with no cross toolchain and no QEMU.
- **build-image** is a call to `release-image.yml`. It takes the two musl
  archives out of the run's artifacts, verifies each against the build lane's
  signer identity before it unpacks anything, and builds and pushes the
  two-platform image.
- **finalize-release** checks that the draft carries every asset this version
  promises and publishes only then, so a half-assembled release is never
  visible.

Both lanes build cold. `setup-rust-toolchain` runs with `cache: false` and the
image build with `no-cache: true`, because nothing a prior run could have
poisoned may influence an artifact this lane signs.

Concurrency is `cancel-in-progress: false`. A second tag push queues behind the
first, because a release cancelled part-way through publishing is worse than a
slow one.

## What a release publishes

Per target, six assets on the archive's name, where `<target>` is one of
`x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-gnu` and `aarch64-unknown-linux-musl`:

| Asset | What it is |
|---|---|
| `ferrochart-<tag>-<target>.tar.gz` | the binaries, built with `cargo auditable` |
| `ferrochart-<tag>-<target>.tar.gz.sha256sum` | a `sha256sum -c` line, bare filename |
| `ferrochart-<tag>-<target>.tar.gz.sigstore.json` | the build-provenance bundle |
| `ferrochart-<tag>-<target>.tar.gz.sbom.sigstore.json` | the SBOM-attestation bundle |
| `ferrochart-<tag>-<target>.tar.gz.intoto.jsonl` | the provenance DSSE envelope, one per line |
| `ferrochart-<tag>-<target>.cdx.json` | a CycloneDX 1.5 SBOM of the binary |

Plus three release-wide assets: `compose.yaml`, `compose.yaml.sha256sum`, and
`compose.yaml.sigstore.json`. That is 27 assets, and `finalize-release`
refuses to publish a draft missing any one of them.

The `.sha256sum` is not a signature. It detects a corrupt or truncated
download and nothing else; only the Sigstore bundle answers who built the file.

Four kinds of dependency document are produced, and they answer different
questions:

| Document | Tool | Where it lives |
|---|---|---|
| the binary's own `.dep-v0` section | `cargo-auditable` | inside every shipped binary |
| `ferrochart-<tag>-<target>.cdx.json` | `cargo-cyclonedx`, CycloneDX 1.5 | a release asset, and an attested subject |
| `ferrochart-<tag>-image-<arch>.spdx.json` | syft, SPDX 2.3 | an OCI referrer attestation only, never a release asset |
| BuildKit's own provenance | buildx `provenance: mode=max` | attestation manifests inside the image index |

Every tool version is pinned, and `scripts/checks/versions.sh` fails when a
workflow and `docs/VERSIONS.md` disagree. A tool that runs inside the isolated
lane writes a document that lane then signs, so an unpinned tool would be an
unsigned input to a signed artifact.

## Verifying a release as a consumer

A binary archive, insisting it came from the hardened lane rather than from
any workflow in the repository:

```sh
gh attestation verify ferrochart-<tag>-<target>.tar.gz \
  -R rubentalstra/FerroCHART \
  --signer-workflow rubentalstra/FerroCHART/.github/workflows/release-build.yml
```

The image, where the tag resolves to the index, and the SPDX SBOM of one
platform manifest:

```sh
gh attestation verify oci://ghcr.io/rubentalstra/ferrochart:<version> \
  -R rubentalstra/FerroCHART \
  --signer-workflow rubentalstra/FerroCHART/.github/workflows/release-image.yml
gh attestation verify oci://ghcr.io/rubentalstra/ferrochart@<manifest digest> \
  -R rubentalstra/FerroCHART --predicate-type https://spdx.dev/Document/v2.3
```

The quickstart compose file, which is signed by the image lane because it names
the image that lane pushed:

```sh
gh attestation verify compose.yaml \
  -R rubentalstra/FerroCHART \
  --signer-workflow rubentalstra/FerroCHART/.github/workflows/release-image.yml
```

Without a network path to Sigstore, `sha256sum -c` against the `.sha256sum`
beside a file is the offline floor, and the `.sigstore.json` bundles are
attached so that `gh attestation verify --bundle` works offline too.

## The SLSA claim, and what it does not claim

**The claim is SLSA Build Level 3**, on the build requirements of SLSA v1.2
(<https://slsa.dev/spec/v1.2/build-requirements>).

What backs it is one clause. Build L3 requires that the secret material
authenticating the provenance is not accessible to the environment running the
user-defined build steps. Every step of a GitHub Actions job shares one runner
VM, so attesting inside an ordinary job reaches Build L2 at best. The build and
its attestation therefore live in reusable workflows, which run on their own VM
and whose steps a caller cannot add to, and the Fulcio certificate names that
workflow
(<https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-and-reusable-workflows-to-achieve-slsa-v1-build-level-3>).
The structural enforcement is that a `uses:` job cannot carry `steps:`, so
inlining the build back into `release.yml` is unrepresentable rather than
merely discouraged.

What is deliberately not claimed:

- **Reproducible builds and hermetic builds.** Those are separate SLSA tracks,
  and nothing here measures either.
- **Any guarantee at consumption time.** The claim is about how artifacts are
  produced. `--signer-workflow` is enforced in exactly one place, when the
  image lane verifies the archives it is about to unpack, and in that lane's
  check of its own output. There is no admission controller and no
  verification step in any deployment path.
- **Anything about the sibling images the compose file's demo profile pulls.**
  They are other products' releases, verified against their own repositories.

Signing is keyless Sigstore OIDC through `actions/attest`, which is the one
attestation action used here, in both of its modes: no `sbom-path` is
provenance mode, `sbom-path` is SBOM mode. There is no long-lived key
anywhere, and cosign is not in the release path. The lanes need
`id-token: write` and `attestations: write`, declared on the reusable job and
mirrored on the caller; the repository setting they depend on is tracked in the
owner-actions table of `docs/ci-cd.md`.

## The container image

`ghcr.io/rubentalstra/ferrochart`, an index over `linux/amd64` and
`linux/arm64`, tagged `<version>`, `<major>.<minor>`, and `latest` (which the
metadata action skips for a pre-release). GHCR tags are mutable, so a
deployment pins the digest the lane prints.

The image is `docker/Dockerfile`: a single stage over a digest-pinned
`gcr.io/distroless/static-debian13:nonroot`, copying the musl binary the build
lane attested out of a staged `dist/<os>/<arch>/` tree. Nothing executes while
the image is assembled, which is why the arm64 manifest needs no QEMU. It runs
as numeric uid 65532, has no shell and no package manager, and carries no
`HEALTHCHECK`: probe it over HTTP from outside.

## The quickstart compose file

`compose.yaml` is attached to every release verbatim from the tag's checkout.
A downloader needs no clone:

```sh
curl -LO https://github.com/rubentalstra/FerroCHART/releases/latest/download/compose.yaml
```

Its profile-less path runs FerroCHART alone against a CDR and a terminology
server the operator supplies, and refuses to start when either is missing. Its
`demo` profile also starts FerroEHR, its PostgreSQL, and FerroTERM, which are
three separately licensed BUSL-1.1 products; the file's header says so, says
that the profile carries development credentials, and says what a terminology
server with no index can and cannot expand.

The image tag in that file is a committed default, not a value templated at
release time, so the published file is byte-identical to the committed one.
`scripts/checks/versions.sh` keeps that default equal to the workspace version
and refuses any floating `latest` tag in the file.

## Before the tag

1. **The milestone is empty.** `gh issue list --milestone vX.Y.Z --state open`
   answers nothing, or the owner calls the cut and moves the stragglers to the
   next milestone.
2. **The version moves in every file the pin matrix names.** The root
   `Cargo.toml` `[workspace.package]` `version`, `CITATION.cff`, the
   product-version row of `docs/VERSIONS.md`, and the image tag default in
   `compose.yaml`. `scripts/checks/versions.sh` fails on any file left behind,
   and the `plan` job checks the same files against the tag.
3. **The changelog names the release.** `[Unreleased]` becomes the version and
   the date, with a fresh empty `[Unreleased]` above it and a new link
   reference. What sits under the version heading is what the release notes
   say, so read it as the release notes before you tag.
4. **The gates pass on the release commit.** The tier-1 set is
   `zizmor --min-severity=low .github/`, `actionlint`, `shellcheck
   --severity=style` over every tracked shell program, `hadolint` over every
   tracked Dockerfile, `scripts/checks/comment-style.sh --all`, and
   `scripts/checks/versions.sh`. The Rust set is `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
   `cargo nextest run --workspace --locked`, `cargo test --doc`, `cargo doc`
   with `RUSTDOCFLAGS=-D warnings`, `cargo deny check`, and
   `cargo hack check --rust-version`.

## The tag

The signed tag is the owner's:

```sh
git tag -s vX.Y.Z -m "vX.Y.Z" <the merged release commit>
git push origin vX.Y.Z
```

The `release-tags` ruleset requires a signature on `refs/tags/v*`, so an
unsigned tag is refused at push time. `release.yml` takes it from there.

A first cut of a new lane is worth rehearsing on a pre-release tag. A tag
carrying a suffix, `vX.Y.Z-rc.1`, publishes as a pre-release and runs the whole
path for real, including the attestations and the registry push.

## After the tag

1. **Verify the published release as a consumer**, with the commands above.
   The lane already verified the image the way a consumer would, but the
   binaries are only checked at the release page's asset list.
2. **Read the published release.** Its notes are the changelog section, and its
   asset list is what `finalize-release` demanded.
3. **Close the milestone.** `gh api -X PATCH
   repos/rubentalstra/FerroCHART/milestones/<n> -f state=closed`. A milestone
   is a delivery promise, and one left open after its release is cut says the
   promise is still outstanding. Nothing closes it automatically, and both
   cuts so far needed it done by hand.
4. **Post the board status update** with what shipped and what the next
   milestone targets (`.claude/rules/project-board.md`).

## What a published release is protected against

Two protections cover different things, and both are on.

**The release is frozen.** GitHub's repository-level immutable-releases setting
is enabled, so once a release is published its notes and its assets cannot be
edited. That is why the lane assembles a draft and publishes last: the draft is
the only window in which assets can still be attached.

**The tag is protected.** The `release-tags` ruleset is active on
`refs/tags/v*` and blocks `deletion` and `non_fast_forward` updates, and
requires signatures. A pushed `vX.Y.Z` cannot be moved to another commit and
cannot be deleted, so the commit a release names stays the commit it was cut
from.

Read the notes before you tag. Once the release publishes, the text you wrote
is the text that stands.

**The no-retag rule is ours, not the platform's.** A bad cut ships forward as a
new patch version. Never move a tag, never delete a release and recreate it,
and never edit a published release's notes to fix what the changelog got wrong;
fix the changelog and cut the next version. The lane enforces the half it can:
`github-release` refuses to reopen an already-published release for the same
tag, and fails with that message instead.

## Sources

- SLSA v1.2 build requirements: <https://slsa.dev/spec/v1.2/build-requirements>
- Artifact attestations and reusable workflows for SLSA Build Level 3:
  <https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-and-reusable-workflows-to-achieve-slsa-v1-build-level-3>
- `cargo auditable`: <https://github.com/rust-secure-code/cargo-auditable>
- Docker build annotations:
  <https://docs.docker.com/build/ci/github-actions/annotations/>
- Available rules for rulesets:
  <https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets>
- Managing releases:
  <https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository>
- GitHub Actions security hardening:
  <https://docs.github.com/en/actions/security-for-github-actions/security-hardening-for-github-actions>
