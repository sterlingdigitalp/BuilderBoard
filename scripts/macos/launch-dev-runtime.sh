#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${BUILDERBOARD_DEV_INSTALL_DIR:-/Applications}"
APP_NAME="${BUILDERBOARD_DEV_APP_NAME:-BuilderBoard Dev.app}"
INSTALL_APP="$INSTALL_DIR/$APP_NAME"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The local runtime app is only supported on macOS." >&2
  exit 1
fi

if [[ ! -d "$INSTALL_APP" ]]; then
  echo "Local runtime not found: $INSTALL_APP" >&2
  echo "Run: npm run runtime:build" >&2
  exit 1
fi

"$ROOT_DIR/scripts/macos/assert-packaged-runtime.sh"

echo "Launching $INSTALL_APP"
open -n "$INSTALL_APP"
