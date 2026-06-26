#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IDENTITY_NAME="${BUILDERBOARD_LOCAL_SIGNING_IDENTITY:-BuilderBoard Local Development}"
SOURCE_APP="$ROOT_DIR/target/release/bundle/macos/BuilderBoard.app"
INSTALL_DIR="${BUILDERBOARD_DEV_INSTALL_DIR:-/Applications}"
APP_NAME="${BUILDERBOARD_DEV_APP_NAME:-BuilderBoard Dev.app}"
INSTALL_APP="$INSTALL_DIR/$APP_NAME"
LAUNCH_AFTER_BUILD=0

usage() {
  cat <<EOF
Usage: scripts/macos/build-dev-runtime.sh [--launch]

Builds, packages, signs, and installs the local development runtime at:
  $INSTALL_APP

Environment:
  BUILDERBOARD_LOCAL_SIGNING_IDENTITY  Signing identity name
  BUILDERBOARD_DEV_INSTALL_DIR         Install directory, default /Applications
  BUILDERBOARD_DEV_APP_NAME            App bundle name, default BuilderBoard Dev.app
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --launch)
      LAUNCH_AFTER_BUILD=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The packaged local runtime workflow is only supported on macOS." >&2
  exit 1
fi

if ! security find-identity -v -p codesigning 2>/dev/null | grep -Fq "\"$IDENTITY_NAME\""; then
  echo "Missing local signing identity: $IDENTITY_NAME" >&2
  echo "Run: npm run runtime:setup" >&2
  exit 1
fi

metric_ms() {
  local started="$1"
  local ended
  ended="$(now_ms)"
  echo "$((ended - started))"
}

now_ms() {
  perl -MTime::HiRes=time -e 'printf "%.0f\n", time() * 1000'
}

BUILD_STARTED="$(now_ms)"
echo "Building BuilderBoard frontend and macOS app bundle..."
(
  cd "$ROOT_DIR"
  npm run tauri -- build --bundles app
)
BUILD_MS="$(metric_ms "$BUILD_STARTED")"

if [[ ! -d "$SOURCE_APP" ]]; then
  echo "Expected packaged app not found: $SOURCE_APP" >&2
  exit 1
fi

INSTALL_STARTED="$(now_ms)"
echo "Installing local runtime to: $INSTALL_APP"
mkdir -p "$INSTALL_DIR"
TEMP_APP="$INSTALL_DIR/.BuilderBoard Dev.app.tmp"
rm -rf "$TEMP_APP"
ditto "$SOURCE_APP" "$TEMP_APP"
rm -rf "$INSTALL_APP"
mv "$TEMP_APP" "$INSTALL_APP"
INSTALL_MS="$(metric_ms "$INSTALL_STARTED")"

SIGN_STARTED="$(now_ms)"
echo "Signing local runtime with: $IDENTITY_NAME"
codesign \
  --force \
  --deep \
  --timestamp=none \
  --sign "$IDENTITY_NAME" \
  "$INSTALL_APP"
SIGN_MS="$(metric_ms "$SIGN_STARTED")"

VERIFY_STARTED="$(now_ms)"
codesign --verify --deep --strict --verbose=2 "$INSTALL_APP"
VERIFY_MS="$(metric_ms "$VERIFY_STARTED")"

echo "Runtime identity:"
codesign -dv --verbose=2 "$INSTALL_APP" 2>&1 | sed -n '1,24p'
codesign -d -r- "$INSTALL_APP" 2>&1 | sed -n '1,8p'

echo "BuilderBoard local runtime ready."
echo "build_ms=$BUILD_MS"
echo "install_ms=$INSTALL_MS"
echo "sign_ms=$SIGN_MS"
echo "verify_ms=$VERIFY_MS"
echo "app=$INSTALL_APP"

if [[ "$LAUNCH_AFTER_BUILD" == "1" ]]; then
  "$ROOT_DIR/scripts/macos/launch-dev-runtime.sh"
fi
