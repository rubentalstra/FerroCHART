#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# assemble.sh: the site as GitHub Pages serves it, from the landing page, the
# brand assets, and the book, into one directory.
#
#   scripts/site/assemble.sh OUT
#
# The landing page is served at the site root and the book under /docs/, which
# website/book/book.toml declares with `site-url = "/docs/"` so the book's
# asset, search, and 404 links resolve under the prefix.
#
# The mark and the favicons are copied from assets/brand/, which is the one
# authority for them. Nothing under website/landing/ holds a second copy, so a
# change to the mark is made once and every surface picks it up.
#
# Serve the result to check it, because `mdbook serve` alone cannot: the
# book's absolute /docs/ asset paths only resolve when the book sits under
# that prefix.
#
# `.claude/rules/rust-style.md` bans Python across this repository, so the
# instruction names a server that ships with the tooling already pinned here.
#
#   scripts/site/assemble.sh _site && miniserve _site
#
# Any static server rooted at _site does; the point is that /docs/ resolves.
set -euo pipefail
cd "$(dirname "$0")/../.."

readonly OUT="${1:?usage: $0 OUT}"
readonly LANDING=website/landing
readonly BRAND=assets/brand

# The brand files the site references, named one by one so a missing file
# fails the assembly instead of publishing a page with a broken mark. The
# palette source and the brand README stay out of the published tree.
readonly BRAND_FILES=(
  apple-touch-icon.png
  favicon-16.png
  favicon-32.png
  favicon.ico
  favicon.svg
  ferrochart-icon.svg
  ferrochart-social.png
  tokens.css
)

# mdBook takes its tab icon from theme/favicon.svg and theme/favicon.png and
# offers no path setting, so the two are staged from assets/brand/ here rather
# than committed as a second copy of the mark. website/book/theme/ is ignored
# by git for that reason, and `mdbook build` on its own falls back to mdBook's
# default icon.
mkdir -p website/book/theme
cp "$BRAND/favicon.svg" website/book/theme/favicon.svg
cp "$BRAND/favicon-32.png" website/book/theme/favicon.png

mdbook build website/book

rm -rf "$OUT"
mkdir -p "$OUT/docs" "$OUT/$BRAND"
cp -R "$LANDING"/. "$OUT/"
for f in "${BRAND_FILES[@]}"; do
  cp "$BRAND/$f" "$OUT/$BRAND/$f"
done
cp -R website/book/book/. "$OUT/docs/"

# A client that finds no icon link asks for /favicon.ico at the site root, and
# the book under /docs/ links only its own theme icons, so the .ico is served
# from both places out of the one file in assets/brand/.
cp "$BRAND/favicon.ico" "$OUT/favicon.ico"

# GitHub Pages serves /404.html for a miss anywhere on the site, so the book's
# 404 page is promoted to the root. Its asset paths are absolute under /docs/
# because of site-url, which is what lets one file work at both depths.
if [[ -f "$OUT/docs/404.html" ]]; then
  cp "$OUT/docs/404.html" "$OUT/404.html"
fi

echo "assemble: site at $OUT (landing at /, the book at /docs/)"
