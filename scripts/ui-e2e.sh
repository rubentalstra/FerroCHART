#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# The renderer's browser journeys: a headless Chromium driving the bundle
# Trunk serves over the form surface `ferrochart` serves, through WebDriver
# (https://www.w3.org/TR/webdriver2/).
#
#   scripts/ui-e2e.sh
#   scripts/ui-e2e.sh --base-url URL --webdriver URL
#   scripts/ui-e2e.sh --docs-shots
#
# The battery must not change a tracked file. The documentation capture is the
# one exception and runs only when --docs-shots asks for it: it drives the same
# deployment and writes one PNG per screen per ground into
# website/book/src/operate/img/renderer, which the book embeds. An ordinary
# run, on a pull request or on a laptop, rewrites no image.
#
# Without arguments the script owns everything it drives. It stages the two
# operational templates below into a directory of their own, starts the server
# over them, builds the renderer bundle and serves it with Trunk, and runs a
# pinned Chromium in a container beside them. The browser reaches the renderer
# over the host gateway, which is why the renderer binds every interface here
# and the server stays on loopback: Trunk proxies /api to it from the host.
#
# Both ports come from app/ferrochart-renderer/Trunk.toml, so the battery
# drives the committed local-development configuration rather than a second
# one. A port already in use stops the run naming it, because a renderer that
# reached somebody else's server would fail on content nobody can explain.
#
# With --base-url and --webdriver it builds and starts nothing and drives what
# you already have. Both are required together, because a browser that cannot
# reach the address is a red lane with no defect behind it: a browser in a
# container reaches a server on the host through the host gateway, not on
# 127.0.0.1. A deployment driven that way has to serve the same two templates,
# and FERROCHART_UI_E2E_FORMS has to name them, or the journeys fail on a
# library that lists something else.
#
# The published container image carries the server binary and no bundle, and
# the server serves no /ui route, so there is no released artefact that answers
# the addresses below. That is issue #166, and it is why this script serves the
# renderer with Trunk rather than running compose.yaml.
#
# No CDR and no terminology server are started. Every screen the battery drives
# is a pure read of the form surface: the template library, the forms those
# templates compile to, and the design system. Nothing here commits, and a
# screen that did reach the CDR would fail loudly on an address nothing
# answers rather than be photographed. The round trip against a running CDR is
# scripts/test-cdr.sh.
#
# FERROCHART_UI_E2E_SHOTS_DIR sends the capture somewhere other than the book,
# for looking at an image without touching the checkout.
#
# A journey that fails writes what it was looking at into
# target/ui-e2e-failures: a screenshot and the whole document, one pair per
# failed wait. A passing run leaves the directory empty, and the ui-e2e CI job
# uploads it as an artifact only when the run failed.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# The browser, pinned by index digest so every run drives the same Chromium
# and the same chromedriver. Selenium publishes the pair in one image and keeps
# them in step; the tag records which pair this digest is. docs/VERSIONS.md
# carries the row and scripts/checks/versions.sh keeps the two in agreement.
readonly BROWSER_IMAGE="selenium/standalone-chromium:4.48.0-20260905@sha256:fcf9eef47b9546a2252937481a8298ce0958d20c9d91e040d480184e80b41c76"

# The name the browser addresses the renderer by. It resolves to the host
# gateway, so the container reaches Trunk running on the host.
readonly RENDERER_HOST="ferrochart"

# The operational templates the battery drives, by file stem under
# corpus/templates/ckm. This list is the single declaration: the server is
# given exactly these, the template library is asserted to list exactly these,
# and each one's form is a journey and an image. Both are committed openEHR CKM
# exports and carry no patient content.
readonly TEMPLATES=(
  family-history-summary-item-r2
  alcohol-consumption-summary-item-r2
)

# Where the staged templates and the failure evidence go. Neither is tracked.
readonly STAGED="$root/target/ui-e2e-templates"
readonly FAILURES_DIR="$root/target/ui-e2e-failures"

# Seconds to wait for the server and the browser to answer. Both are already
# built or already pulled by the time they are probed.
readonly READY_TIMEOUT=120

