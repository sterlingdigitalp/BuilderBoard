import { useMemo, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { PaneGrid } from "./components/PaneGrid";
import { usePaneWorkspace } from "./hooks/usePaneWorkspace";
import type { ThemeMode } from "./types/layout";

export function App() {
  const [theme, setTheme] = useState<ThemeMode>("light");
  const workspace = usePaneWorkspace();

  const nextTheme = useMemo<ThemeMode>(
    () => (theme === "light" ? "dark" : "light"),
    [theme]
  );

  return (
    <div className="app-shell" data-theme={theme}>
      <Sidebar
        theme={theme}
        onToggleTheme={() => setTheme(nextTheme)}
        projects={workspace.projects}
        launcherProjectId={workspace.launcherProjectId}
        isLoadingProjects={workspace.isLoadingProjects}
        isProjectMutating={workspace.isProjectMutating}
        onLaunchProject={workspace.launchProjectPane}
        onCreateProject={workspace.createProjectAndLaunchPane}
      />
      <main className="workspace" aria-label="BuilderBoard workspace">
        <PaneGrid
          projects={workspace.projects}
          panes={workspace.panes}
          focusedPaneId={workspace.focusedPaneId}
          onFocusPane={workspace.focusPane}
          registerPaneRef={workspace.registerPaneRef}
          isLoading={workspace.isLoading}
          isMutating={workspace.isMutating}
          error={workspace.error}
          projectError={workspace.projectError}
          onCreatePane={workspace.createPane}
          onClosePane={workspace.closePane}
          onChangePaneProject={workspace.changePaneProject}
          onCreateProjectForPane={workspace.bindPaneToNewProject}
        />
      </main>
    </div>
  );
}
