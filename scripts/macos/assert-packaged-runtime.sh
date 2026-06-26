#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALL_DIR="${BUILDERBOARD_DEV_INSTALL_DIR:-/Applications}"
APP_NAME="${BUILDERBOARD_DEV_APP_NAME:-BuilderBoard Dev.app}"
INSTALL_APP="$INSTALL_DIR/$APP_NAME"
DEBUG_APP="$ROOT_DIR/target/debug/builderboard"
EXPECTED_IDENTIFIER="${BUILDERBOARD_BUNDLE_IDENTIFIER:-com.builderboard.app}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  exit 0
fi

if [[ "${BUILDERBOARD_ALLOW_DEBUG_RUNTIME:-0}" == "1" ]]; then
  exit 0
fi

if ! process_table="$(ps ax -o pid= -o command= 2>/dev/null)"; then
  cat >&2 <<EOF
Unable to inspect running processes.

Authenticated runtime testing must prove that no debug BuilderBoard runtime is
running. Close any `npm run dev` / `cargo tauri dev` sessions and retry from a
normal terminal.
EOF
  exit 1
fi

debug_processes="$(
  printf "%s\n" "$process_table" \
    | grep -F "$DEBUG_APP" \
    | grep -v grep \
    || true
)"

tauri_dev_processes="$(
  printf "%s\n" "$process_table" \
    | grep -F "$ROOT_DIR/node_modules/.bin/tauri dev" \
    | grep -v grep \
    || true
)"

if [[ -n "$debug_processes" || -n "$tauri_dev_processes" ]]; then
  cat >&2 <<EOF
BuilderBoard debug runtime is currently running.

Authenticated runtime testing must use only:
  $INSTALL_APP

The debug executable has an unstable Keychain identity:
  $DEBUG_APP

Stop these processes before launching the packaged runtime:
$debug_processes
$tauri_dev_processes

For unauthenticated UI-only development, set:
  BUILDERBOARD_ALLOW_DEBUG_RUNTIME=1
EOF
  exit 1
fi

if [[ ! -d "$INSTALL_APP" ]]; then
  cat >&2 <<EOF
Packaged BuilderBoard runtime not found:
  $INSTALL_APP

Run:
  npm run runtime:build
EOF
  exit 1
fi

designated_requirement="$(codesign -d -r- "$INSTALL_APP" 2>&1 || true)"
signature_details="$(codesign -dv --verbose=4 "$INSTALL_APP" 2>&1 || true)"

if ! printf "%s\n" "$designated_requirement" | grep -Fq "identifier \"$EXPECTED_IDENTIFIER\""; then
  cat >&2 <<EOF
Packaged BuilderBoard runtime does not present the expected bundle identifier:
  $INSTALL_APP

Expected identifier:
  $EXPECTED_IDENTIFIER

Actual designated requirement:
$designated_requirement

Run:
  npm run runtime:setup
  npm run runtime:build
EOF
  exit 1
fi

if ! printf "%s\n" "$designated_requirement" | grep -Fq "certificate root = H"; then
  cat >&2 <<EOF
Packaged BuilderBoard runtime is not signed with a stable certificate identity:
  $INSTALL_APP

Actual designated requirement:
$designated_requirement

Run:
  npm run runtime:setup
  npm run runtime:build
EOF
  exit 1
fi

if printf "%s\n" "$signature_details" | grep -Fq "Signature=adhoc"; then
  cat >&2 <<EOF
Packaged BuilderBoard runtime is ad-hoc signed and cannot be used for authenticated testing:
  $INSTALL_APP

Run:
  npm run runtime:setup
  npm run runtime:build
EOF
  exit 1
fi