# Seconds to wait for the renderer to answer. `trunk serve` runs a build
# before it serves anything, and even over a warm cargo cache that build still
# runs wasm-bindgen and wasm-opt over a bundle of about a megabyte, so this one
# is a build budget rather than a startup budget.
readonly RENDERER_TIMEOUT=900

# How many journeys drive the browser at once. The Selenium image allows one
# session per container by default, so a second journey would sit in the grid's
# new-session queue until the first quit; the image's own README raises the
# ceiling with SE_NODE_MAX_SESSIONS plus SE_NODE_OVERRIDE_MAX_SESSIONS and says
# not to exceed the available processors
# (https://github.com/SeleniumHQ/docker-selenium). Four is the ubuntu-latest
# runner's processor count, and the same number is the test-thread count below,
# so the journeys never ask for a session the browser cannot open.
readonly BROWSER_SESSIONS=4

base_url=""
webdriver=""
docs_shots=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) base_url=$2; shift 2 ;;
    --webdriver) webdriver=$2; shift 2 ;;
    --docs-shots) docs_shots=1; shift ;;
    *) echo "ui-e2e: unknown argument: $1" >&2; exit 2 ;;
  esac
done

if { [[ -n "$base_url" ]] && [[ -z "$webdriver" ]]; } ||
   { [[ -z "$base_url" ]] && [[ -n "$webdriver" ]]; }; then
  echo "ui-e2e: --base-url and --webdriver are given together or not at all" >&2
  exit 2
fi

need() {
  command -v "$1" >/dev/null 2>&1 ||
    { echo "ui-e2e: $1 is not installed; $2" >&2; exit 1; }
}

need cargo "install the toolchain in rust-toolchain.toml"
need cargo-nextest "cargo install cargo-nextest --locked"
need curl "the readiness probes ask each service whether it answers"

# The identifier one operational template states for itself, which is what
# addresses its form. It is the value of the FIRST template_id element: a later
# one belongs to an archetype the template composes, not to the template.
template_id_of() {
  awk '
    /<template_id>/ { inside = 1; next }
    inside && match($0, /<value>[^<]*<\/value>/) {
      line = substr($0, RSTART + 7, RLENGTH - 15)
      print line
      exit
    }
  ' "$1"
}

# The renderer's committed build and serve configuration, and the copy this
# run drives.
#
# The battery cannot use the committed file as it stands, because its ports are
# the fixed local-development ones and a laptop or a runner may already be
# using either. It cannot move them on the command line either: Trunk documents
# a command-line proxy as replacing the ones in the config file
# (https://trunkrs.dev/advanced/proxy/), and 0.21.14 registers both and then
# panics on the duplicate route. So the run writes a copy with this run's ports
# in it, beside the original so every relative path still resolves, and removes
# it on the way out. Removing the copy is issue #168.
readonly TRUNK_TOML=app/ferrochart-renderer/Trunk.toml
readonly TRUNK_RUN=app/ferrochart-renderer/Trunk.ui-e2e.toml

