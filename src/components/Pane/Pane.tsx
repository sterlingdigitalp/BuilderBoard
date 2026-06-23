import type { PaneDefinition } from "../../types/layout";

interface PaneProps {
  pane: PaneDefinition;
}

export function Pane({ pane }: PaneProps) {
  return (
    <section className="pane" aria-labelledby={`${pane.id}-title`}>
      <header className="pane__header">
        <h2 id={`${pane.id}-title`}>{pane.title}</h2>
      </header>
      <div className="pane__body" aria-label={`${pane.title} empty workspace`} />
    </section>
  );
}
