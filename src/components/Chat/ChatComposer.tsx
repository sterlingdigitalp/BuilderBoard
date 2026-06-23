import type { CSSProperties, KeyboardEvent } from "react";

interface ChatComposerProps {
  value: string;
  disabled: boolean;
  canSend: boolean;
  onChange: (value: string) => void;
  onSend: () => Promise<void>;
}

const composerStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(0, 1fr) auto",
  gap: 8,
  padding: 10,
  borderTop: "1px solid var(--pane-border)"
};

const inputStyle: CSSProperties = {
  width: "100%",
  minWidth: 0,
  minHeight: 36,
  maxHeight: 92,
  resize: "vertical",
  border: "1px solid var(--pane-border)",
  borderRadius: 8,
  padding: "8px 9px",
  background: "var(--button-bg)",
  color: "var(--text-strong)",
  font: "inherit",
  fontSize: "0.82rem"
};

const buttonStyle: CSSProperties = {
  alignSelf: "end",
  height: 36,
  minWidth: 64,
  border: "1px solid var(--button-active-bg)",
  borderRadius: 8,
  background: "var(--button-active-bg)",
  color: "var(--button-active-fg)",
  font: "inherit",
  fontSize: "0.8rem",
  fontWeight: 700,
  cursor: "pointer"
};

const disabledButtonStyle: CSSProperties = {
  ...buttonStyle,
  opacity: 0.55,
  cursor: "default"
};

export function ChatComposer({
  value,
  disabled,
  canSend,
  onChange,
  onSend
}: ChatComposerProps) {
  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }

    event.preventDefault();

    if (canSend) {
      void onSend();
    }
  }

  return (
    <form
      style={composerStyle}
      onSubmit={(event) => {
        event.preventDefault();
        if (canSend) {
          void onSend();
        }
      }}
    >
      <textarea
        style={inputStyle}
        value={value}
        disabled={disabled}
        placeholder="Message OpenAI"
        aria-label="Message"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      <button type="submit" style={canSend ? buttonStyle : disabledButtonStyle} disabled={!canSend}>
        Send
      </button>
    </form>
  );
}
