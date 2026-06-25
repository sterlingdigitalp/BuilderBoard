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
  gap: 4,
  padding: 4,
  borderTop: "1px solid var(--pane-border)"
};

const inputStyle: CSSProperties = {
  width: "100%",
  minWidth: 0,
  height: 30,
  minHeight: 30,
  maxHeight: 30,
  resize: "none",
  border: "1px solid var(--pane-border)",
  borderRadius: 6,
  padding: "5px 7px",
  background: "var(--button-bg)",
  color: "var(--text-strong)",
  font: "inherit",
  fontSize: "0.78rem"
};

const buttonStyle: CSSProperties = {
  alignSelf: "end",
  height: 30,
  minWidth: 52,
  border: "1px solid var(--button-active-bg)",
  borderRadius: 6,
  background: "var(--button-active-bg)",
  color: "var(--button-active-fg)",
  font: "inherit",
  fontSize: "0.76rem",
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
        placeholder="Message Builder"
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
