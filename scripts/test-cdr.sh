#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# test-cdr.sh: run the ITS-REST client's wire tests against a real CDR.
#
#   scripts/test-cdr.sh            # start the compose demo CDR, test, stop
#   scripts/test-cdr.sh --keep     # leave it running afterwards
#
# The mock tests prove the client sends the right request and reads the right
# response. They prove nothing about whether a real CDR agrees, which is what
# these cases are for, and why they are #[ignore]d in a normal run: an ignored
# case reports as ignored rather than as a pass, so a run without a CDR cannot
# be mistaken for one with it.
#
# Point it at a CDR you already have instead of starting one:
#
#   FERROCHART_TEST_CDR_URL=https://cdr.example/openehr \
#     FERROCHART_TEST_CDR_AUTHORIZATION='Bearer …' scripts/test-cdr.sh
#
# The CDR it starts is the demo profile of the release's own compose.yaml, so
# what is tested against is the artefact a downloader runs.
set -euo pipefail
cd "$(dirname "$0")/.."

readonly SERVICE=ferroehr
readonly PORT="${FERROEHR_PORT:-8081}"
# Development credentials, and they are not a secret: the demo profile's own
# header says so, and this is the single Basic user it serves.
readonly DEMO_USER=ferroehr
readonly DEMO_PASSWORD=ferroehr

keep=0
if [[ "${1:-}" == "--keep" ]]; then
  keep=1
elif [[ $# -gt 0 ]]; then
  echo "usage: $0 [--keep]" >&2
  exit 2
fi

started=0

stop() {
  if [[ "$started" -eq 1 && "$keep" -eq 0 ]]; then
    echo "test-cdr: stopping the demo CDR"
    compose down >/dev/null 2>&1 || true
  fi
}
trap stop EXIT

compose() {
  # The demo profile's own variables are required by compose.yaml even for a
  # single service, so they are supplied here rather than left to the shell.
  FERROCHART_CDR_URL="http://${SERVICE}:8080/ferroehr/rest/openehr" \
    FERROCHART_TERM_URL="http://ferroterm:8080/r4" \
    docker compose --profile demo "$@"
}

if [[ -z "${FERROCHART_TEST_CDR_URL:-}" ]]; then
  if ! docker info >/dev/null 2>&1; then
    echo "test-cdr: docker is not running, and FERROCHART_TEST_CDR_URL is unset" >&2
    echo "test-cdr: start docker, or point FERROCHART_TEST_CDR_URL at a CDR" >&2
    exit 1
  fi

  echo "test-cdr: starting the demo CDR on port ${PORT}"
  compose up -d "$SERVICE"
  started=1

  credentials="$(printf '%s:%s' "$DEMO_USER" "$DEMO_PASSWORD" | base64)"
  export FERROCHART_TEST_CDR_URL="http://127.0.0.1:${PORT}/ferroehr/rest/openehr"
  export FERROCHART_TEST_CDR_AUTHORIZATION="Basic ${credentials}"

  echo "test-cdr: waiting for ${FERROCHART_TEST_CDR_URL}"
  for _ in $(seq 1 60); do
    status="$(curl -fsS -o /dev/null -w '%{http_code}' \
      -H "Authorization: ${FERROCHART_TEST_CDR_AUTHORIZATION}" \
      "${FERROCHART_TEST_CDR_URL}/v1/definition/template/adl1.4" 2>/dev/null || true)"
    # Any answered status means the CDR is serving; the tests judge the rest.
    if [[ -n "$status" && "$status" != "000" ]]; then
      echo "test-cdr: the CDR answered ${status}"
      break
    fi
    sleep 2
  done
  if [[ -z "${status:-}" || "$status" == "000" ]]; then
    echo "test-cdr: the CDR never answered" >&2
    compose logs --tail 50 "$SERVICE" >&2 || true
    exit 1
  fi
fi

echo "test-cdr: running the wire tests against ${FERROCHART_TEST_CDR_URL}"
# One thread: the cases share one CDR and one uploaded template.
#
# ferrochart-server carries the gate of docs/architecture.md section 9, whose
# live cases commit a COMPOSITION FerroCHART built and validated. A CDR
# refusing one of those is a FerroCHART defect, so they run here beside the
# client's own wire tests.
cargo nextest run --locked -p ferrochart-cdr -p ferrochart-server \
  --run-ignored all --test-threads 1
