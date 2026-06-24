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
    return "";
  }

  if (state === "enriching") {
    return "gathering context";
  }

  return state;
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
  const isControlsDisabled = chat.isLoading || isBusy || isMutating;
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
          accounts={chat.accounts}
          selectedAccountId={chat.selectedAccountId}
          selectedModelId={chat.selectedModelId}
          selectedReasoningLevel={chat.selectedReasoningLevel}
          projects={projects}
          project={project}
          disabled={isControlsDisabled}
          onSelectAccount={chat.setSelectedAccountId}
          onSelectModel={chat.selectModel}
          onSelectReasoning={chat.selectReasoning}
          onSelectProject={onSelectProject}
          onCreateProject={onCreateProject}
        />
        {status ? (
          <span className="pane__status" aria-live="polite">
            {status}
          </span>
        ) : null}
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
