import { useEffect, useRef } from "react";
import type { PaneDefinition } from "../../types/layout";
import type { ProjectDto } from "../../types/projects";
import { usePaneChat } from "../../hooks/usePaneChat";
import { ChatPane } from "../Chat";
import { ChatControls } from "../Chat/ChatControls";

interface PaneProps {
  pane: PaneDefinition;
  paneNumber: number;
  projects: ProjectDto[];
  project: ProjectDto | null;
  isFocused: boolean;
  onFocus: (paneId: string) => void;
  registerRef: (paneId: string, element: HTMLElement | null) => void;
  isMutating: boolean;
  onClose: (paneId: string) => Promise<void>;
  onSelectProject: (projectId: string) => void;
  onCreateProject: () => void;
  onCreate?: () => Promise<void>;
}

function statusLabel(state: string): string {
  if (state === "idle") {
    return "Ready";
  }

  if (state === "sending") {
    return "Queued";
  }

  if (state === "enriching") {
    return "Running";
  }

  if (state === "streaming") {
    return "Streaming";
  }

  if (state === "error") {
    return "Needs Attention";
  }

  return "Running";
}

function statusTone(state: string): string {
  if (state === "error") {
    return " pane__status--attention";
  }

  if (state === "sending" || state === "enriching" || state === "streaming") {
    return " pane__status--running";
  }

  return " pane__status--ready";
}

export function Pane({
  pane,
  paneNumber,
  projects,
  project,
  isFocused,
  onFocus,
  registerRef,
  isMutating,
  onClose,
  onSelectProject,
  onCreateProject,
  onCreate
}: PaneProps) {
  const paneRef = useRef<HTMLElement | null>(null);
  const chat = usePaneChat(pane);
  const isBusy =
    chat.displayState === "sending" ||
    chat.displayState === "enriching" ||
    chat.displayState === "streaming";
  const isControlsDisabled = isBusy || isMutating;
  const status = statusLabel(chat.displayState);

  useEffect(() => {
    registerRef(pane.id, paneRef.current);
    return () => registerRef(pane.id, null);
  }, [pane.id, registerRef]);

  return (
    <section
      ref={paneRef}
      className={`pane${isFocused ? " pane--focused" : ""}`}
      aria-labelledby={`${pane.id}-title`}
      onFocusCapture={() => onFocus(pane.id)}
      onPointerDownCapture={() => onFocus(pane.id)}
    >
      <header className="pane__header">
        <h2 id={`${pane.id}-title`}>{paneNumber}</h2>
        <ChatControls
          builders={chat.builders}
          selectedBuilderId={chat.selectedBuilderId}
          engines={chat.engines}
          selectedEngineId={chat.selectedEngineId}
          accounts={chat.accounts}
          selectedAccountId={chat.selectedAccountId}
          selectedModelId={chat.selectedModelId}
          selectedEffort={chat.selectedEffort}
          projects={projects}
          project={project}
          disabled={isControlsDisabled}
          builderError={chat.builderError}
          engineError={chat.engineError}
          accountError={chat.accountError}
          statusSlot={
            <span className={`pane__status${statusTone(chat.displayState)}`} aria-live="polite">
              {status}
            </span>
          }
          onSelectBuilder={chat.selectBuilder}
          onSelectEngine={chat.selectEngine}
          onSelectAccount={chat.setSelectedAccountId}
          onSelectModel={chat.selectModel}
          onSelectEffort={chat.selectEffort}
          onSelectProject={onSelectProject}
          onCreateProject={onCreateProject}
        />
        {onCreate ? (
          <button
            className="pane__icon-button"
            type="button"
            onClick={() => void onCreate()}
            disabled={isMutating}
            aria-label="Add pane"
          >
            +
          </button>
        ) : null}
        <button
          className="pane__icon-button"
          type="button"
          onClick={() => void onClose(pane.id)}
          disabled={isMutating}
          aria-label="Close pane"
        >
          ✕
        </button>
      </header>
      <div className="pane__body" aria-label={`Pane ${paneNumber} chat workspace`}>
        <ChatPane chat={chat} />
      </div>
    </section>
  );
}
