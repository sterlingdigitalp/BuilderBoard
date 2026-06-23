import { AccountCreateForm } from "./AccountCreateForm";
import type {
  AccountCreateInput,
  AccountDto,
  OAuthConnectionStatus,
  ProviderDto
} from "../../types/accounts";

interface AccountProviderSectionProps {
  accounts: AccountDto[];
  provider: ProviderDto;
  isMutating: boolean;
  oauthMessage: string | null;
  oauthStatus: OAuthConnectionStatus;
  onCreate: (input: AccountCreateInput) => Promise<void>;
  onConnectOAuth: () => Promise<void>;
  onDisconnect: (accountId: string) => Promise<void>;
}

function statusLabel(status: AccountDto["status"]): string {
  return status[0].toUpperCase() + status.slice(1);
}

function authLabel(authType: AccountDto["authType"]): string {
  return authType === "api_key" ? "API key" : "OAuth";
}

function oauthStatusLabel(status: OAuthConnectionStatus): string {
  if (status === "idle") {
    return "OAuth ready";
  }

  return status[0].toUpperCase() + status.slice(1);
}

function metadataText(account: AccountDto): string {
  if (account.authType !== "oauth") {
    return "API-key account";
  }

  return [
    account.externalEmail ? `Email: ${account.externalEmail}` : "Email: Not provided",
    account.tokenExpiresAt ? `Expires: ${account.tokenExpiresAt}` : "Expires: Not provided",
    account.lastUsedAt ? `Last used: ${account.lastUsedAt}` : "Last used: Not yet"
  ].join(" · ");
}

export function AccountProviderSection({
  accounts,
  provider,
  isMutating,
  oauthMessage,
  oauthStatus,
  onCreate,
  onConnectOAuth,
  onDisconnect
}: AccountProviderSectionProps) {
  const providerAccounts = accounts.filter((account) => account.providerId === provider.id);
  const canConnectGoogle = provider.id === "google";

  return (
    <section className="pane" aria-labelledby={`${provider.id}-accounts-title`}>
      <header className="pane__header">
        <h2 id={`${provider.id}-accounts-title`}>{provider.displayName}</h2>
      </header>
      <div className="pane__body" style={{ display: "grid", gap: 14, overflow: "auto", padding: 14 }}>
        <AccountCreateForm provider={provider} isDisabled={isMutating} onCreate={onCreate} />
        {canConnectGoogle ? (
          <div
            style={{
              border: "1px solid var(--pane-border)",
              borderRadius: 8,
              display: "grid",
              gap: 8,
              padding: 10
            }}
          >
            <button type="button" disabled={isMutating || oauthStatus === "waiting"} onClick={() => void onConnectOAuth()}>
              Connect Google
            </button>
            <span style={{ color: "var(--button-fg)", fontSize: "0.82rem" }}>
              OAuth Status: {oauthStatusLabel(oauthStatus)}
            </span>
            {oauthMessage ? (
              <span style={{ color: "var(--button-fg)", fontSize: "0.82rem" }}>{oauthMessage}</span>
            ) : null}
          </div>
        ) : null}
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
                <span>Auth Type: {authLabel(account.authType)}</span>
                <span>Status: {statusLabel(account.status)}</span>
                <span>Default: {account.isDefault ? "Yes" : "No"}</span>
                {account.authType === "oauth" ? (
                  <span
                    style={{
                      border: "1px solid var(--button-active-bg)",
                      borderRadius: 8,
                      color: "var(--button-active-bg)",
                      fontSize: "0.74rem",
                      fontWeight: 700,
                      justifySelf: "start",
                      padding: "3px 7px"
                    }}
                  >
                    OAuth
                  </span>
                ) : null}
                <span style={{ color: "var(--button-fg)", fontSize: "0.82rem" }}>
                  {metadataText(account)}
                </span>
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
