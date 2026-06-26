BB-0013 — Repeated Keychain authorization prompts prevent normal runtime use

Observed Runtime:

* BuilderBoard repeatedly prompts for the macOS login keychain password.
* This occurs during normal engineering use.
* The prompt returns even after previous authorization.

Expected Runtime:

* BuilderBoard should behave like Codex Desktop, Cursor, or OpenCode.
* Launch → Work.
* No repeated macOS password prompts during normal use.

Priority:

P0

Because this directly violates:

“Launch the application” and “normal engineering work.”

BB-0013

Mixed executable identities cause repeated Keychain authorization prompts

Not:

“Keychain is broken.”

The observed runtime problem is specifically that authenticated runtime testing is occurring across both the signed packaged application and the tauri dev/debug executable, resulting in mismatched Keychain ACLs and repeated authorization prompts.

The success criteria are clear:

* Launch only the packaged app.
* Authenticate once.
* Quit and relaunch repeatedly.
* No further Keychain prompts during normal authenticated use.

That makes BB-0013 a concrete, testable runtime issue rather than a vague Keychain problem, and it fits neatly into the Runtime Engineering Ledger alongside the other Version 1 blockers.