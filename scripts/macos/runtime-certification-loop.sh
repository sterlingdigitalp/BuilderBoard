#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CYCLES="${BUILDERBOARD_RUNTIME_CERTIFICATION_CYCLES:-20}"
INSTALL_DIR="${BUILDERBOARD_DEV_INSTALL_DIR:-/Applications}"
APP_NAME="${BUILDERBOARD_DEV_APP_NAME:-BuilderBoard Dev.app}"
INSTALL_APP="$INSTALL_DIR/$APP_NAME"
METRICS_DIR="${BUILDERBOARD_RUNTIME_METRICS_DIR:-$ROOT_DIR/target/runtime-certification}"
METRICS_FILE="$METRICS_DIR/runtime-certification-$(date +%Y%m%dT%H%M%S).csv"
REQUEST_COMMAND="${BUILDERBOARD_RUNTIME_REQUEST_COMMAND:-}"

now_ms() {
  perl -MTime::HiRes=time -e 'printf "%.0f\n", time() * 1000'
}

usage() {
  cat <<EOF
Usage: scripts/macos/runtime-certification-loop.sh

Runs repeated build/package/sign/install/launch cycles for the packaged local runtime.

Environment:
  BUILDERBOARD_RUNTIME_CERTIFICATION_CYCLES  Number of cycles, default 20
  BUILDERBOARD_RUNTIME_REQUEST_COMMAND       Optional authenticated request command
  BUILDERBOARD_RUNTIME_METRICS_DIR           Metrics directory
  BUILDERBOARD_DEV_INSTALL_DIR               Install directory
  BUILDERBOARD_DEV_APP_NAME                  App bundle name
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

mkdir -p "$METRICS_DIR"
echo "cycle,build_ms,launch_ms,request_ms,launch_status,request_status,keychain_prompts,notes" >"$METRICS_FILE"

for cycle in $(seq 1 "$CYCLES"); do
  echo "Runtime certification cycle $cycle/$CYCLES"

  started="$(now_ms)"
  build_output="$("$ROOT_DIR/scripts/macos/build-dev-runtime.sh" 2>&1)"
  build_status=$?
  ended="$(now_ms)"
  build_ms="$((ended - started))"

  if [[ "$build_status" -ne 0 ]]; then
    printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
      "$cycle" "$build_ms" 0 0 "FAILED" "SKIPPED" "UNKNOWN" "build failed" >>"$METRICS_FILE"
    echo "$build_output"
    exit "$build_status"
  fi

  "$ROOT_DIR/scripts/macos/assert-packaged-runtime.sh"

  launch_started="$(now_ms)"
  open -n "$INSTALL_APP"
  launch_status="OK"
  sleep 5
  launch_ms="$(($(now_ms) - launch_started))"

  request_status="SKIPPED"
  request_ms=0
  notes="set BUILDERBOARD_RUNTIME_REQUEST_COMMAND to execute authenticated request"

  if [[ -n "$REQUEST_COMMAND" ]]; then
    request_started="$(now_ms)"
    if bash -lc "$REQUEST_COMMAND"; then
      request_status="OK"
      notes="authenticated request command succeeded"
    else
      request_status="FAILED"
      notes="authenticated request command failed"
    fi
    request_ms="$(($(now_ms) - request_started))"
  fi

  osascript -e 'tell application "BuilderBoard" to quit' >/dev/null 2>&1 || true
  osascript -e 'tell application "BuilderBoard Dev" to quit' >/dev/null 2>&1 || true
  sleep 2

  printf '%s,%s,%s,%s,%s,%s,%s,%s\n' \
    "$cycle" "$build_ms" "$launch_ms" "$request_ms" "$launch_status" "$request_status" "MANUAL_OBSERVATION_REQUIRED" "$notes" >>"$METRICS_FILE"
done

echo "Runtime certification loop complete."
echo "Metrics: $METRICS_FILE"
