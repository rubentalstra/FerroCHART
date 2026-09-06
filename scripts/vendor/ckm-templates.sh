#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# Vendor the openEHR CKM template library as OPT 1.4 XML.
#
# Source: the public openEHR Clinical Knowledge Manager REST API
# (https://ckm.openehr.org/ckm/rest/v1). Every file is CKM's own Operational
# Template export, vendored verbatim (.claude/rules/vendored-inputs.md).
#
# These templates are what the compiler is measured against: the field
# derivation table, the form definition snapshots and the overlay key all read
# this tree.
#
# LICENCE SPLIT: CKM publishes no repository-level licence, and each export
# either carries an `other_details id="licence"` statement or carries nothing.
# A file stating CC-BY-SA is redistributable and is committed. A file stating
# nothing is NOT redistributed: silence is not permission
# (.claude/rules/vendored-inputs.md), so it lands under `unstated/`, which
# .gitignore refuses. Fetch it locally for breadth; never commit it.
#
# PAGINATION: the list endpoints page with `?size=M&offset=N`. `page`,
# `limit`, `pageSize` and `count` are silently ignored and you get a 20-row
# first page, which reads exactly like "CKM publishes 20 templates". The fetch
# below asserts the count grew.
#
# PINNING: CKM is not a git repository and publishes no tags, so a commit pin
# is not available. Each template is pinned by its `cid`, which carries the
# asset version, together with `resourceMainId` and `versionAsset`. Those three
# are immutable for a given version and are recorded per file in PROVENANCE.md.
#
# Usage:
#   scripts/vendor/ckm-templates.sh              # fetch the library
#   CKM_JOBS=8 scripts/vendor/ckm-templates.sh   # parallel fetches (default 4)
set -Eeuo pipefail

CKM="https://ckm.openehr.org/ckm/rest/v1"
OUT="corpus/templates/ckm"
JOBS="${CKM_JOBS:-4}"
PAGE=200

# The xargs worker. Not a user-facing mode.
if [[ "${1:-}" == "--fetch-one" ]]; then
  cid="$2"
  dest="$3"
  for _ in 1 2 3; do
    if curl -fsS --max-time 240 "$CKM/templates/$cid/opt" \
        -H "Accept: application/xml" -o "$dest"; then
      # A CKM error page is a 200 carrying HTML, so check the payload itself.
      if head -c 2048 "$dest" | grep -q "<template"; then
        exit 0
      fi
      rm -f "$dest"
    fi
    sleep 2
  done
  echo "FAILED $cid" >&2
  exit 1
fi

command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

cd "$(git rev-parse --show-toplevel)"

echo "== listing the CKM template library"
work="$(mktemp -d)"
trap 'rm -f "$work"/*; rmdir "$work"' EXIT
offset=0
: > "$work/parts"
while :; do
  curl -fsS --max-time 120 "$CKM/templates?size=$PAGE&offset=$offset" \
    -H "Accept: application/json" -o "$work/page"
  count="$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$work/page")"
  cat "$work/page" >> "$work/parts"
  printf '\n' >> "$work/parts"
  echo "   offset $offset: $count"
  [ "$count" -lt "$PAGE" ] && break
  offset=$((offset + PAGE))
done

python3 - "$work/parts" "$work/list.json" <<'PY'
import json, sys

parts, out = sys.argv[1], sys.argv[2]
rows = []
for chunk in open(parts).read().split("\n"):
    chunk = chunk.strip()
    if chunk:
        rows.extend(json.loads(chunk))
seen, uniq = set(), []
for r in sorted(rows, key=lambda r: r["cid"]):
    if r["cid"] not in seen:
        seen.add(r["cid"])
        uniq.append(r)
json.dump(uniq, open(out, "w"), indent=1, sort_keys=True)
print(f"   {len(uniq)} templates listed")
PY

total="$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' "$work/list.json")"
[ "$total" -gt 20 ] || { echo "only $total listed: the pagination contract regressed" >&2; exit 1; }

echo "== fetching $total operational templates into $OUT"
# Clear only what this script writes. A stray file stays visible rather than
# being swept away by a recursive delete.
mkdir -p "$OUT"
find "$OUT" -maxdepth 1 -name '*.opt' -delete
rm -f "$OUT/PROVENANCE.md"

python3 - "$work/list.json" "$OUT" > "$work/jobs" <<'PY'
import json, re, sys

rows = json.load(open(sys.argv[1]))
out = sys.argv[2]
used = set()
for r in rows:
    slug = re.sub(r"[^a-z0-9]+", "-", r["resourceMainDisplayName"].lower()).strip("-")
    slug = slug or r["cid"].replace(".", "-")
    base, n = slug, 2
    while slug in used:
        slug, n = f"{base}-{n}", n + 1
    used.add(slug)
    print(f"{r['cid']}\t{out}/{slug}.opt")
PY

tr '\t' '\n' < "$work/jobs" | xargs -P "$JOBS" -n 2 "$0" --fetch-one || true

got="$(find "$OUT" -maxdepth 1 -name '*.opt' | wc -l | tr -d ' ')"
echo "   fetched $got of $total"

echo "== sorting by the licence each export states"
mkdir -p "$OUT/unstated"
find "$OUT/unstated" -maxdepth 1 -name '*.opt' -delete
python3 - "$OUT" <<'PY2'
import glob, os, re, shutil, sys

out = sys.argv[1]
moved = kept = 0
for path in sorted(glob.glob(os.path.join(out, "*.opt"))):
    head = open(path, encoding="utf-8", errors="replace").read(60000)
    m = re.search(r'<other_details id="licence">(.*?)</other_details>', head, re.S)
    text = " ".join(m.group(1).split()) if m else ""
    if "by-sa" in text.lower() or "ShareAlike" in text:
        kept += 1
    else:
        shutil.move(path, os.path.join(out, "unstated", os.path.basename(path)))
        moved += 1
