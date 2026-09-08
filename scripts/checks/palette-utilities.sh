#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
#
# Every screen styles against the semantic tokens of
# `app/ferrochart-renderer/style/tailwind.css`, never against a raw palette
# utility and never with a per-element `dark:` override, and no control draws
# its own focus ring. That rule is what
# keeps both themes in lockstep, and it is the rule a hurried screen breaks
# first: one `dark:bg-slate-900` compiles, looks right in the theme it was
# written in, and is wrong in the other one forever.
#
# Neither sibling enforces this. Both state it in prose and rely on care.
# This check is FerroCHART's own (issue #101), and it fails the build.
#
# Usage: scripts/checks/palette-utilities.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sources="$root/app/ferrochart-renderer/src"
stylesheet="$root/app/ferrochart-renderer/style/tailwind.css"

if [[ ! -d "$sources" ]]; then
  echo "palette-utilities: SKIPPED, which is NOT a pass: $sources does not exist."
  exit 0
fi

status=0
report() {
  status=1
  echo "::error file=$1,line=$2::$3"
  echo "  $1:$2: $3"
}

# A Tailwind colour utility naming a stock palette ramp. The token utilities
# this app does use (bg-surface, text-ink, border-edge) carry no numeric step,
# so the trailing number is what separates the two.
ramp='(bg|text|border|ring|outline|fill|stroke|decoration|divide|from|via|to|accent|caret|shadow)-(slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-[0-9]{2,3}'

# The scan stops at `#[cfg(test)]`. A test module asserts these rules and has
# to name what it forbids, so scanning it would fail the check on the very
# code that enforces it.
scan() {
  local pattern="$1" message="$2" file line
  while IFS= read -r file; do
    while IFS=: read -r line _; do
      [[ -z "$line" ]] && continue
      report "${file#"$root/"}" "$line" "$message"
    done < <(awk '/^#\[cfg\(test\)\]/ { exit } { print }' "$file" | grep -nE "$pattern" || true)
  done < <(find "$sources" -name '*.rs' -type f | sort)
}

scan "$ramp" "a raw palette utility; style against a semantic token"

# The focus indicator is ONE `:focus-visible` rule in the stylesheet's base
# layer, so no control draws its own and none can ship without one. What is
# refused is a class that draws a RING or an OUTLINE of its own, and
# `outline-none`, which removes the indicator and leaves nothing behind it.
#
# A `focus:` utility that changes something else is legitimate and stays
# legal. The skip link is the case that proves it: WCAG 2.2 success criterion
# 2.4.1 wants it revealed on focus, so it carries `focus:not-sr-only` and a
# position, and it draws no ring of its own.
scan '(focus|focus-visible):(ring|outline)' "a control drawing its own focus indicator; the stylesheet's base layer owns it"
scan '"[^"]*\boutline-none\b' "removing the focus indicator leaves a control with none"
scan '\bdark:' "a per-element dark: override; dark mode is the tokens"
scan '#[0-9a-fA-F]{6}\b' "a hex colour outside the stylesheet"

# The stylesheet has to keep defining what the utilities above resolve to.
for token in surface raised sunken edge edge-strong ink ink-muted ink-faint \
  accent accent-hover accent-subtle accent-ink on-accent ok warn danger scrim; do
  if ! grep -qE "^\s+--color-${token}: var\(--${token}\);" "$stylesheet"; then
    status=1
    echo "::error file=${stylesheet#"$root/"}::--color-${token} is not bridged into @theme inline"
    echo "  --color-${token} is not bridged into @theme inline"
  fi
done

if [[ "$status" -ne 0 ]]; then
  echo
  echo "palette-utilities: FAILED. The tokens are in ${stylesheet#"$root/"}."
  exit 1
fi

echo "palette-utilities: every screen styles against the semantic tokens."
