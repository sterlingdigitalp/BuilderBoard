import { useAccountsSettings } from "../../hooks/useAccountsSettings";
import { AccountProviderSection } from "./AccountProviderSection";

export function AccountsSettingsView() {
  const {
    accounts,
    providers,
    isLoading,
    isMutating,
    error,
    oauthMessages,
    oauthStatuses,
    createApiKeyAccount,
    connectOpenAiOAuth,
    connectGoogleOAuth,
    disconnectAccount
  } = useAccountsSettings();

  if (isLoading) {
    return (
      <div className="pane-grid" aria-busy="true" aria-label="Loading accounts">
        {Array.from({ length: 3 }, (_, index) => (
          <section className="pane" key={index} aria-label="Loading account provider">
            <header className="pane__header">
              <h2>Loading</h2>
            </header>
            <div className="pane__body" />
          </section>
        ))}
      </div>
    );
  }

  return (
    <div className="pane-grid" aria-label="Accounts settings">
      {providers.map((provider) => (
        <AccountProviderSection
          key={provider.id}
          accounts={accounts}
          provider={provider}
          isMutating={isMutating}
          oauthMessage={provider.id === "openai" || provider.id === "google" ? oauthMessages[provider.id] : null}
          oauthStatus={provider.id === "openai" || provider.id === "google" ? oauthStatuses[provider.id] : "idle"}
          onCreate={createApiKeyAccount}
          onConnectOAuth={provider.id === "openai" ? connectOpenAiOAuth : connectGoogleOAuth}
          onDisconnect={disconnectAccount}
        />
      ))}
      {error ? (
        <section className="pane" aria-label="Account command error">
          <header className="pane__header">
            <h2>Account Error</h2>
          </header>
          <div className="pane__body" aria-live="polite" style={{ overflow: "auto", padding: 14 }}>
            {error}
          </div>
        </section>
      ) : null}
    </div>
  );
}
