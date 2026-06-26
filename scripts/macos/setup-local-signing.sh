#!/usr/bin/env bash
set -euo pipefail

IDENTITY_NAME="${BUILDERBOARD_LOCAL_SIGNING_IDENTITY:-BuilderBoard Local Development}"
KEYCHAIN_PATH="${BUILDERBOARD_KEYCHAIN_PATH:-$HOME/Library/Keychains/login.keychain-db}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/builderboard-signing.XXXXXX")"
FORCE=0

usage() {
  cat <<EOF
Usage: scripts/macos/setup-local-signing.sh [--force]

Creates a free local self-signed code-signing identity for BuilderBoard.

Environment:
  BUILDERBOARD_LOCAL_SIGNING_IDENTITY  Signing identity name
  BUILDERBOARD_KEYCHAIN_PATH           Keychain path, default login keychain
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force)
      FORCE=1
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

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Local signing setup is only required on macOS."
  exit 0
fi

if [[ "$FORCE" == "1" ]]; then
  echo "Removing existing local signing certificate, if present: $IDENTITY_NAME"
  security delete-certificate -c "$IDENTITY_NAME" "$KEYCHAIN_PATH" >/dev/null 2>&1 || true
fi

verify_identity_trust() {
  local test_binary="$WORK_DIR/sign-test-existing"
  cp /bin/echo "$test_binary"
  codesign --force --timestamp=none --sign "$IDENTITY_NAME" "$test_binary" >/dev/null
  codesign --verify --strict --verbose=2 "$test_binary" >/dev/null 2>&1
}

if [[ "$FORCE" != "1" ]] && security find-identity -v -p codesigning "$KEYCHAIN_PATH" 2>/dev/null | grep -Fq "\"$IDENTITY_NAME\""; then
  if verify_identity_trust; then
    echo "Found trusted local BuilderBoard signing identity: $IDENTITY_NAME"
    exit 0
  fi

  echo "Found local signing identity, but strict macOS verification does not trust it."
  echo "Repairing local certificate trust for code signing."
  EXISTING_CERT_PATH="$WORK_DIR/builderboard-existing-local-dev.crt"
  security find-certificate -c "$IDENTITY_NAME" -p "$KEYCHAIN_PATH" >"$EXISTING_CERT_PATH"
  security add-trusted-cert \
    -r trustRoot \
    -p codeSign \
    -k "$KEYCHAIN_PATH" \
    "$EXISTING_CERT_PATH"

  if verify_identity_trust; then
    echo "Local BuilderBoard signing identity trust repaired: $IDENTITY_NAME"
    exit 0
  fi

  echo "Unable to repair trust for '$IDENTITY_NAME'." >&2
  echo "Run: npm run runtime:setup -- --force" >&2
  exit 1
fi

OPENSSL_CONFIG="$WORK_DIR/openssl.cnf"
CERT_PATH="$WORK_DIR/builderboard-local-dev.crt"
KEY_PATH="$WORK_DIR/builderboard-local-dev.key"
P12_PATH="$WORK_DIR/builderboard-local-dev.p12"
P12_PASSWORD="$(openssl rand -hex 24)"

cat >"$OPENSSL_CONFIG" <<EOF
[ req ]
default_bits = 2048
prompt = no
default_md = sha256
x509_extensions = v3_codesign
distinguished_name = dn

[ dn ]
CN = $IDENTITY_NAME
O = BuilderBoard Local Development
OU = Runtime Certification

[ v3_codesign ]
basicConstraints = critical, CA:true
keyUsage = critical, digitalSignature, keyCertSign, cRLSign
extendedKeyUsage = codeSigning
subjectKeyIdentifier = hash
EOF

echo "Creating local self-signed code-signing certificate: $IDENTITY_NAME"
openssl req \
  -new \
  -newkey rsa:2048 \
  -nodes \
  -x509 \
  -days 3650 \
  -keyout "$KEY_PATH" \
  -out "$CERT_PATH" \
  -config "$OPENSSL_CONFIG" >/dev/null 2>&1

openssl pkcs12 \
  -legacy \
  -export \
  -name "$IDENTITY_NAME" \
  -inkey "$KEY_PATH" \
  -in "$CERT_PATH" \
  -out "$P12_PATH" \
  -passout "pass:$P12_PASSWORD" >/dev/null 2>&1

echo "Importing certificate and private key into: $KEYCHAIN_PATH"
security import "$P12_PATH" \
  -k "$KEYCHAIN_PATH" \
  -P "$P12_PASSWORD" \
  -T /usr/bin/codesign \
  -T /usr/bin/security

echo "Trusting certificate for local code signing."
security add-trusted-cert \
  -r trustRoot \
  -p codeSign \
  -k "$KEYCHAIN_PATH" \
  "$CERT_PATH"

echo "Verifying local code-signing identity."
TEST_BINARY="$WORK_DIR/sign-test"
cp /bin/echo "$TEST_BINARY"
codesign --force --timestamp=none --sign "$IDENTITY_NAME" "$TEST_BINARY" >/dev/null
if ! codesign --verify --strict --verbose=2 "$TEST_BINARY" >/dev/null 2>&1; then
  echo "Local signing identity can sign, but strict macOS verification does not trust it." >&2
  echo "Open Keychain Access and set '$IDENTITY_NAME' to Always Trust for Code Signing, then rerun this command." >&2
  exit 1
fi

if ! security find-identity -v -p codesigning "$KEYCHAIN_PATH" | grep -Fq "\"$IDENTITY_NAME\""; then
  echo "Failed to create a valid local code-signing identity named '$IDENTITY_NAME'." >&2
  echo "Open Keychain Access and confirm the certificate is trusted for Code Signing." >&2
  exit 1
fi

echo "Local BuilderBoard signing identity is ready: $IDENTITY_NAME"
echo "macOS may ask for your account password during this one-time setup."
