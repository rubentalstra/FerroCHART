#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# CHANGELOG.md keeps the shape Keep a Changelog 1.1.0 gives it.
#
#   scripts/checks/changelog.sh
#
# Every pull request adds an entry under [Unreleased], and the easy way to do
# that is to paste a new `### Changed` above the old one. Do it three times and
# one release has three Changed sections, which is how a reader stops being
# able to find anything. That is the defect this refuses.
#
# What it checks, per release section:
#
#   1. a change-type heading appears at most once;
#   2. every heading is one Keep a Changelog names, spelled its way;
#   3. the headings run in the order that specification lists them;
#   4. a heading has at least one entry under it.
#
# It also checks that [Unreleased] is present and first, because a release is
# cut by renaming it.
#
# Exit 0 = the shape holds. Exit 1 = a finding.
#
# https://keepachangelog.com/en/1.1.0/
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

readonly FILE=CHANGELOG.md

# The six Keep a Changelog names, in the order the specification lists them.
readonly ORDER=(Added Changed Deprecated Removed Fixed Security)

fail=0
note() { printf '  %s\n' "$*"; }
bad() {
  printf '  FINDING: %s\n' "$*" >&2
  fail=1
}

if [[ ! -f "$FILE" ]]; then
  echo "changelog: no $FILE, skipped"
  exit 0
fi

# The rank of a heading in the Keep a Changelog order, or empty when it names
# nothing that specification defines.
rank_of() {
  local wanted="$1" index=0
  for name in "${ORDER[@]}"; do
    if [[ "$name" == "$wanted" ]]; then
      printf '%s' "$index"
      return 0
    fi
    index=$((index + 1))
  done
  return 1
}

echo "== the sections of $FILE"

section=""
seen=""
last_rank=-1
heading=""
entries=0
sections=0
first_section=""

# Closes the heading being read, so an empty one is reported against the
# section it sat in rather than against the next.
close_heading() {
  if [[ -n "$heading" && "$entries" -eq 0 ]]; then
    bad "[$section] has a '### $heading' with no entry under it"
  fi
  heading=""
  entries=0
}

while IFS= read -r line; do
  case "$line" in
  '## ['*)
    close_heading
    section="${line#\#\# \[}"
    section="${section%%]*}"
    sections=$((sections + 1))
    if [[ -z "$first_section" ]]; then
      first_section="$section"
    fi
    seen=""
    last_rank=-1
    ;;
  '### '*)
    close_heading
    heading="${line#\#\#\# }"
    if [[ -z "$section" ]]; then
      bad "a '### $heading' sits above every release section"
      continue
    fi
    if ! this_rank="$(rank_of "$heading")"; then
      bad "[$section] has '### $heading', which Keep a Changelog does not name"
      continue
    fi
    case " $seen " in
    *" $heading "*)
      bad "[$section] has more than one '### $heading'"
      ;;
    *)
      seen="$seen $heading"
      ;;
    esac
    if [[ "$this_rank" -lt "$last_rank" ]]; then
      bad "[$section] puts '### $heading' after a heading Keep a Changelog lists later"
    fi
    last_rank="$this_rank"
    ;;
  '- '*)
    if [[ -n "$heading" ]]; then
      entries=$((entries + 1))
    fi
    ;;
  esac
done <"$FILE"
close_heading

if [[ "$sections" -eq 0 ]]; then
  bad "$FILE names no release section"
fi
if [[ "$first_section" != "Unreleased" ]]; then
  bad "the first section is [$first_section] and a changelog opens with [Unreleased]"
fi

echo
if [[ "$fail" -ne 0 ]]; then
  echo "changelog: findings above" >&2
  exit 1
fi
note "OK: $sections sections, each with one heading per change type, in order"
