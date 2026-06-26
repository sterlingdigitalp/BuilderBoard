# Runtime User Experience Report

**Date:** 2026-06-27
**Evaluator:** Jules (AI Developer)
**Environment:** Linux Sandbox (Ubuntu 24.04), Node.js, Rust 1.77+, Xvfb (Virtual Framebuffer)

## Executive Summary

This report documents the findings from an attempt to run and evaluate the BuilderBoard application end-to-end as a developer on a Linux environment. The goal was to verify whether BuilderBoard could realistically replace four separate AI coding assistants by examining its usability, stability, and core workflows.

**Critical Limitation:** The local Linux environment lacks the macOS-specific components required to use the packaged runtime loops (`npm run runtime:build` and `npm run runtime:certify`). Furthermore, attempts to run the Tauri backend under Xvfb resulted in GTK initialization failures (`Could not get DRI3 device`). Because of this, testing had to rely on the Vite frontend development server (`npm run dev`) and visual inspection of the UI via Playwright in Chromium. The `window.__TAURI__` object is unavailable in standard browsers, leading to immediate JavaScript errors when the UI attempts to communicate with the backend.

Due to these constraints, a complete end-to-end evaluation of engineering workflows (like sending requests and receiving code) was not fully possible. However, significant UX friction points, startup issues, and UI behaviors were successfully evaluated.

---

## Findings

### 1. Startup and Environment Dependency Issues

**Steps to Reproduce:**
1. Clone the repository.
2. Run `npm install`.
3. Run `npm run dev` in a standard Linux environment.
4. Open the provided `localhost:1420` URL in a browser to view the UI.

**Expected Behavior:**
The application starts or gracefully warns the user about missing capabilities if running outside the packaged Tauri context.

**Observed Behavior:**
- Running `npm run dev` logs a warning: `BuilderBoard dev mode is for unauthenticated UI work only. Use npm run runtime:build -- --launch for authenticated runtime testing.`. This is a positive developer experience, clarifying the intent of the dev script.
- However, when opening the UI in a browser, attempting to perform *any* action (such as opening a pane or checking workspace status) results in a silent failure or an explicit crash message in the UI:
  - `Workspace Error: Cannot read properties of undefined (reading 'invoke')`
  - `Execution Failed: Cannot read properties of undefined (reading 'transformCallback')`
- **Severity:** High (for cross-platform developer experience).
- **Classification:** Runtime Bug / Documentation Issue.
- **Notes:** While `LOCAL_DEVELOPMENT_RUNTIME.md` specifies the macOS-only nature of the certification loop, `npm run dev` is documented as being available for "unauthenticated UI-only work." If the UI crashes immediately upon rendering because `window.__TAURI__` is undefined, the "UI-only work" becomes impossible without the Tauri shell. A mock implementation of the Tauri `invoke` API (or graceful degradation) is necessary for true UI-only development.

### 2. Multi-Pane Creation Experience

**Steps to Reproduce:**
1. Load the UI.
2. Observe the default layout.
3. Click the `+` button in the left sidebar to create a new pane.

**Expected Behavior:**
A new pane opens cleanly, ready for configuration.

**Observed Behavior:**
- The new pane opens with the default state.
- The UI immediately surfaces backend connectivity errors (`Cannot read properties of undefined (reading 'transformCallback')`) in a prominent red/error bar at the bottom of the pane (above the input box).
- The global Workspace error (`Cannot read properties of undefined (reading 'invoke')`) persists in the right pane.
- **Severity:** Medium (UX Friction).
- **Classification:** UX Issue.
- **Notes:** Error handling in the UI is highly visible, which is good for debugging, but the lack of fallback for missing Tauri bindings makes the UI feel brittle when tested in isolation.

### 3. Builder Configuration and Navigation

**Steps to Reproduce:**
1. Inspect the top bar of a newly created pane.
2. Attempt to configure the Builder (e.g., select project, model, role).

**Expected Behavior:**
Dropdowns and selectors are clearly labeled, easy to use, and provide immediate feedback.

**Observed Behavior:**
- The top bar of the pane contains dropdowns that are heavily truncated (e.g., `No Proje...`, `Builder re...`, `Model ur...`, `Med...`, `Engir...`, `No ac...`).
- This makes it difficult to understand what each dropdown controls without interacting with them.
- **Severity:** Medium (UX Friction).
- **Classification:** UX Issue.
- **Notes:** The aggressive truncation of labels in the configuration toolbar significantly harms discoverability. A user configuring four separate panes needs to be able to glance at the toolbar and understand the configuration (e.g., distinguishing "Builder C" from "Builder T"). The layout should probably allocate more space to these dropdowns or use icons alongside text.

### 4. Input Area and Chat Interface

**Steps to Reproduce:**
1. Look at the input area at the bottom of a pane.

**Expected Behavior:**
A clear affordance for typing multi-line engineering requests.

**Observed Behavior:**
- The input is a single-line text input field (`<input>`) rather than a multi-line `textarea`.
- It has a simple "Send" button.
- **Severity:** High (Workflow Friction).
- **Classification:** UX Issue.
- **Notes:** Software engineering tasks often require pasting code snippets, multi-line error logs, or detailed instructions. A single-line input field is a massive source of friction for a coding assistant. This should be a resizable `textarea` that supports multi-line pasting and enter-to-send (with shift-enter for newlines).

### 5. Multi-Pane Layout and Core Promise Evaluation

**Observation:**
The application effectively uses a tiled or tabbed layout to separate the workspaces.
However, if a user is meant to manage *four* independent AI engineers simultaneously, the visual noise of four separate toolbars, error bars, and chat histories could become overwhelming.

**Classification:** Workflow Friction.
**Notes:** The core promise is "four independent AI software engineers simultaneously from a single window." The current UI provides the independence (each pane has its own configuration), but the density of information (truncated dropdowns, persistent error banners) works against the "simultaneously from a single window" aspect. When scaled to four panes, the usable chat/code area will be severely limited unless the toolbars are streamlined.

---

## Summary of Recommendations

1. **Implement Tauri Mocking for UI Dev:** If `npm run dev` is truly meant for UI work, inject a mock `window.__TAURI__` object with dummy `invoke` responses so developers can work on components without the app crashing.
2. **Fix Dropdown Truncation:** Redesign the pane header. Consider moving secondary configuration (like role or model) into a settings modal or a collapsible panel to free up horizontal space.
3. **Multi-line Input:** Replace the single-line input field with a multi-line `textarea` that auto-expands.
4. **Linux/Cross-Platform CI:** The current testing and certification loop is heavily tied to macOS Keychain and code signing. To scale development, consider a headless or cross-platform mock capability for testing core planner logic without needing a macOS UI environment.

*End of Report*
