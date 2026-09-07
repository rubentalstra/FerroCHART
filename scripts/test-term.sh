#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# test-term.sh: run the terminology client's wire tests against a real server.
#
#   scripts/test-term.sh              # HL7's public R4 terminology service
#   scripts/test-term.sh --ferroterm  # the compose demo profile, then stop it
#   scripts/test-term.sh --ferroterm --keep
#
# The mock tests prove the client sends the right request and reads the right
# response. They prove nothing about whether a real server agrees, which is
# what these cases are for, and why they are #[ignore]d in a normal run: an
# ignored case reports as ignored rather than as a pass, so a run without a
# server cannot be mistaken for one with it.
#
# The default target is HL7's own public R4 service, so the acceptance
# criterion "works against any conformant FHIR terminology server, tested
# against at least one that is not FerroTERM" is met by the default run. The
# cases are read-only and ask only for value sets FHIR R4 4.0.1 publishes
# itself.
#
# Point it at a server you already have instead:
#
#   FERROCHART_TEST_TERM_URL=https://tx.example/r4 \
#     FERROCHART_TEST_TERM_AUTHORIZATION='Bearer …' scripts/test-term.sh
set -euo pipefail
cd "$(dirname "$0")/.."

readonly SERVICE=ferroterm
readonly PORT="${FERROTERM_PORT:-8082}"
# HL7's public R4 terminology service, the reference conformant deployment
# that is not FerroTERM.
readonly PUBLIC_R4=https://tx.fhir.org/r4

ferroterm=0
keep=0
for argument in "$@"; do
  case "$argument" in
    --ferroterm) ferroterm=1 ;;
    --keep) keep=1 ;;
    *)
      echo "usage: $0 [--ferroterm] [--keep]" >&2
      exit 2
      ;;
  esac
done

started=0

stop() {
  if [[ "$started" -eq 1 && "$keep" -eq 0 ]]; then
    echo "test-term: stopping the demo terminology server"
    compose down >/dev/null 2>&1 || true
  fi
}
trap stop EXIT

compose() {
  # The demo profile's own variables are required by compose.yaml even for a
  # single service, so they are supplied here rather than left to the shell.
  FERROCHART_CDR_URL="http://ferroehr:8080/ferroehr/rest/openehr" \
    FERROCHART_TERM_URL="http://${SERVICE}:8080/r4" \
    docker compose --profile demo "$@"
}

if [[ -z "${FERROCHART_TEST_TERM_URL:-}" && "$ferroterm" -eq 1 ]]; then
  if ! docker info >/dev/null 2>&1; then
    echo "test-term: docker is not running, and FERROCHART_TEST_TERM_URL is unset" >&2
    exit 1
  fi
  echo "test-term: starting the demo terminology server on port ${PORT}"
  compose up -d "$SERVICE"
  started=1
  export FERROCHART_TEST_TERM_URL="http://127.0.0.1:${PORT}/r4"
fi

if [[ -z "${FERROCHART_TEST_TERM_URL:-}" ]]; then
  export FERROCHART_TEST_TERM_URL="$PUBLIC_R4"
fi

echo "test-term: waiting for ${FERROCHART_TEST_TERM_URL}"
status=""
for _ in $(seq 1 30); do
  status="$(curl -fsS -m 30 -o /dev/null -w '%{http_code}' \
    -H 'Accept: application/fhir+json' \
    "${FERROCHART_TEST_TERM_URL}/metadata" 2>/dev/null || true)"
  # Any answered status means the server is serving; the tests judge the rest.
  if [[ -n "$status" && "$status" != "000" ]]; then
    echo "test-term: the server answered ${status}"
    break
  fi
  sleep 2
done
if [[ -z "$status" || "$status" == "000" ]]; then
  echo "test-term: the server never answered" >&2
  if [[ "$started" -eq 1 ]]; then
    compose logs --tail 50 "$SERVICE" >&2 || true
  fi
  exit 1
fi

echo "test-term: running the wire tests against ${FERROCHART_TEST_TERM_URL}"
# One thread: a public terminology service is a shared resource, and the cases
# are small enough that serial is fast.
cargo nextest run --locked -p ferrochart-term --run-ignored all --test-threads 1
