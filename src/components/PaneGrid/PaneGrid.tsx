import { Pane } from "../Pane";
import { AccountsSettingsView } from "../Accounts";
import { useShellView } from "../../hooks/useShellView";
import type { PaneDto } from "../../types/layout";
import type { ProjectDto } from "../../types/projects";

interface PaneGridProps {
  projects: ProjectDto[];
  panes: PaneDto[];
  focusedPaneId: string | null;
  onFocusPane: (paneId: string) => void;
  registerPaneRef: (paneId: string, element: HTMLElement | null) => void;
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  projectError: string | null;
  onCreatePane: () => Promise<void>;
  onClosePane: (paneId: string) => Promise<void>;
  onChangePaneProject: (paneId: string, projectId: string, projects: ProjectDto[]) => Promise<void>;
  onCreateProjectForPane: (paneId: string) => Promise<void>;
}

export function PaneGrid({
  projects,
  panes,
  focusedPaneId,
  onFocusPane,
  registerPaneRef,
  isLoading,
  isMutating,
  error,
  projectError,
  onCreatePane,
  onClosePane,
  onChangePaneProject,
  onCreateProjectForPane
}: PaneGridProps) {
  const view = useShellView();

  if (view === "accounts") {
    return <AccountsSettingsView />;
  }

  if (isLoading) {
    return (
      <div className="pane-grid" aria-busy="true" aria-label="Loading pane layout">
        {Array.from({ length: 4 }, (_, index) => (
          <section className="pane" key={index} aria-label="Loading pane">
            <header className="pane__header">
              <h2>{index + 1}</h2>
            </header>
            <div className="pane__body" />
          </section>
        ))}
      </div>
    );
  }

  return (
    <div className="pane-grid" aria-label="Four pane layout">
      {panes.length === 0 && (
        <section className="pane" aria-label="No panes">
          <header className="pane__header">
            <h2>1</h2>
            <button
              className="pane__icon-button"
              type="button"
              onClick={() => void onCreatePane()}
              disabled={isMutating}
              aria-label="Add pane"
            >
              +
            </button>
          </header>
          <div className="pane__body" />
        </section>
      )}
      {panes.map((pane, index) => (
        <Pane
          key={pane.id}
          pane={pane}
          paneNumber={index + 1}
          projects={projects}
          project={projects.find((project) => project.id === pane.projectId) ?? null}
          isFocused={focusedPaneId === pane.id}
          onFocus={onFocusPane}
          registerRef={registerPaneRef}
          isMutating={isMutating}
          onClose={onClosePane}
          onSelectProject={(projectId) => void onChangePaneProject(pane.id, projectId, projects)}
          onCreateProject={() => void onCreateProjectForPane(pane.id)}
          onCreate={index === 0 ? onCreatePane : undefined}
        />
      ))}
      {error || projectError ? (
        <section className="pane" aria-label="Pane persistence error">
          <header className="pane__header">
            <h2>Project Error</h2>
          </header>
          <div className="pane__body" aria-live="polite">
            {error ?? projectError}
          </div>
        </section>
      ) : null}
    </div>
  );
}
