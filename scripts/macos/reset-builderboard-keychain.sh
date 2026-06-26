#!/usr/bin/env bash
set -euo pipefail

SERVICE="${BUILDERBOARD_KEYCHAIN_SERVICE:-com.builderboard.app}"
KEYCHAIN_PATH="${BUILDERBOARD_KEYCHAIN_PATH:-$HOME/Library/Keychains/login.keychain-db}"
YES=0
DRY_RUN=0

usage() {
  cat <<EOF
Usage: scripts/macos/reset-builderboard-keychain.sh [--yes] [--dry-run]

Removes BuilderBoard credential entries from macOS Keychain service:
  $SERVICE

This does not weaken security and does not export secrets. It removes stale
credential entries so they can be recreated once from the stable packaged app:
  /Applications/BuilderBoard Dev.app

Options:
  --yes      Delete without interactive confirmation
  --dry-run  List matching entries only
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --yes)
      YES=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
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
  echo "BuilderBoard Keychain reset is only supported on macOS." >&2
  exit 1
fi

echo "BuilderBoard Keychain entries for service: $SERVICE"
security dump-keychain -a "$KEYCHAIN_PATH" 2>/dev/null \
  | awk -v service="$SERVICE" '
      /^keychain:/ {
        if (svce == service) {
          print "  account=" acct
        }
        acct = ""
        svce = ""
      }
      /"acct"<blob>=/ {
        acct = $0
        sub(/^.*"acct"<blob>=/, "", acct)
      }
      /"svce"<blob>=/ {
        svce = $0
        sub(/^.*"svce"<blob>=/, "", svce)
        gsub(/^"|"$/, "", svce)
      }
      END {
        if (svce == service) {
          print "  account=" acct
        }
      }
    ' || true

if [[ "$DRY_RUN" == "1" ]]; then
  exit 0
fi

if [[ "$YES" != "1" ]]; then
  cat <<EOF

This will delete all BuilderBoard credentials stored under:
  $SERVICE

After deletion, launch the packaged app and reconnect accounts once:
  npm run runtime:launch

Continue? Type DELETE to proceed:
EOF
  read -r confirmation
  if [[ "$confirmation" != "DELETE" ]]; then
    echo "Cancelled."
    exit 1
  fi
fi

deleted=0
while security find-generic-password -s "$SERVICE" "$KEYCHAIN_PATH" >/dev/null 2>&1; do
  security delete-generic-password -s "$SERVICE" "$KEYCHAIN_PATH" >/dev/null
  deleted=$((deleted + 1))
done

echo "Deleted $deleted BuilderBoard Keychain entr$(if [[ "$deleted" == "1" ]]; then echo "y"; else echo "ies"; fi)."
echo "Launch the packaged runtime and recreate credentials under the stable app identity:"
echo "  npm run runtime:launch"
