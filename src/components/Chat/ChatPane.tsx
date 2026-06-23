import type { CSSProperties } from "react";
import { usePaneChat } from "../../hooks/usePaneChat";
import type { PaneDefinition } from "../../types/layout";
import { ChatComposer } from "./ChatComposer";
import { ChatControls } from "./ChatControls";
import { MessageList } from "./MessageList";

interface ChatPaneProps {
  pane: PaneDefinition;
}

const shellStyle: CSSProperties = {
  display: "grid",
  gridTemplateRows: "auto minmax(0, 1fr) auto auto",
  minHeight: 0,
  height: "100%"
};

const statusStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: 8,
  padding: "6px 10px",
  borderBottom: "1px solid var(--pane-border)",
  color: "var(--button-fg)",
  fontSize: "0.72rem"
};

const errorStyle: CSSProperties = {
  padding: "7px 10px",
  borderTop: "1px solid var(--pane-border)",
  color: "var(--text-strong)",
  background: "color-mix(in srgb, var(--button-active-bg) 12%, var(--pane-bg))",
  fontSize: "0.76rem",
  lineHeight: 1.35
};

function statusLabel(state: string): string {
  switch (state) {
    case "sending":
      return "Sending";
    case "streaming":
      return "Streaming";
    case "error":
      return "Error";
    default:
      return "Idle";
  }
}

export function ChatPane({ pane }: ChatPaneProps) {
  const chat = usePaneChat(pane);
  const isBusy = chat.displayState === "sending" || chat.displayState === "streaming";
  const isComposerDisabled = chat.isLoading || isBusy;

  return (
    <div style={shellStyle} aria-label="Chat interface">
      <ChatControls
        accounts={chat.accounts}
        selectedAccountId={chat.selectedAccountId}
        selectedModelId={chat.selectedModelId}
        disabled={isComposerDisabled}
        onSelectAccount={chat.setSelectedAccountId}
        onSelectModel={chat.setSelectedModelId}
      />

      <div style={statusStyle} aria-live="polite">
        <span>{statusLabel(chat.displayState)}</span>
        <span>OpenAI API key</span>
      </div>

      <MessageList messages={chat.messages} isLoading={chat.isLoading} />

      {chat.error ? (
        <div style={errorStyle} role="alert">
          {chat.error}
        </div>
      ) : null}

      <ChatComposer
        value={chat.inputValue}
        disabled={isComposerDisabled}
        canSend={chat.canSend}
        onChange={chat.setInputValue}
        onSend={chat.sendMessage}
      />
    </div>
  );
}
