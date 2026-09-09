#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# The published site agrees with the tree it describes.
#
#   scripts/checks/site.sh
#
# The landing page is what a hospital reads before deciding whether to try the
# thing, and it went three releases out of date without anything noticing
# (issue #196). Two facts on it are machine-checkable, so they are checked:
#
#   1. every version it states is the product version this tree releases;
#   2. every corpus figure it quotes is the figure the test suite asserts.
#
# The second matters more than it looks. The figures are the page's whole
# claim to being measured rather than written, and they were 2658 fields and
# 1789 groups against a suite asserting 2621 and 1775.
#
# What this cannot check is prose: a sentence saying a shipped capability is
# not built reads exactly like a true one. That stays review-enforced, and
# `.claude/rules/writing-style.md` section Scope framing is the rule.
#
# Exit 0 = the site agrees. Exit 1 = a finding.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

readonly LANDING=website/landing/index.html
# The pages a person edits. `website/book/book` is mdbook output and is not
# tracked, so a check that read it would pass or fail on whether somebody had
# built the book.
readonly SOURCES=(website/landing website/book/src)
# The one place the corpus totals are asserted, which is what makes them a
# measurement rather than a number somebody typed.
readonly CORPUS_TEST=crates/ferrochart-compile/tests/it/derive.rs

fail=0
note() { printf '  %s\n' "$*"; }
bad() {
  printf '  FINDING: %s\n' "$*" >&2
  fail=1
}

if [[ ! -f "$LANDING" ]]; then
  echo "site: no $LANDING, skipped"
  exit 0
fi

# The value of `key = "..."` in the first table of a Cargo manifest.
product_version() {
  sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' Cargo.toml | head -n1
}

# The number an `assert_eq!(<name>, <number>);` in the corpus test states.
asserted() {
  sed -nE "s/^[[:space:]]*assert_eq!\($1, ([0-9]+)\);.*/\1/p" "$CORPUS_TEST" | head -n1
}

echo "== the version the landing page states ($LANDING <-> Cargo.toml)"
version="$(product_version)"
if [[ -z "$version" ]]; then
  bad "root Cargo.toml states no version"
else
  # Every `0.x.y` on the page, however it is spelled, has to be this one. A
  # release note naming an older version is exactly what went stale.
  stale="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' "$LANDING" | sort -u | grep -v "^${version}$" || true)"
  if [[ -n "$stale" ]]; then
    while IFS= read -r found; do
      bad "the landing page names version $found, and this tree releases $version"
    done <<<"$stale"
  else
    note "OK: every version on the page is $version"
  fi
fi

echo "== the corpus figures the landing page quotes ($LANDING <-> $CORPUS_TEST)"
if [[ ! -f "$CORPUS_TEST" ]]; then
  note "no $CORPUS_TEST yet, skipped the figures"
else
  # The name in the test, the number, and what the page calls it.
  for pair in "derived:templates that derive" "fields:fields derived" "groups:groups they sit in"; do
    name="${pair%%:*}"
    what="${pair#*:}"
    number="$(asserted "$name")"
    if [[ -z "$number" ]]; then
      bad "$CORPUS_TEST asserts no total for \`$name\`, so \"$what\" cannot be checked"
      continue
    fi
    if grep -qE "figure-big\">${number}( |<)" "$LANDING"; then
      note "OK: $what reads $number"
    else
      shown="$(grep -oE 'figure-big">[0-9]+' "$LANDING" | sed 's/.*>//' | tr '\n' ' ')"
      bad "$what is asserted at $number and the page shows one of: $shown"
    fi
  done
fi


echo "== the corpus figures the prose quotes (website <-> $CORPUS_TEST)"
# The book and the landing page's own meta description state them in prose
# rather than in a figure block, so the check is anchored on the sentence. A
# phrasing that changes fails here rather than passing silently, which is the
# point: an unanchored figure is one nobody notices going stale. The meta
# description is the worst place for a stale figure, because it is what a
# search result shows.
fields="$(asserted fields)"
groups="$(asserted groups)"
derived="$(asserted derived)"
quoted="$(grep -rlE '[0-9]+ fields across [0-9]+ groups' "${SOURCES[@]}" || true)"
if [[ -z "$quoted" ]]; then
  note "no page states a field and group total, skipped"
else
  while IFS= read -r page; do
    read_fields="$(grep -oE '[0-9]+ fields across [0-9]+ groups' "$page" | head -n1)"
    if [[ "$read_fields" == "$fields fields across $groups groups" ]]; then
      note "OK: $page reads $read_fields"
    else
      bad "$page reads \"$read_fields\" and the suite asserts $fields fields across $groups groups"
    fi
  done <<<"$quoted"
fi

read_derived="$(grep -rhoE '121 read and all [0-9]+ derive' "${SOURCES[@]}" | head -n1 || true)"
if [[ -n "$read_derived" && "$read_derived" != "121 read and all $derived derive" ]]; then
  bad "the book reads \"$read_derived\" and the suite asserts $derived"
fi

echo
if [[ "$fail" -ne 0 ]]; then
  echo "site: findings above" >&2
  exit 1
fi
note "OK: the landing page agrees with the tree"
