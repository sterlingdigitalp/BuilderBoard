# Runtime First Release Checklist

*Before any release ships, every question must answer Yes.*

---

## The Checklist

| # | Question | Answer | Evidence |
|---|----------|--------|----------|
| 1 | **Does the application launch without crash?** | Yes / No | Screenshot or log showing startup complete |
| 2 | **Can a user send a message and receive a coherent response?** | Yes / No | Screenshot of chat exchange |
| 3 | **Can a tool call execute and return results?** | Yes / No | Screenshot of tool output in conversation |
| 4 | **Does the tool loop terminate correctly (no exhaust)?** | Yes / No | Log showing loop end < 10 rounds |
| 5 | **Have all Bronze Olympic events passed?** | Yes / No | Ledger entry for Bronze certification |
| 6 | **Is Runtime Certification current for this release?** | Yes / No | Certification date matches release version |

---

## If Any Answer is No

**The release is blocked.**

Do not ship. Do not bypass. Do not "ship now and fix later."

If the answer is No:

1. Record the blocker in the Runtime Ledger.
2. Fix the underlying issue.
3. Re-run the checklist.
4. Recertify.

---

## Evidence Requirements

Each Yes answer must be backed by evidence that can be independently verified:

- **For questions 1-4**: A screenshot or screen recording of the running application demonstrating the behavior.
- **For question 5**: A link to the ledger entry showing the Bronze certification results.
- **For question 6**: The current RUNTIME_CERTIFICATION.md with the certification date matching the release version.

Evidence is stored in `docs/runtime/certification/` alongside the certification snapshots.

---

## Emergency Exceptions

There are no emergency exceptions. An "emergency" release that bypasses the checklist is not safe. If a real emergency requires a hotfix, the hotfix must still pass the checklist against the hotfix build.

---

## Sign-Off

| Role | Signature | Date |
|------|-----------|------|
| Builder T | | |
| Builder V | | |
| Builder C | | |

All three signatures are required for release.