# Whether anything is listening on a local port, so a published port never
# lands on a socket another process already holds.
port_taken() {
  (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null || return 1
  exec 3<&-
  return 0
}

# The first free port at or above $1.
free_port() {
  local candidate
  for candidate in $(seq "$1" "$(($1 + 40))"); do
    if ! port_taken "$candidate"; then
      echo "$candidate"
      return 0
    fi
  done
  echo "ui-e2e: no free port in $1..$(($1 + 40))" >&2
  return 1
}

# Waits until $1 answers, or says what it did instead. $2 names it, $3 is a
# process id to watch, or empty for a container watched by name in $4, and $5
# is how many seconds to give it.
wait_for_http() {
  local probe=$1 what=$2 pid=$3 container=$4 budget=$5 waited=0
  while [[ "$waited" -lt "$((budget * 5))" ]]; do
    if [[ -n "$pid" ]] && ! kill -0 "$pid" 2>/dev/null; then
      echo "ui-e2e: $what exited before it was ready" >&2
      return 1
    fi
    if [[ -n "$container" ]] &&
       [[ "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null)" != true ]]; then
      echo "ui-e2e: $what exited before it was ready" >&2
      docker logs "$container" >&2 || true
      return 1
    fi
    if curl -sf "$probe" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
    waited=$((waited + 1))
  done
  echo "ui-e2e: $what did not answer $probe within ${budget}s" >&2
  [[ -z "$container" ]] || docker logs "$container" >&2 || true
  return 1
}

server_pid=""
renderer_pid=""
browser=""
cleanup() {
  [[ -z "$browser" ]] || docker rm -f "$browser" >/dev/null 2>&1 || true
  [[ -z "$renderer_pid" ]] || kill "$renderer_pid" 2>/dev/null || true
  [[ -z "$server_pid" ]] || kill "$server_pid" 2>/dev/null || true
  rm -f "$TRUNK_RUN"
}

# The forms the journeys drive, as one `stem<TAB>identifier` line each. The
# tests read this rather than naming a template of their own, so the staged
# directory and the addresses they open cannot disagree.
forms=""
echo "== the operational templates, staged into target/ui-e2e-templates"
rm -rf "$STAGED"
mkdir -p "$STAGED"
for stem in "${TEMPLATES[@]}"; do
  source_file="corpus/templates/ckm/$stem.opt"
  [[ -f "$source_file" ]] || { echo "ui-e2e: no $source_file" >&2; exit 1; }
  cp "$source_file" "$STAGED/$stem.opt"
  identifier="$(template_id_of "$source_file")"
  [[ -n "$identifier" ]] ||
    { echo "ui-e2e: $source_file states no template identifier" >&2; exit 1; }
  forms+="$stem"$'\t'"$identifier"$'\n'
  echo "   $stem -> $identifier"
done

if [[ -z "$base_url" ]]; then
  need docker "the managed mode runs the browser as a container"
  need trunk "the pin is in docs/VERSIONS.md; the bundle is what the journeys drive"
  docker info >/dev/null 2>&1 ||
    { echo "ui-e2e: docker is installed but not running" >&2; exit 1; }

  trap cleanup EXIT

  api_port="$(free_port 8140)"
  ui_port="$(free_port 8180)"
  # The committed configuration with this run's ports in it. Rewriting rather
  # than writing one from scratch keeps the build settings, the Tailwind pin
  # and the proxy set in the one file a local session reads, so a change there
  # reaches this lane.
  sed -E \
    -e "s|^backend = \"http://127\.0\.0\.1:[0-9]+/|backend = \"http://127.0.0.1:$api_port/|" \
    -e "s|^port = [0-9]+$|port = $ui_port|" \
    "$TRUNK_TOML" > "$TRUNK_RUN"
  grep -q "127.0.0.1:$api_port/api" "$TRUNK_RUN" ||
    { echo "ui-e2e: $TRUNK_TOML proxies no /api backend on loopback" >&2; exit 1; }
  grep -qx "port = $ui_port" "$TRUNK_RUN" ||
    { echo "ui-e2e: $TRUNK_TOML states no [serve] port" >&2; exit 1; }

  echo "== the form surface on 127.0.0.1:$api_port"
  cargo build --release --locked -p ferrochart
  # The two endpoints are required at startup and neither is reached by any
  # screen the battery drives, so they name a loopback port nothing listens on:
  # a screen that did call one fails on a refused connection rather than
  # rendering something that looks answered.
  FERROCHART_LISTEN="127.0.0.1:$api_port" \
    FERROCHART_CDR_URL="http://127.0.0.1:1/openehr" \
    FERROCHART_TERM_URL="http://127.0.0.1:1/r4" \
    FERROCHART_TEMPLATES="$STAGED" \
    RUST_LOG="${RUST_LOG:-info}" \
    target/release/ferrochart &
  server_pid=$!
  wait_for_http "http://127.0.0.1:$api_port/health" "the form surface" "$server_pid" "" "$READY_TIMEOUT"

  echo "== the renderer bundle, served on 0.0.0.0:$ui_port"
  # Every interface, because the browser is in a container and reaches the
  # host by its gateway address. Everything else, the serve port and the proxy
  # to the form surface included, comes from the rewritten configuration.
  # --no-autoreload drops the websocket a watching dev server injects, which
  # nothing here reloads for.
  (
    cd app/ferrochart-renderer
    # `exec` so the recorded process id is Trunk's own and cleanup reaches it
    # rather than a subshell that has already gone.
    # The `design` feature is on by default, and one of the screens below is
    # the style guide it draws. A release bundle drops it (#154).
    exec trunk serve --config "$(basename "$TRUNK_RUN")" \
      --release --locked --address 0.0.0.0 --no-autoreload
  ) &
  renderer_pid=$!
  wait_for_http "http://127.0.0.1:$ui_port/ui/" "the renderer" "$renderer_pid" "" "$RENDERER_TIMEOUT"
  # A bundle served without its backend would draw every screen as a refusal,
  # and the journeys would then fail on a missing control rather than on the
  # proxy that is actually broken.
  curl -sf "http://127.0.0.1:$ui_port/api/templates" >/dev/null 2>&1 || {
    echo "ui-e2e: the renderer does not proxy /api, so no screen can read anything" >&2
    exit 1
  }

  # Chromium needs more than the default 64 MB /dev/shm; without this it
  # crashes on a page of any size
  # (https://developer.chrome.com/docs/chromium/headless).
  webdriver_port="$(free_port 4444)"
  browser="ferrochart-ui-e2e-$$"
  echo "== the browser on 127.0.0.1:$webdriver_port"
  docker run --detach --name "$browser" --shm-size 2g \
    --add-host "$RENDERER_HOST:host-gateway" \
    --env "SE_NODE_MAX_SESSIONS=$BROWSER_SESSIONS" \
    --env SE_NODE_OVERRIDE_MAX_SESSIONS=true \
    --publish "127.0.0.1:$webdriver_port:4444" "$BROWSER_IMAGE" >/dev/null
  wait_for_http "http://127.0.0.1:$webdriver_port/status" "the browser" "" "$browser" "$READY_TIMEOUT"

  base_url="http://$RENDERER_HOST:$ui_port"
  webdriver="http://127.0.0.1:$webdriver_port"
fi

# A previous run's evidence would be uploaded beside this run's, so the
# directory starts empty and only a failing journey writes into it.
rm -rf "$FAILURES_DIR"
mkdir -p "$FAILURES_DIR"

echo "== the journeys, against $base_url through $webdriver"
# The journeys live outside the workspace, for the reason e2e/Cargo.toml
# records, so they are run by manifest path rather than by package.
#
# The capture pass is excluded by a nextest set difference, so it never runs
# beside the journeys and never writes an image nobody asked for
# (https://nexte.st/docs/filtersets/).
FERROCHART_UI_E2E_BASE_URL="$base_url" \
  FERROCHART_UI_E2E_WEBDRIVER="$webdriver" \
  FERROCHART_UI_E2E_FAILURES="$FAILURES_DIR" \
  FERROCHART_UI_E2E_FORMS="$forms" \
  cargo nextest run --manifest-path e2e/Cargo.toml --locked \
    --test-threads "$BROWSER_SESSIONS" \
    -E 'binary(it) - test(/^docs_shots::/)'

# The documentation capture, which is the one thing here that writes into the
# checkout. It runs after the journeys, so an image is only ever taken of a
# renderer the journeys have just found working.
if [[ -n "$docs_shots" ]]; then
  echo "== the documentation screenshots, into website/book/src/operate/img/renderer"
  FERROCHART_UI_E2E_BASE_URL="$base_url" \
    FERROCHART_UI_E2E_WEBDRIVER="$webdriver" \
    FERROCHART_UI_E2E_FAILURES="$FAILURES_DIR" \
    FERROCHART_UI_E2E_FORMS="$forms" \
    FERROCHART_UI_E2E_DOCS_SHOTS=1 \
    cargo nextest run --manifest-path e2e/Cargo.toml --locked --no-capture \
      -E 'test(/^docs_shots::/)'
  bash scripts/checks/docs-shots.sh
fi
