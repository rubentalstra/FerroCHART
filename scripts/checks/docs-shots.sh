#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# The book's screenshots: every one is real, named, and used.
#
#   scripts/checks/docs-shots.sh
#
# A screenshot of a blank page passes a gate and lies to a reader, so the
# images the capture pass writes are checked before they are committed:
#
#   1. every file in the shots directory is a PNG by its magic bytes;
#   2. every one is bigger than a screen with nothing drawn on it, which is
#      what an empty page compresses to;
#   3. every file name is one the capture pass produces, so a stray image
#      cannot arrive beside them and no name can carry anything but a screen
#      and a committed template stem;
#   4. every image is embedded by a book page, and every book reference into
#      the directory resolves to a file that is there.
#
# The images themselves carry no entered content: the capture types nothing
# into a form, and the forms are committed, synthetic openEHR CKM exports
# (corpus/templates/ckm/PROVENANCE.md).
#
# Exit 0 = every image is real and used. Exit 1 = a finding.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

readonly SHOTS=website/book/src/operate/img/renderer
readonly PAGES=website/book/src

# The smallest a real screenshot is, in bytes. A 1440-pixel-wide capture of a
# screen with nothing on it compresses into a few kilobytes; every screen the
# battery drives carries text, controls and rules and lands far above this.
readonly MIN_BYTES=20000

# The names the capture pass writes: one per screen, optionally on the dark
# ground. A form's name carries the stem of the template file it was compiled
# from, which is a committed CKM export.
readonly NAME_SHAPE='^(templates|design|form-[a-z0-9]+(-[a-z0-9]+)*)(-dark)?\.png$'

fail=0
note() { printf '  %s\n' "$*"; }
bad() {
  printf '  FINDING: %s\n' "$*" >&2
  fail=1
}

if [[ ! -d "$SHOTS" ]]; then
  echo "docs-shots: no $SHOTS yet, skipped"
  exit 0
fi

echo "== the images in $SHOTS"
shopt -s nullglob
images=("$SHOTS"/*)
shopt -u nullglob
if [[ "${#images[@]}" -eq 0 ]]; then
  bad "$SHOTS holds no image, so every page that embeds one is broken"
fi

for image in "${images[@]}"; do
  name="$(basename "$image")"
  if [[ ! -f "$image" ]]; then
    bad "$image is not a file"
    continue
  fi
  # The PNG signature, the first eight bytes of every PNG
  # (https://www.w3.org/TR/png-3/#5PNG-file-signature).
  signature="$(head -c 8 "$image" | od -An -tx1 | tr -d ' \n')"
  if [[ "$signature" != "89504e470d0a1a0a" ]]; then
    bad "$name is not a PNG (it begins $signature)"
  fi
  bytes="$(wc -c <"$image" | tr -d ' ')"
  if [[ "$bytes" -lt "$MIN_BYTES" ]]; then
    bad "$name is $bytes bytes, under the $MIN_BYTES a drawn screen takes; it may be blank"
  fi
  if ! printf '%s' "$name" | grep -Eq "$NAME_SHAPE"; then
    bad "$name is not a name the capture pass writes"
  fi
  if ! grep -Rql -- "img/renderer/$name" "$PAGES"; then
    bad "$name is committed and no book page embeds it"
  fi
done

echo "== the references from $PAGES"
while IFS= read -r wanted; do
  [[ -n "$wanted" ]] || continue
  if [[ ! -f "$SHOTS/$wanted" ]]; then
    bad "a book page embeds img/renderer/$wanted and no such image exists"
  fi
done < <(grep -Rho -- 'img/renderer/[A-Za-z0-9._-]*' "$PAGES" |
  sed 's|img/renderer/||' | sort -u)

echo
if [[ "$fail" -ne 0 ]]; then
  echo "docs-shots: findings above" >&2
  exit 1
fi
note "OK: ${#images[@]} images, every one real and embedded"
