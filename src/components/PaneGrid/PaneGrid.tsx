import { Pane } from "../Pane";
import { AccountsSettingsView } from "../Accounts";
import { usePersistentPanes } from "../../hooks/usePersistentPanes";
import { useShellView } from "../../hooks/useShellView";

interface PaneGridProps {
  panes?: unknown[];
}

export function PaneGrid(_props: PaneGridProps) {
  const view = useShellView();

  if (view === "accounts") {
    return <AccountsSettingsView />;
  }

  return <PersistentPaneGrid />;
}

function PersistentPaneGrid() {
  const { panes, isLoading, isMutating, error, createPane, closePane } = usePersistentPanes();

  if (isLoading) {
    return (
      <div className="pane-grid" aria-busy="true" aria-label="Loading pane layout">
        {Array.from({ length: 4 }, (_, index) => (
          <section className="pane" key={index} aria-label="Loading pane">
            <header className="pane__header">
              <h2>Loading</h2>
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
            <h2>No panes</h2>
            <button type="button" onClick={createPane} disabled={isMutating}>
              Add
            </button>
          </header>
          <div className="pane__body" />
        </section>
      )}
      {panes.map((pane, index) => (
        <Pane
          key={pane.id}
          pane={pane}
          onClose={closePane}
          isMutating={isMutating}
          onCreate={index === 0 ? createPane : undefined}
        />
      ))}
      {error ? (
        <section className="pane" aria-label="Pane persistence error">
          <header className="pane__header">
            <h2>Pane Error</h2>
          </header>
          <div className="pane__body" aria-live="polite">
            {error}
          </div>
        </section>
      ) : null}
    </div>
  );
}
