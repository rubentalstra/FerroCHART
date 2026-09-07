#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# Table-of-contents guard for the book (issue #98).
#
# Every book build prints:
#
#   The mdbook-toc preprocessor was built against version 0.5.0 of mdbook,
#   but we're being called from version 0.5.4
#
# That warning is not the signal it looks like. mdbook-toc 0.15.4 declares
# `mdbook-preprocessor ^0.5.0`, that crate's own README states it follows
# semver for its APIs, and `^0.5.0` admits 0.5.4, so the two are compatible by
# construction. mdBook compares the exact version string rather than the
# semver range, so the warning fires between every compatible pair and would
# fail a build forever if it were treated as an error.
#
# What can actually break is the output: a preprocessor that stops running,
# or runs and produces nothing, leaves the marker in the page or drops the
# list. mdBook reports neither as an error, so the docs lane would publish a
# chapter with a missing table of contents and a green build.
#
# This checks the output instead of the warning. For every source chapter
# carrying a `<!-- toc -->` marker, the rendered page must have consumed the
# marker and must carry at least one in-page anchor.
#
# Usage:
#   scripts/checks/book-toc.sh [BOOK_DIR]
#
# BOOK_DIR defaults to website/book. The book must already be built; the docs
# lane runs this after `scripts/site/assemble.sh`.
#
# Exit 0 = every chapter that asks for a table of contents has one.
# Exit 1 = one does not. Exit 2 = usage, or the book is not built.

set -euo pipefail

cd "$(dirname "$0")/../.."

if [ "$#" -gt 1 ]; then
  echo "usage: scripts/checks/book-toc.sh [BOOK_DIR]" >&2
  exit 2
fi

readonly BOOK="${1:-website/book}"
readonly SRC="$BOOK/src"
readonly OUT="$BOOK/book"

if [ ! -d "$OUT" ]; then
  echo "book-toc: $OUT does not exist; build the book first" >&2
  exit 2
fi

failed=0
checked=0

while IFS= read -r chapter; do
  page="$OUT/${chapter#"$SRC"/}"
  page="${page%.md}.html"

  if [ ! -f "$page" ]; then
    echo "  FAIL: $chapter asks for a table of contents and rendered no $page" >&2
    failed=1
    continue
  fi

  checked=$((checked + 1))

  # The preprocessor consumes the marker. One left in the output means it
  # never ran, which mdBook reports as a warning at most.
  if grep -qF '<!-- toc -->' "$page"; then
    echo "  FAIL: $page still carries the toc marker, so the preprocessor did not run" >&2
    failed=1
    continue
  fi

  # A table of contents is a list of links to headings on the same page.
  if ! grep -qE '<a href="#[a-z0-9-]+">' "$page"; then
    echo "  FAIL: $page has no in-page links, so its table of contents is empty" >&2
    failed=1
    continue
  fi

  echo "  OK: ${chapter#"$SRC"/}"
done < <(grep -rlF '<!-- toc -->' "$SRC" --include='*.md' | sort)

if [ "$checked" -eq 0 ] && [ "$failed" -eq 0 ]; then
  # No chapter asks for one, so the preprocessor is unused and the pin should
  # go. Saying nothing here would let the pin outlive its only reader.
  echo "book-toc: no chapter carries a toc marker; the mdbook-toc pin has no reader" >&2
  exit 1
fi

if [ "$failed" -ne 0 ]; then
  echo "book-toc: a chapter that asks for a table of contents has none" >&2
  exit 1
fi

echo "book-toc: OK ($checked chapters)."
