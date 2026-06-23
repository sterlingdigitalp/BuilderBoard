import type { ThemeMode } from "../../types/layout";

interface SidebarProps {
  theme: ThemeMode;
  onToggleTheme: () => void;
}

export function Sidebar({ theme, onToggleTheme }: SidebarProps) {
  const themeLabel = theme === "light" ? "Switch to dark theme" : "Switch to light theme";

  return (
    <aside className="sidebar" aria-label="Primary navigation">
      <div className="sidebar__brand" aria-label="BuilderBoard home">
        BB
      </div>
      <nav className="sidebar__nav" aria-label="Workspace sections">
        <button className="sidebar__button sidebar__button--active" type="button" aria-label="Board">
          <span aria-hidden="true">□</span>
        </button>
        <button className="sidebar__button" type="button" aria-label="Library">
          <span aria-hidden="true">◇</span>
        </button>
      </nav>
      <button className="sidebar__button sidebar__theme" type="button" onClick={onToggleTheme} aria-label={themeLabel}>
        <span aria-hidden="true">{theme === "light" ? "☾" : "☼"}</span>
      </button>
    </aside>
  );
}
