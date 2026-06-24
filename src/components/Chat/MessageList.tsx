import type { CSSProperties } from "react";
import type { MessageDto } from "../../types/chat";

interface MessageListProps {
  messages: MessageDto[];
  isLoading: boolean;
}

const listStyle: CSSProperties = {
  display: "grid",
  alignContent: "start",
  gap: 5,
  minHeight: 0,
  overflow: "auto",
  padding: 5
};

const emptyStyle: CSSProperties = {
  display: "grid",
  placeItems: "center",
  minHeight: 72,
  color: "var(--button-fg)",
  fontSize: "0.78rem",
  opacity: 0.72,
  textAlign: "center"
};

const messageStyle: CSSProperties = {
  display: "grid",
  gap: 3,
  maxWidth: "96%",
  padding: "5px 7px",
  border: "1px solid var(--pane-border)",
  borderRadius: 6,
  background: "var(--button-bg)",
  color: "var(--text-strong)",
  fontSize: "0.8rem",
  lineHeight: 1.34,
  whiteSpace: "pre-wrap",
  overflowWrap: "anywhere"
};

const userMessageStyle: CSSProperties = {
  ...messageStyle,
  justifySelf: "end",
  borderColor: "var(--button-active-bg)"
};

const metaStyle: CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: 5,
  fontSize: "0.6rem",
  fontWeight: 700,
  opacity: 0.7,
  textTransform: "uppercase"
};

const pendingText: Record<MessageDto["status"], string> = {
  pending: "Waiting for response...",
  streaming: "Streaming...",
  complete: "",
  error: "Assistant response failed."
};

function displayContent(message: MessageDto): string {
  if (message.content.trim().length > 0) {
    return message.content;
  }

  return pendingText[message.status] || "";
}

function roleLabel(role: MessageDto["role"]): string {
  return role === "user" ? "You" : role;
}

export function MessageList({ messages, isLoading }: MessageListProps) {
  if (isLoading) {
    return (
      <div style={emptyStyle} aria-busy="true">
        Loading messages
      </div>
    );
  }

  if (messages.length === 0) {
    return (
      <div style={emptyStyle} aria-label="No messages">
        Start a conversation in this pane.
      </div>
    );
  }

  return (
    <div style={listStyle} aria-label="Message list" aria-live="polite">
      {messages.map((message) => (
        <article
          key={message.id}
          style={message.role === "user" ? userMessageStyle : messageStyle}
          aria-label={`${roleLabel(message.role)} message, ${message.status}`}
        >
          <div style={metaStyle}>
            <span>{roleLabel(message.role)}</span>
            <span>{message.status}</span>
          </div>
          <div>{displayContent(message)}</div>
        </article>
      ))}
    </div>
  );
}
