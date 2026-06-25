import type { CSSProperties } from "react";
import type { MessageDto } from "../../types/chat";

interface MessageListProps {
  messages: MessageDto[];
  isLoading: boolean;
  error: string | null;
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
  pending: "Queued...",
  streaming: "Streaming...",
  complete: "",
  error: "Execution failed."
};

function displayContent(message: MessageDto): string {
  if (message.content.trim().length > 0) {
    return message.content;
  }

  return pendingText[message.status] || "";
}

function roleLabel(role: MessageDto["role"]): string {
  if (role === "user") {
    return "You";
  }

  if (role === "assistant") {
    return "Builder";
  }

  return role === "tool" ? "Tool" : "System";
}

function statusLabel(status: MessageDto["status"]): string {
  if (status === "complete") {
    return "";
  }

  if (status === "pending") {
    return "Queued";
  }

  if (status === "error") {
    return "Needs Attention";
  }

  return "Streaming";
}

export function MessageList({ messages, isLoading, error }: MessageListProps) {
  if (isLoading) {
    return (
      <div style={emptyStyle} aria-busy="true">
        Loading messages
      </div>
    );
  }

  if (error) {
    return (
      <div style={emptyStyle} aria-label="Message history unavailable">
        {error}
      </div>
    );
  }

  if (messages.length === 0) {
    return (
      <div style={emptyStyle} aria-label="No messages">
        Builder is ready.
      </div>
    );
  }

  return (
    <div style={listStyle} aria-label="Message list" aria-live="polite">
      {messages.map((message) => (
        <article
          key={message.id}
          style={message.role === "user" ? userMessageStyle : messageStyle}
          aria-label={`${roleLabel(message.role)} message${statusLabel(message.status) ? `, ${statusLabel(message.status)}` : ""}`}
        >
          <div style={metaStyle}>
            <span>{roleLabel(message.role)}</span>
            {statusLabel(message.status) ? <span>{statusLabel(message.status)}</span> : null}
          </div>
          <div>{displayContent(message)}</div>
        </article>
      ))}
    </div>
  );
}
