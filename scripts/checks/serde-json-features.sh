#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# serde_json feature guard (docs/architecture.md sections 4, 6 and 11).
#
# Cargo features are additive and unify across a build graph, so a feature any
# dependency turns on, at any depth, is turned on for every crate in the
# workspace. serde_json carries features that change how a number and a
# buffered value are handled, and FerroCHART publishes two documents that go
# through serde_json:
#
#   the form definition   the contract a third party writes a renderer
#                         against.
#   the layout overlay    stored on disk and replayed against a later
#                         recompile, so its bytes are stored user work.
#
# The hazard is not theoretical. Under `arbitrary_precision` a number reaches
# serde's content buffer as a map rather than as a number, so an internally
# tagged enum carrying an f64 stops deserializing, and adopting one dependency
# took the form definition off the wire with no change to this tree at all.
# Neither `cargo deny` nor scripts/checks/versions.sh has an opinion about what
# a pin pulls in, which is the gap this fills.
#
# The check reads the resolved feature set of serde_json from `cargo metadata`,
# with and without --all-features, and compares it against the set recorded
# below. A REFUSED feature fails. So does one this file has never seen: a
# dependency turning on something new is the event worth stopping on, and
# reviewing it means deciding what it does to the two documents above and
# recording that decision here.
#
# Usage:
#   scripts/checks/serde-json-features.sh
#
# Exit 0 = the resolved set is the reviewed one. Exit 1 = a refused or an
# unreviewed feature is enabled. Exit 2 = usage, or a missing tool.

set -euo pipefail

cd "$(dirname "$0")/../.."

if [ "$#" -ne 0 ]; then
  echo "usage: scripts/checks/serde-json-features.sh" >&2
  exit 2
fi

for tool in cargo jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "serde-json-features: $tool is not installed" >&2
    exit 2
  fi
done

if [ ! -f Cargo.toml ]; then
  echo "serde-json-features: SKIP (no root Cargo.toml yet)"
  exit 0
fi

# A feature that changes what the two published documents do on the wire. The
# reason is printed on failure, so a person reading the CI log learns what
# broke rather than only which name is banned.
refused=(
  "arbitrary_precision|a number reaches serde's content buffer as a map, so an internally tagged enum carrying an f64 no longer deserializes, and the form definition holds several"
)

# A feature that has been read and does nothing to the two documents. Every
# entry carries the reason it is harmless here.
permitted=(
  "default|the crate's own default set, which is std alone"
  "std|the standard library, which every build here has"
  "float_roundtrip|parses a float to the nearest representable value, so a magnitude reads back as it was written"
  "indexmap|the map preserve_order stores a serde_json::Value object in"
  "preserve_order|orders the members of a serde_json::Value object by insertion; every map of the two documents is a BTreeMap serialized by derive, which never goes through Value"
  "raw_value|adds serde_json::value::RawValue, which nothing is required to use"
  "unbounded_depth|lifts the recursion limit behind an opt-in reader, and neither document's reader asks for it"
)

# The resolved features of serde_json, as cargo works them out for the whole
# workspace. Both resolves are read because the Rust lanes run with and without
# --all-features, and a feature enabled in only one of them is still enabled.
resolved_features() {
  cargo metadata --locked --format-version 1 "$@" | jq -r '
    (reduce .packages[] as $p ({}; .[$p.id] = $p.name)) as $name_of
    | .resolve.nodes[]
    | select($name_of[.id] == "serde_json")
    | .features[]
  '
}

features="$(
  {
    resolved_features
    resolved_features --all-features
  } | sort -u
)"

if [ -z "$features" ]; then
  echo "serde-json-features: SKIP (serde_json is not in the resolved graph)"
  exit 0
fi

# What turned a feature on, which is the only part a person can act on.
enabler_tree() {
  echo "  the feature graph that turns it on:" >&2
  cargo tree --locked --workspace --edges features --invert serde_json >&2 || true
}

failed=0

while IFS= read -r feature; do
  [ -n "$feature" ] || continue

  reason=""
  for entry in "${refused[@]}"; do
    if [ "${entry%%|*}" = "$feature" ]; then
      reason="${entry#*|}"
      break
    fi
  done
  if [ -n "$reason" ]; then
    failed=1
    echo "  FAIL: serde_json/$feature is enabled" >&2
    echo "    $reason" >&2
    continue
  fi

  known=0
  for entry in "${permitted[@]}"; do
    if [ "${entry%%|*}" = "$feature" ]; then
      known=1
      echo "  OK: serde_json/$feature (${entry#*|})"
      break
    fi
  done
  if [ "$known" -eq 0 ]; then
    failed=1
    echo "  FAIL: serde_json/$feature is enabled and has never been reviewed" >&2
    echo "    decide what it does to the form definition and the overlay, then" >&2
    echo "    record it in scripts/checks/serde-json-features.sh" >&2
  fi
done <<<"$features"

if [ "$failed" -ne 0 ]; then
  enabler_tree
  echo "serde-json-features: a dependency changed the serde_json feature set" >&2
  exit 1
fi

echo "serde-json-features: OK (the resolved feature set is the reviewed one)"
