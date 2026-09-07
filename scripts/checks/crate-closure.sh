#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# First-party dependency-closure guard (docs/architecture.md sections 10, 11).
#
# Five crates carry a promise about what they link, and a promise no tool
# checks is a wish:
#
#   ferrochart-form       the published contract a third party writes a
#                         renderer against. It links no other crate of this
#                         tree, which is what lets a renderer take it alone.
#   ferrochart-renderer   the browser binary. It links ferrochart-form and no
#                         other crate of this tree, so the UI cannot reach an
#                         engine crate.
#   ferrochart-cdr        the ITS-REST client. It links ferrochart-form for the
#                         identifiers on the wire and nothing else, so it stays
#                         a client rather than half an engine.
#   ferrochart-compose    the composition builder. It links ferrochart-form and
#                         nothing else, so it works from the form definition
#                         rather than from the template.
#   ferrochart-term       the terminology client. It links ferrochart-compile
#                         for the template terminology it resolves value sets
#                         out of, and ferrochart-form, and nothing else: it
#                         never reaches the CDR client, the overlay store or
#                         the server surface.
#
# The check reads the resolved graph from `cargo metadata` and walks the
# normal and build closure of each package, so it catches a first-party crate
# arriving transitively as well as directly. Dev-dependencies are excluded:
# they build the test binaries and ship in nothing.
#
# Usage:
#   scripts/checks/crate-closure.sh
#
# Exit 0 = every promise holds. Exit 1 = a first-party crate that should not
# be in a closure is in it. Exit 2 = usage, or a missing tool.

set -euo pipefail

cd "$(dirname "$0")/../.."

if [ "$#" -ne 0 ]; then
  echo "usage: scripts/checks/crate-closure.sh" >&2
  exit 2
fi

for tool in cargo jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "crate-closure: $tool is not installed" >&2
    exit 2
  fi
done

if [ ! -f Cargo.toml ]; then
  echo "crate-closure: SKIP (no root Cargo.toml yet)"
  exit 0
fi

# One row per promise: the package, then every first-party crate its closure
# may contain. An empty second field means none at all.
promises=(
  "ferrochart-form|"
  "ferrochart-renderer|ferrochart-form"
  "ferrochart-cdr|ferrochart-form"
  "ferrochart-compose|ferrochart-form"
  "ferrochart-term|ferrochart-compile ferrochart-form"
)

metadata="$(cargo metadata --locked --format-version 1 --all-features)"

# Every workspace member, by name. A crate is first-party exactly when it is a
# member, so a new crate joins this check by joining the workspace.
members="$(jq -r '
  [.workspace_members[] as $id | .packages[] | select(.id == $id) | .name] | unique | .[]
' <<<"$metadata")"

# The transitive normal and build closure of one package, reported as the
# first-party crate names in it other than the package itself.
first_party_closure() {
  jq -r --arg root "$1" --arg members "$members" '
    def grow($deps; $frontier; $seen):
      ($frontier - $seen) as $new
      | if ($new | length) == 0
        then $seen
        else grow($deps; ($new | map($deps[.] // []) | add // []); ($seen + $new))
        end;

    ($members | split("\n") | map(select(length > 0))) as $first_party
    | (reduce .packages[] as $p ({}; .[$p.id] = $p.name)) as $name_of
    | (reduce .resolve.nodes[] as $n ({};
        .[$n.id] = [
          $n.deps[]
          | select([.dep_kinds[].kind] | any(. == null or . == "build"))
          | .pkg
        ]
      )) as $deps
    | ($name_of | to_entries | map(select(.value == $root) | .key)) as $roots
    | grow($deps; $roots; [])
    | map($name_of[.])
    | map(select(IN($first_party[])))
    | map(select(. != $root))
    | unique
    | sort
    | .[]
  ' <<<"$metadata"
}

failed=0

for promise in "${promises[@]}"; do
  package="${promise%%|*}"
  permitted="${promise#*|}"

  if ! grep -qxF "$package" <<<"$members"; then
    echo "  SKIP: $package is not a workspace member yet"
    continue
  fi

  linked=()
  unexpected=()
  while IFS= read -r crate; do
    [ -n "$crate" ] || continue
    linked+=("$crate")
    found=0
    for allowed in $permitted; do
      if [ "$crate" = "$allowed" ]; then
        found=1
        break
      fi
    done
    if [ "$found" -eq 0 ]; then
      unexpected+=("$crate")
    fi
  done < <(first_party_closure "$package")

  if [ "${#unexpected[@]}" -eq 0 ]; then
    if [ "${#linked[@]}" -eq 0 ]; then
      echo "  OK: $package links no other crate of this tree"
    else
      echo "  OK: $package links ${linked[*]} and nothing else of this tree"
    fi
  else
    failed=1
    echo "  FAIL: $package links a first-party crate it promised not to:" >&2
    printf '    %s\n' "${unexpected[@]}" >&2
    if [ -z "$permitted" ]; then
      echo "    permitted: none (docs/architecture.md section 11)" >&2
    else
      echo "    permitted: $permitted (docs/architecture.md sections 10, 11)" >&2
    fi
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "crate-closure: a promised boundary was crossed" >&2
  exit 1
fi

echo "crate-closure: OK (every promised boundary holds)"
