#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# Fetch the ADL 2 archetype library, with its ADL 1.4 twins.
#
# Source: https://github.com/openEHR/adl-archetypes, the directory
# `Reference/CKM_2013_12_09/`, which upstream describes as archetypes exported
# from the Clinical Knowledge Manager on 2013-12-09. Pinned by commit.
#
# WHY NOT CKM: the live openEHR CKM publishes ADL 1.4 only. Its archetype
# export endpoints answer `adl` and `xml`; `adl2`, `adls` and `opt` all 404
# (verified on the wire 2026-09-07). So the ADL 2 side has to come from a
# pinned library, and it is never produced by running an ADL 1.4 to 2
# converter of our own: that would validate a reader against its own output.
#
# WHAT MAKES THIS PACK WORTH FETCHING: it carries the same clinical archetype
# in both dialects. That is the only real input the matched-pair property has,
# which is that both readers fill one internal constraint model equivalently
# (docs/architecture.md section 3).
#
# NOTHING HERE IS COMMITTED. The upstream repository has no licence file, and
# of its 652 archetypes exactly one states a licence of its own; the rest
# carry a copyright line and nothing that grants redistribution. Silence is
# not permission (.claude/rules/vendored-inputs.md), so the tree lands in a
# directory .gitignore refuses and this repository ships none of it. Run this
# script to get it locally, and CI runs it too.
#
# Usage:
#   scripts/vendor/adl2-archetypes.sh
set -Eeuo pipefail

REPO="openEHR/adl-archetypes"
# The current head of the upstream default branch, 2025-06-27. The tree is a
# published reference export and has not moved in years; re-pin deliberately.
COMMIT="093c77ea003742b9540e3dd377d615e2b26f2996"
SUBDIR="Reference/CKM_2013_12_09"
OUT="corpus/archetypes/adl2"

command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v tar >/dev/null || { echo "tar is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

cd "$(git rev-parse --show-toplevel)"

work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

echo "== fetching $REPO at $COMMIT"
curl -fsSL --max-time 300 \
  "https://codeload.github.com/$REPO/tar.gz/$COMMIT" -o "$work/pack.tar.gz"

echo "== extracting $SUBDIR"
mkdir -p "$OUT"
find "$OUT" -maxdepth 2 \( -name '*.adls' -o -name '*.adl' \) -delete
tar xzf "$work/pack.tar.gz" --strip-components=3 -C "$OUT" \
  "$(basename "$REPO")-$COMMIT/$SUBDIR"

adls="$(find "$OUT" -name '*.adls' | wc -l | tr -d ' ')"
adl="$(find "$OUT" -name '*.adl' | wc -l | tr -d ' ')"
echo "   ADL 2: $adls, ADL 1.4 twins: $adl"
[ "$adls" -gt 100 ] || { echo "only $adls ADL 2 files: the upstream layout changed" >&2; exit 1; }

echo "== writing $OUT/PROVENANCE.md"
python3 - "$OUT" "$REPO" "$COMMIT" "$SUBDIR" <<'PY'
import datetime
import glob
import os
import re
import sys

out, repo, commit, subdir = sys.argv[1:5]
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def licence_of(path):
    """The licence the archetype states, if it states one."""
    text = open(path, encoding="utf-8", errors="replace").read(40000)
    for pattern in (r'\["licence"\]\s*=\s*<"([^"]*)"', r'licence\s*=\s*<"([^"]*)"'):
        m = re.search(pattern, text)
        if m:
            return " ".join(m.group(1).split())
    return ""


adls = sorted(glob.glob(os.path.join(out, "**", "*.adls"), recursive=True))
adl = sorted(glob.glob(os.path.join(out, "**", "*.adl"), recursive=True))


def concept(path):
    """The archetype identity without its dialect or its patch version.

    ADL 2 names a file `...v1.0.0.adls` and ADL 1.4 names its twin `...v1.adl`
    (openEHR AM Release-2.3.0 `Identification.html`), so a comparison on the
    bare stem finds nothing.
    """
    name = os.path.basename(path)
    name = name[: name.rfind(".")]
    return re.sub(r"\.v(\d+)(?:\.\d+)*$", r".v\1", name)


paired = {concept(p) for p in adls} & {concept(p) for p in adl}
stated = [(p, licence_of(p)) for p in adls + adl]
with_licence = [(p, v) for p, v in stated if v]

lines = [
    "<!-- SPDX-FileCopyrightText: Ruben Talstra -->",
    "<!-- SPDX-License-Identifier: BUSL-1.1 -->",
    "",
    "# The ADL 2 archetype pack: provenance",
    "",
    f"Fetched from <https://github.com/{repo}>, directory `{subdir}`, at commit",
    f"`{commit}`, by `scripts/vendor/adl2-archetypes.sh` on {now}.",
    "",
    "Upstream describes the tree as archetypes exported from the openEHR",
    "Clinical Knowledge Manager on 2013-12-09.",
    "",
    f"- ADL 2 archetypes (`*.adls`): {len(adls)}",
    f"- ADL 1.4 twins (`*.adl`): {len(adl)}",
    f"- Archetypes present in both dialects: {len(paired)}",
    "",
    "## Why this source and not CKM",
    "",
    "The live openEHR CKM publishes ADL 1.4 only. Its archetype export answers",
    "`adl` and `xml`, and `adl2`, `adls` and `opt` all return 404, verified on",
    "the wire on 2026-09-07. The ADL 2 side therefore comes from this pinned",
    "library.",
    "",
    "It is never produced by running an ADL 1.4 to ADL 2 converter of our own.",
    "That would validate the ADL 2 reader against output this project",
    "generated, which proves nothing about either.",
    "",
    "## Nothing here is committed",
    "",
    "The upstream repository carries no licence file. Of the",
    f"{len(adls) + len(adl)} archetypes in this tree, {len(with_licence)} states a",
    "licence of its own; every other file carries a copyright line and nothing",
    "that grants redistribution.",
    "",
    "Silence is not permission (`.claude/rules/vendored-inputs.md`), so this",
    "repository ships none of it. `.gitignore` refuses the archetype files, and",
    "this record is committed in their place so the omission is visible. Run",
    "the fetch script to get the tree locally.",
    "",
    "This is a stricter reading than a sibling project applied to the same",
    "upstream, and it is deliberate: a provenance record is a legal record, and",
    "an unstated licence is not a permissive one.",
    "",
    "## The one file that states a licence",
    "",
]
if with_licence:
    lines += ["| file | licence as stated |", "|---|---|"]
    for path, value in sorted(with_licence):
        lines.append(f"| `{os.path.relpath(path, out)}` | {value[:100]} |")
else:
    lines.append("None.")
lines += [
    "",
    "## No patient data",
    "",
    "These are clinical models rather than clinical data. Nothing here is a",
    "record about a person, and nothing in this repository's tests may be",
    "(`.claude/rules/testing.md`).",
    "",
]
open(os.path.join(out, "PROVENANCE.md"), "w").write("\n".join(lines))
print(f"   {len(paired)} matched pairs, {len(with_licence)} file states a licence")
PY
echo "== done"
