# Local Development Runtime

BuilderBoard runtime certification uses a packaged, locally signed macOS app.

Before beginning runtime testing, review the [Engineering Evidence Library](AUDITS/README.md)
for existing investigations and findings relevant to your work.

Do not use `cargo tauri dev` or `npm run dev` for authenticated runtime certification. Those commands run a debug executable whose ad-hoc identity changes after rebuilds, which can cause repeated macOS Keychain prompts.

`npm run dev` remains available for unauthenticated UI-only work. It is not a valid authenticated runtime.

## Goal

```text
Clone repository
Install dependencies
Build BuilderBoard
Launch BuilderBoard
Authenticate once if macOS asks
Work normally
Rebuild
Launch again
Continue working without repeated Keychain prompts
```

This workflow is free. It does not require a paid Apple Developer account and does not move secrets outside macOS Keychain.

## One-Time Setup

Install dependencies:

```sh
npm install
```

Create the local code-signing identity:

```sh
npm run runtime:setup
```

This creates a self-signed local certificate named:

```text
BuilderBoard Local Development
```

The certificate and private key are stored in the user login Keychain. macOS may ask for the account password during this one-time setup. That prompt is expected.

Verify the identity exists:

```sh
security find-identity -v -p codesigning
```

You should see `BuilderBoard Local Development`.

If the certificate was created before this workflow existed, recreate it:

```sh
npm run runtime:setup -- --force
```

## Build And Install The Runtime

Build, package, sign, verify, and install the local runtime:

```sh
npm run runtime:build
```

The app is installed at:

```text
/Applications/BuilderBoard Dev.app
```

Launch it:

```sh
npm run runtime:launch
```

Or build and launch in one step:

```sh
npm run runtime:build -- --launch
```

## Normal Runtime Workflow

Use this workflow for all authenticated runtime work:

```text
npm run runtime:build
npm run runtime:launch
```

After the first Keychain authorization, rebuilding and reinstalling the app should preserve Keychain trust because the app is signed with the same local signing identity each time.

The launch and certification scripts refuse to run if the debug executable is still active:

```text
/Users/sterlingdigital/BuilderBoard/target/debug/builderboard
```

If this happens, quit `npm run dev` / `cargo tauri dev`, then launch the packaged runtime again.

## One-Time Keychain ACL Migration

If BuilderBoard previously stored credentials while running from `cargo tauri dev`, `npm run dev`, or `target/debug/builderboard`, macOS may have authorized those credentials for stale debug executable hashes instead of the stable packaged app.

Symptoms:

- macOS asks for Keychain access repeatedly after rebuilds.
- Keychain ACLs mention `target/debug/builderboard`.
- Authenticated runtime testing works once, then prompts again.

Reset only BuilderBoard's local Keychain credential entries:

```sh
npm run runtime:keychain:reset -- --dry-run
npm run runtime:keychain:reset -- --yes
```

Then recreate credentials from the packaged app only:

```sh
npm run runtime:build -- --launch
```

Reconnect accounts once in `/Applications/BuilderBoard Dev.app`.

This does not weaken security and does not move secrets outside Keychain. It deletes stale BuilderBoard credentials so macOS can recreate them under the stable packaged app identity.

## Keychain Behavior

The first launch of the packaged runtime (`/Applications/BuilderBoard Dev.app`) may prompt for Keychain access once. This is expected — the packaged app needs to register its identity with the macOS Keychain.

Subsequent launches should **not** prompt for Keychain access. If the Keychain prompt appears on every launch, this indicates a runtime regression. The issue must be recorded in the Runtime Engineering Ledger as a new entry.

### Troubleshooting

If Keychain prompts persist:

1. Verify the packaged runtime is being used (`/Applications/BuilderBoard Dev.app`), not `cargo tauri dev`.
2. Run `scripts/macos/reconnect-keychain.sh` to reset stale credentials.
3. If prompts continue after reset, file a ledger entry.

## Development vs Certification Runtime

