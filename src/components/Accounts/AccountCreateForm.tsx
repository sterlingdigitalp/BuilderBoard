import { useState } from "react";
import type { FormEvent } from "react";
import type { AccountCreateInput, ProviderDto } from "../../types/accounts";

interface AccountCreateFormProps {
  provider: ProviderDto;
  isDisabled: boolean;
  onCreate: (input: AccountCreateInput) => Promise<void>;
}

export function AccountCreateForm({ provider, isDisabled, onCreate }: AccountCreateFormProps) {
  const [label, setLabel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [isDefault, setIsDefault] = useState(false);

  const canCreate = provider.authMode === "api_key" && label.trim() !== "" && apiKey.trim() !== "";

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!canCreate) {
      return;
    }

    await onCreate({
      providerId: provider.id,
      label: label.trim(),
      apiKey: apiKey.trim(),
      isDefault
    });
    setLabel("");
    setApiKey("");
    setIsDefault(false);
  }

  if (provider.authMode !== "api_key") {
    return (
      <div style={{ color: "var(--button-fg)", fontSize: "0.82rem" }}>
        API accounts are not available for this service.
      </div>
    );
  }

  return (
    <form onSubmit={handleSubmit} style={{ display: "grid", gap: 8 }}>
      <input
        aria-label={`${provider.displayName} account label`}
        disabled={isDisabled}
        onChange={(event) => setLabel(event.target.value)}
        placeholder="Account label"
        value={label}
      />
      <input
        aria-label={`${provider.displayName} API key`}
        autoComplete="off"
        disabled={isDisabled}
        onChange={(event) => setApiKey(event.target.value)}
        placeholder="API key"
        type="password"
        value={apiKey}
      />
      <label style={{ alignItems: "center", display: "flex", gap: 8, fontSize: "0.82rem" }}>
        <input
          checked={isDefault}
          disabled={isDisabled}
          onChange={(event) => setIsDefault(event.target.checked)}
          type="checkbox"
        />
        Default account
      </label>
      <button type="submit" disabled={isDisabled || !canCreate}>
        Create API Account
      </button>
    </form>
  );
}
