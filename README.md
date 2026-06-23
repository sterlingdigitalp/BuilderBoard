# BuilderBoard

BuilderBoard is currently in Phase 1: a desktop application shell only.

## Phase 1 Scope

- Tauri 2.x desktop application structure
- React, TypeScript, and Vite frontend
- Fixed 64px left sidebar
- Four empty pane containers in a 2x2 workspace grid
- Light and dark theme support
- No backend integration, provider logic, persistence, settings modal, or chat functionality

## Project Structure

```text
src/
  components/
    Pane/
    PaneGrid/
    Sidebar/
  styles/
  types/
src-tauri/
```

## Validation

Run these commands from the project root:

```bash
npm run build
npm run typecheck
cargo check
```

Manual validation scenarios:

- Launch the app.
- Verify the left sidebar is visible.
- Verify four empty panes render in a 2x2 grid.
- Resize the window and confirm the desktop layout remains stable.
- Use the sidebar theme button and confirm light/dark themes switch.
