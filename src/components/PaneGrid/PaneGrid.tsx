import { Pane } from "../Pane";
import type { PaneDefinition } from "../../types/layout";

interface PaneGridProps {
  panes: PaneDefinition[];
}

export function PaneGrid({ panes }: PaneGridProps) {
  return (
    <div className="pane-grid" aria-label="Four pane layout">
      {panes.map((pane) => (
        <Pane key={pane.id} pane={pane} />
      ))}
    </div>
  );
}