| Mode | Command | Purpose | Keychain Stability |
|------|---------|---------|-------------------|
| **Development** | `npm run dev` | UI development, component testing, HMR | Unstable — may lose credentials across restarts |
| **Certification** | `npm run runtime:build -- --launch` | Authenticated Olympic event execution | Stable — packaged identity persists |

Builder T and Builder V must use the packaged runtime for all Olympic event execution.

## Builder T Workflow

Builder T must use the packaged local runtime:

```sh
npm run runtime:build
npm run runtime:launch
```

Builder T must not use:

```sh
npm run dev
cargo tauri dev
```

for runtime certification.

Before authenticated certification, Builder T must confirm no debug runtime is active:

```sh
scripts/macos/assert-packaged-runtime.sh
```

## Builder V Workflow

Builder V uses the same runtime as Builder T:

```text
/Applications/BuilderBoard Dev.app
```

There is one runtime source of truth for certification.

Builder V must reject evidence collected from `target/debug/builderboard` or `cargo tauri dev` for authenticated provider work.

## Runtime Certification Loop

Run the packaged runtime loop:

```sh
npm run runtime:certify
```

By default this performs 20 build/package/sign/install/launch cycles and writes metrics to:

```text
target/runtime-certification/
```

This proves that rebuilds preserve the packaged runtime identity. To certify authenticated provider work, provide a command that sends a Builder request and exits successfully only when the request succeeds:

```sh
BUILDERBOARD_RUNTIME_REQUEST_COMMAND='your-request-command' npm run runtime:certify
```

The metrics CSV records:

- cycle
- build time
- launch time
- request time
- launch status
- request status
- Keychain prompt observation field
- notes

A certification run is fully autonomous only when `BUILDERBOARD_RUNTIME_REQUEST_COMMAND` is configured for the local test environment. Without it, request status is recorded as `SKIPPED`.

## Why This Is Required

macOS Keychain authorizes access based on application identity. A debug Tauri executable is ad-hoc signed and identified by its code hash. Rebuilding changes that hash, so macOS can treat the rebuilt app as a new program.

The packaged local runtime uses a stable app bundle and a stable local signing identity.

## Troubleshooting

### `Missing local signing identity`

Run:

```sh
npm run runtime:setup
```

### macOS asks for a password during setup

Expected. The script is creating and trusting a local code-signing identity.

### macOS asks for Keychain access on first provider request

Expected once. Choose the option that allows BuilderBoard to access the credential going forward.

### macOS asks again after every rebuild

Check the installed app identity:

```sh
codesign -dv --verbose=4 "/Applications/BuilderBoard Dev.app" 2>&1
codesign -d -r- "/Applications/BuilderBoard Dev.app" 2>&1
```

Confirm it is signed by `BuilderBoard Local Development`, not ad-hoc.

Strict verification should pass:

```sh
codesign --verify --deep --strict --verbose=4 "/Applications/BuilderBoard Dev.app"
```

If verification reports `CSSMERR_TP_NOT_TRUSTED`, recreate the local identity:

```sh
npm run runtime:setup -- --force
npm run runtime:build
```

If the app verifies but prompts continue, stale credentials are probably ACL-bound to an old debug executable hash. Reset BuilderBoard credentials and recreate them from the packaged app:

```sh
npm run runtime:keychain:reset -- --dry-run
npm run runtime:keychain:reset -- --yes
npm run runtime:build -- --launch
```

Do not run `npm run dev` while performing authenticated validation.

### `/Applications` cannot be written

Use a user-writable stable directory:

```sh
BUILDERBOARD_DEV_INSTALL_DIR="$HOME/Applications" npm run runtime:build
BUILDERBOARD_DEV_INSTALL_DIR="$HOME/Applications" npm run runtime:launch
```

Use the same directory every time.

### Reset the local signing identity

Open Keychain Access, search for:

```text
BuilderBoard Local Development
```

Delete the certificate and private key, then rerun:

```sh
npm run runtime:setup
```

## Production Difference

This local certificate is for development only.

Public production distribution should use:

- Developer ID Application signing
- notarization
- stable production bundle identifier
- the same macOS Keychain storage model
