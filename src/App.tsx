import { useMemo, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { PaneGrid } from "./components/PaneGrid";
import type { ThemeMode } from "./types/layout";

const panes = [
  { id: "pane-1", title: "Pane 1" },
  { id: "pane-2", title: "Pane 2" },
  { id: "pane-3", title: "Pane 3" },
  { id: "pane-4", title: "Pane 4" }
];

export function App() {
  const [theme, setTheme] = useState<ThemeMode>("light");

  const nextTheme = useMemo<ThemeMode>(
    () => (theme === "light" ? "dark" : "light"),
    [theme]
  );

  return (
    <div className="app-shell" data-theme={theme}>
      <Sidebar
        theme={theme}
        onToggleTheme={() => setTheme(nextTheme)}
      />
      <main className="workspace" aria-label="BuilderBoard workspace">
        <PaneGrid panes={panes} />
      </main>
    </div>
  );
}
