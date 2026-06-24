import type { ThemeMode } from "../../types/layout";
import type { ProjectDto } from "../../types/projects";
import { setShellView, useShellView } from "../../hooks/useShellView";
import { ProjectRail } from "./ProjectRail";

interface SidebarProps {
  theme: ThemeMode;
  onToggleTheme: () => void;
  projects: ProjectDto[];
  launcherProjectId: string;
  isLoadingProjects: boolean;
  isProjectMutating: boolean;
  onLaunchProject: (projectId: string) => Promise<void>;
  onCreateProject: () => Promise<void>;
}

export function Sidebar({
  theme,
  onToggleTheme,
  projects,
  launcherProjectId,
  isLoadingProjects,
  isProjectMutating,
  onLaunchProject,
  onCreateProject
}: SidebarProps) {
  const view = useShellView();
  const themeLabel = theme === "light" ? "Switch to dark theme" : "Switch to light theme";

  return (
    <aside className="sidebar" aria-label="Primary navigation">
      <div className="sidebar__brand" aria-label="BuilderBoard home">
        BB
      </div>
      <nav className="sidebar__nav" aria-label="Application sections">
        <button
          className={`sidebar__button${view === "board" ? " sidebar__button--active" : ""}`}
          type="button"
          aria-label="Board"
          onClick={() => setShellView("board")}
        >
          <span aria-hidden="true">□</span>
        </button>
        <button
          className={`sidebar__button${view === "accounts" ? " sidebar__button--active" : ""}`}
          type="button"
          aria-label="Accounts"
          onClick={() => setShellView("accounts")}
        >
          <span aria-hidden="true">◇</span>
        </button>
      </nav>
      <ProjectRail
        projects={projects}
        activeProjectId={launcherProjectId}
        isLoading={isLoadingProjects}
        isMutating={isProjectMutating}
        onCreateProject={() => {
          void onCreateProject();
          setShellView("board");
        }}
        onLaunchProject={onLaunchProject}
      />
      <button className="sidebar__button sidebar__theme" type="button" onClick={onToggleTheme} aria-label={themeLabel}>
        <span aria-hidden="true">{theme === "light" ? "☾" : "☼"}</span>
      </button>
    </aside>
  );
}