print(f"   {kept} redistributable, {moved} unstated (not committed)")
PY2

echo "== writing $OUT/PROVENANCE.md"
python3 - "$work/list.json" "$work/jobs" "$OUT" <<'PY'
import datetime
import json
import os
import re
import sys

rows = {r["cid"]: r for r in json.load(open(sys.argv[1]))}
jobs = [line.split("\t") for line in open(sys.argv[2]).read().splitlines() if line]
out = sys.argv[3]
now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def licence_of(path):
    """The licence the export states, in `other_details id="licence"`."""
    try:
        head = open(path, encoding="utf-8", errors="replace").read(60000)
    except OSError:
        return "unreadable"
    m = re.search(r'<other_details id="licence">(.*?)</other_details>', head, re.S)
    if not m:
        return "not stated"
    text = " ".join(m.group(1).split())
    if "by-sa/4.0" in text or "ShareAlike 4.0" in text:
        return "CC-BY-SA-4.0"
    if "by-sa/3.0" in text or "ShareAlike 3.0" in text:
        return "CC-BY-SA-3.0"
    return text[:80]


def located(dest):
    """Where the sort put this file, or None when it never arrived."""
    unstated = os.path.join(os.path.dirname(dest), "unstated", os.path.basename(dest))
    if os.path.exists(dest):
        return dest
    if os.path.exists(unstated):
        return unstated
    return None


placed = [(cid, dest, located(dest)) for cid, dest in jobs]
present = [(cid, p) for cid, _, p in placed if p and "unstated" not in p]
withheld = [(cid, p) for cid, _, p in placed if p and "unstated" in p]
missing = [(cid, dest) for cid, dest, p in placed if p is None]

lines = [
    "<!-- SPDX-FileCopyrightText: Ruben Talstra -->",
    "<!-- SPDX-License-Identifier: BUSL-1.1 -->",
    "",
    "# The openEHR CKM template pack: provenance",
    "",
    "Vendored verbatim from the openEHR Clinical Knowledge Manager REST API,",
    "<https://ckm.openehr.org/ckm/rest/v1>, by",
    "`scripts/vendor/ckm-templates.sh`. Each file is CKM's own Operational",
    "Template export for the cited template.",
    "",
    f"- Fetched: {now}",
    f"- Templates listed by CKM: {len(jobs)}",
    f"- Committed here, licence stated: {len(present)}",
    f"- Withheld, licence not stated: {len(withheld)}",
    f"- Unreachable at fetch time: {len(missing)}",
    "",
    "## Pinning",
    "",
    "CKM is not a git repository and publishes no tags, so a commit pin is not",
    "available. Each template is pinned by its `cid`, which carries the asset",
    "version, together with the resource id and the asset version number. Those",
    "are immutable for a given version of a template, so a re-run that returns a",
    "different `cid` for the same file is a real change rather than a moving",
    "reference.",
    "",
    "## Licensing",
    "",
    "CKM publishes no repository-level licence, and each export either states",
    "one in `other_details id=\"licence\"` or states nothing.",
    "",
    "**A file stating a Creative Commons licence is committed here.** Those are",
    "CC-BY-SA-4.0 and CC-BY-SA-3.0, whose terms are at",
    "<https://creativecommons.org/licenses/by-sa/4.0/> and",
    "<https://creativecommons.org/licenses/by-sa/3.0/>. Attribution rides along",
    "in each file, which is vendored verbatim and never edited.",
    "",
    "**A file stating nothing is not redistributed.** Silence is not",
    "permission (`.claude/rules/vendored-inputs.md`), so the fetch puts those",
    "under `unstated/`, which `.gitignore` refuses. Run the script to get them",
    "locally when you want the full breadth; they are never committed and never",
    "published.",
    "",
    "Nothing here is relicensed. The project's own code and text are BUSL-1.1",
    "(`CLAUDE.md`); this tree is not.",
    "",
    "## No patient data",
    "",
    "These are clinical models rather than clinical data. Nothing here is a",
    "record about a person, and nothing in this repository's tests may be",
    "(`.claude/rules/testing.md`).",
    "",
    "## The pack",
    "",
    "| file | cid | resource id | asset version | status | licence as stated |",
    "|---|---|---|---|---|---|",
]
for cid, dest in sorted(present, key=lambda p: p[1]):
    r = rows[cid]
    lines.append(
        f"| `{os.path.basename(dest)}` | `{cid}` | `{r['resourceMainId']}` | "
        f"{r.get('versionAsset', '')} | {r.get('status', '')} | {licence_of(dest)} |"
    )
if withheld:
    lines += [
        "",
        "## Withheld: the export states no licence",
        "",
        "CKM lists these and returns them, and they carry no licence statement,",
        "so this repository ships none of them. The fetch script writes them to",
        "`unstated/` for local use.",
        "",
        "| cid | file | status |",
        "|---|---|---|",
    ]
    for cid, path in sorted(withheld, key=lambda p: p[1]):
        r = rows[cid]
        lines.append(f"| `{cid}` | `{os.path.basename(path)}` | {r.get('status', '')} |")

if missing:
    lines += [
        "",
        "## Unreachable at fetch time",
        "",
        "CKM lists these and returned no export for them. They are recorded",
        "rather than silently skipped.",
        "",
        "| cid | intended file |",
        "|---|---|",
    ]
    for cid, dest in sorted(missing, key=lambda p: p[1]):
        lines.append(f"| `{cid}` | `{os.path.basename(dest)}` |")
lines.append("")
open(os.path.join(out, "PROVENANCE.md"), "w").write("\n".join(lines))
print(f"   provenance written for {len(present)} files")
PY
echo "== done"
