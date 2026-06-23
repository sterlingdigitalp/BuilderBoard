import { AccountCreateForm } from "./AccountCreateForm";
import type { AccountCreateInput, AccountDto, ProviderDto } from "../../types/accounts";

interface AccountProviderSectionProps {
  accounts: AccountDto[];
  provider: ProviderDto;
  isMutating: boolean;
  onCreate: (input: AccountCreateInput) => Promise<void>;
  onDisconnect: (accountId: string) => Promise<void>;
}

function statusLabel(status: AccountDto["status"]): string {
  return status[0].toUpperCase() + status.slice(1);
}

export function AccountProviderSection({
  accounts,
  provider,
  isMutating,
  onCreate,
  onDisconnect
}: AccountProviderSectionProps) {
  const providerAccounts = accounts.filter((account) => account.providerId === provider.id);

  return (
    <section className="pane" aria-labelledby={`${provider.id}-accounts-title`}>
      <header className="pane__header">
        <h2 id={`${provider.id}-accounts-title`}>{provider.displayName}</h2>
      </header>
      <div className="pane__body" style={{ display: "grid", gap: 14, overflow: "auto", padding: 14 }}>
        <AccountCreateForm provider={provider} isDisabled={isMutating} onCreate={onCreate} />
        <div style={{ display: "grid", gap: 8 }}>
          {providerAccounts.length === 0 ? (
            <div style={{ color: "var(--button-fg)", fontSize: "0.82rem" }}>No accounts connected.</div>
          ) : (
            providerAccounts.map((account) => (
              <div
                key={account.id}
                style={{
                  border: "1px solid var(--pane-border)",
                  borderRadius: 8,
                  display: "grid",
                  gap: 8,
                  padding: 10
                }}
              >
                <span>Provider: {provider.displayName}</span>
                <strong>{account.label}</strong>
                <span>Auth Type: {account.authType === "api_key" ? "API key" : "OAuth"}</span>
                <span>Status: {statusLabel(account.status)}</span>
                <span>Default: {account.isDefault ? "Yes" : "No"}</span>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button
                    type="button"
                    disabled={isMutating || account.status === "revoked"}
                    onClick={() => void onDisconnect(account.id)}
                  >
                    Disconnect
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </section>
  );
}
