#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
#
# The renderer's bundle budget. Every reader downloads this bundle, so its
# size is a product decision rather than a build detail, and a gate is the
# only thing that keeps a screen from adding 40 kB nobody notices.
#
# Two figures per asset, from `app/ferrochart-renderer/bundle-size.json`: a
# CEILING that does not move screen by screen, and a PER-CHANGE budget. The
# baseline a pull request is charged against is read out of the merge base,
# never out of the branch under judgement, so a change cannot pass itself by
# editing the recorded number.
#
# Usage:
#   scripts/checks/bundle-size.sh --base <sha>   on a pull request
#   scripts/checks/bundle-size.sh --verify-recorded   on main
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bars="app/ferrochart-renderer/bundle-size.json"
dist="$root/app/ferrochart-renderer/dist"
# A recorded figure and a fresh build differ by a few bytes for reasons that
# are not a regression, so main tolerates a small drift before it complains.
drift_tolerance=2000

mode="${1:---verify-recorded}"
base="${2:-}"

if [[ ! -d "$dist" ]]; then
  echo "bundle-size: SKIPPED, which is NOT a pass: $dist does not exist."
  echo "  Build it first: (cd app/ferrochart-renderer && trunk build --release --locked)"
  exit 0
fi

gzip_bytes() {
  gzip -9 -c "$1" | wc -c | tr -d ' '
}

# The one file matching a glob. Several would mean the build changed shape,
# and summing them would hide which one grew.
only_match() {
  local glob="$1" found count
  found=$(find "$dist" -maxdepth 1 -name "$glob" -type f | sort)
  count=$(printf '%s' "$found" | grep -c . || true)
  if [[ "$count" -ne 1 ]]; then
    echo "bundle-size: $count files match $glob in dist; the build changed shape." >&2
    exit 1
  fi
  printf '%s' "$found"
}

status=0
unbaselined=0
names=$(jq -r '.assets | keys[]' "$root/$bars")

for name in $names; do
  glob=$(jq -r ".assets[\"$name\"].glob" "$root/$bars")
  ceiling=$(jq -r ".assets[\"$name\"].max_gzip_bytes" "$root/$bars")
  budget=$(jq -r ".assets[\"$name\"].max_growth_gzip_bytes" "$root/$bars")
  recorded=$(jq -r ".assets[\"$name\"].measured_gzip_bytes" "$root/$bars")
  measured=$(gzip_bytes "$(only_match "$glob")")

  if [[ "$measured" -gt "$ceiling" ]]; then
    status=1
    echo "::error::$name is $measured gzipped bytes, over its $ceiling ceiling."
  fi

  case "$mode" in
    --base)
      [[ -n "$base" ]] || {
        echo "bundle-size: --base needs a commit." >&2
        exit 1
      }
      # The baseline comes out of the merge base, never out of the branch
      # under judgement. It is absent twice: on the change that adds this
      # file, and on the change that adds a new asset to it. Neither is a
      # regression and neither is a pass either, so both say so out loud and
      # the ceiling still applies.
      baseline=""
      if base_json=$(git -C "$root" show "$base:$bars" 2>/dev/null); then
        baseline=$(printf '%s' "$base_json" |
          jq -r ".assets[\"$name\"].measured_gzip_bytes // empty")
      fi
      if [[ -z "$baseline" ]]; then
        unbaselined=$((unbaselined + 1))
        printf '%-5s %7d gzipped, NO BASELINE in the merge base, which is NOT a pass: ' "$name" "$measured"
        printf 'the per-change budget did not apply (ceiling %d)\n' "$ceiling"
        printf '  Record %d as measured_gzip_bytes for %s once this lands.\n' "$measured" "$name"
        continue
      fi
      growth=$((measured - baseline))
      if [[ "$growth" -gt "$budget" ]]; then
        status=1
        echo "::error::$name grew $growth gzipped bytes over the merge base, past its $budget budget."
      fi
      printf '%-5s %7d gzipped, %+d against the merge base (budget %d, ceiling %d)\n' \
        "$name" "$measured" "$growth" "$budget" "$ceiling"
      ;;
    --verify-recorded)
      drift=$((measured - recorded))
      if [[ "${drift#-}" -gt "$drift_tolerance" ]]; then
        status=1
        echo "::error::$name measures $measured against a recorded $recorded. A change grew the bundle and did not record it, and the next change would be charged for both."
      fi
      printf '%-5s %7d gzipped, recorded %d (ceiling %d)\n' \
        "$name" "$measured" "$recorded" "$ceiling"
      ;;
    *)
      echo "bundle-size: unknown mode $mode" >&2
      exit 1
      ;;
  esac
done

[[ "$status" -eq 0 ]] || exit 1
if [[ "$unbaselined" -gt 0 ]]; then
  echo "bundle-size: every asset is inside its ceiling. $unbaselined had no baseline"
  echo "  in the merge base, so the per-change budget did not judge them."
else
  echo "bundle-size: every asset is inside its ceiling and its budget."
fi
