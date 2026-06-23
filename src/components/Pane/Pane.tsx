import type { PaneDefinition } from "../../types/layout";
import { ChatPane } from "../Chat";

interface PaneProps {
  pane: PaneDefinition;
  isMutating: boolean;
  onClose: (paneId: string) => Promise<void>;
  onCreate?: () => Promise<void>;
}

function paneTitle(pane: PaneDefinition): string {
  return pane.title?.trim() || "Untitled Pane";
}

export function Pane({ pane, isMutating, onClose, onCreate }: PaneProps) {
  const title = paneTitle(pane);

  return (
    <section className="pane" aria-labelledby={`${pane.id}-title`}>
      <header className="pane__header">
        <h2 id={`${pane.id}-title`}>{title}</h2>
        {onCreate ? (
          <button type="button" onClick={() => void onCreate()} disabled={isMutating}>
            Add
          </button>
        ) : null}
        <button type="button" onClick={() => void onClose(pane.id)} disabled={isMutating}>
          Close
        </button>
      </header>
      <div className="pane__body" aria-label={`${title} chat workspace`}>
        <ChatPane pane={pane} />
      </div>
    </section>
  );
}
