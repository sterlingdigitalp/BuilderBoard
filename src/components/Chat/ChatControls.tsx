import type { CSSProperties } from "react";
import type { AccountDto } from "../../types/accounts";

interface ChatControlsProps {
  accounts: AccountDto[];
  selectedAccountId: string;
  selectedModelId: string;
  disabled: boolean;
  onSelectAccount: (accountId: string) => void;
  onSelectModel: (modelId: string) => void;
}

const rowStyle: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
  gap: 8,
  padding: "10px 10px 8px",
  borderBottom: "1px solid var(--pane-border)"
};

const fieldStyle: CSSProperties = {
  display: "grid",
  gap: 4,
  minWidth: 0
};

const labelStyle: CSSProperties = {
  fontSize: "0.67rem",
  fontWeight: 700,
  opacity: 0.7
};

const selectStyle: CSSProperties = {
  width: "100%",
  minWidth: 0,
  height: 30,
  border: "1px solid var(--pane-border)",
  borderRadius: 6,
  background: "var(--button-bg)",
  color: "var(--text-strong)",
  font: "inherit",
  fontSize: "0.75rem"
};

export function ChatControls({
  accounts,
  selectedAccountId,
  selectedModelId,
  disabled,
  onSelectAccount,
  onSelectModel
}: ChatControlsProps) {
  return (
    <div style={rowStyle} aria-label="Chat controls">
      <label style={fieldStyle}>
        <span style={labelStyle}>Provider</span>
        <select style={selectStyle} value="openai" disabled={disabled} aria-label="Provider">
          <option value="openai">OpenAI</option>
          <option value="anthropic" disabled>
            Anthropic
          </option>
          <option value="google" disabled>
            Google
          </option>
        </select>
      </label>

      <label style={fieldStyle}>
        <span style={labelStyle}>Account</span>
        <select
          style={selectStyle}
          value={selectedAccountId}
          disabled={disabled || accounts.length === 0}
          onChange={(event) => onSelectAccount(event.target.value)}
          aria-label="OpenAI API-key account"
        >
          {accounts.length === 0 ? (
            <option value="">No API-key account</option>
          ) : (
            accounts.map((account) => (
              <option key={account.id} value={account.id}>
                {account.label}
                {account.isDefault ? " (default)" : ""}
              </option>
            ))
          )}
        </select>
      </label>

      <label style={fieldStyle}>
        <span style={labelStyle}>Model</span>
        <select
          style={selectStyle}
          value={selectedModelId}
          disabled={disabled}
          onChange={(event) => onSelectModel(event.target.value)}
          aria-label="OpenAI model"
        >
          <option value="OpenAIGpt">OpenAI GPT</option>
        </select>
      </label>
    </div>
  );
}
