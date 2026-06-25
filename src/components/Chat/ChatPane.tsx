import type { CSSProperties } from "react";
import type { PaneChatState } from "../../hooks/usePaneChat";
import { ChatComposer } from "./ChatComposer";
import { MessageList } from "./MessageList";

interface ChatPaneProps {
  chat: PaneChatState;
}

const shellStyle: CSSProperties = {
  display: "grid",
  gridTemplateRows: "minmax(0, 1fr) auto auto",
  minHeight: 0,
  height: "100%"
};

const errorStyle: CSSProperties = {
  padding: "4px 6px",
  borderTop: "1px solid var(--pane-border)",
  color: "var(--text-strong)",
  background: "color-mix(in srgb, var(--button-active-bg) 12%, var(--pane-bg))",
  fontSize: "0.72rem",
  lineHeight: 1.35
};

export function ChatPane({ chat }: ChatPaneProps) {
  const isBusy = chat.displayState === "sending" || chat.displayState === "streaming";
  const isComposerDisabled = isBusy;

  return (
    <div style={shellStyle} aria-label="Chat interface">
      <MessageList messages={chat.messages} isLoading={chat.isMessageLoading} error={chat.messageError} />

      {chat.error ? (
        <div style={errorStyle} role="alert">
          <strong>Execution Failed</strong>
          <span> {chat.error}</span>
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
