import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  accountCreateApiKey,
  accountDisconnect,
  accountList,
  oauthStart,
  providerList,
  toAccountDto,
  toOAuthCompleteEvent,
  toOAuthErrorEvent
} from "../stores/accountCommands";
import type {
  AccountCreateInput,
  AccountDto,
  OAuthConnectionStatus,
  ProviderDto
} from "../types/accounts";

const supportedProviderIds = ["openai", "anthropic", "google"];
const oauthProviderIds = ["openai", "google"];
type OAuthProviderId = "openai" | "google";

interface AccountsSettingsState {
  accounts: AccountDto[];
  providers: ProviderDto[];
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  oauthStatuses: Record<OAuthProviderId, OAuthConnectionStatus>;
  oauthMessages: Record<OAuthProviderId, string | null>;
  createApiKeyAccount: (input: AccountCreateInput) => Promise<void>;
  connectOpenAiOAuth: () => Promise<void>;
  connectGoogleOAuth: () => Promise<void>;
  disconnectAccount: (accountId: string) => Promise<void>;
  reloadAccounts: (options?: { silent?: boolean }) => Promise<void>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Account command failed.";
}

function supportedProviders(providers: ProviderDto[]): ProviderDto[] {
  const providerMap = new Map(providers.map((provider) => [provider.id, provider]));

  return supportedProviderIds.map((providerId) => {
    const provider = providerMap.get(providerId);

    if (provider) {
      return provider;
    }

    return {
      id: providerId,
      providerType: providerId,
      displayName: providerId[0].toUpperCase() + providerId.slice(1),
      enabled: false,
      authMode: providerId === "google" ? "oauth" : "api_key"
    };
  });
}

export function useAccountsSettings(): AccountsSettingsState {
  const [accounts, setAccounts] = useState<AccountDto[]>([]);
  const [providers, setProviders] = useState<ProviderDto[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [oauthStatuses, setOAuthStatuses] = useState<Record<OAuthProviderId, OAuthConnectionStatus>>({
    openai: "idle",
    google: "idle"
  });
  const [oauthMessages, setOAuthMessages] = useState<Record<OAuthProviderId, string | null>>({
    openai: null,
    google: null
  });

  const visibleProviders = useMemo(() => supportedProviders(providers), [providers]);

  const reloadAccounts = useCallback(async (options?: { silent?: boolean }) => {
    const silent = options?.silent === true;
    if (!silent) {
      setIsLoading(true);
    }
    setError(null);

    try {
      const [loadedProviders, loadedAccounts] = await Promise.all([providerList(), accountList()]);
      setProviders(loadedProviders);
      setAccounts(loadedAccounts);
    } catch (loadError) {
      setError(errorMessage(loadError));
      if (!silent) {
        setProviders([]);
        setAccounts([]);
      }
    } finally {
      if (!silent) {
        setIsLoading(false);
      }
    }
  }, []);

  const createApiKeyAccount = useCallback(async (input: AccountCreateInput) => {
    setIsMutating(true);
    setError(null);

    try {
      const createdAccount = await accountCreateApiKey(input);
      setAccounts((currentAccounts) => {
        const existingAccounts = createdAccount.isDefault
          ? currentAccounts.map((account) => ({
              ...account,
              isDefault: account.providerId === createdAccount.providerId ? false : account.isDefault
            }))
          : currentAccounts;

        return [...existingAccounts, createdAccount];
      });
    } catch (createError) {
      setError(errorMessage(createError));
    } finally {
      setIsMutating(false);
    }
  }, []);

  const setOAuthState = useCallback(
    (providerId: OAuthProviderId, status: OAuthConnectionStatus, message: string | null) => {
      setOAuthStatuses((currentStatuses) => ({ ...currentStatuses, [providerId]: status }));
      setOAuthMessages((currentMessages) => ({ ...currentMessages, [providerId]: message }));
    },
    []
  );

  const connectOAuth = useCallback(async (providerId: OAuthProviderId) => {
    setIsMutating(true);
    setError(null);
    setOAuthState(providerId, "starting", providerId === "openai" ? "Opening ChatGPT sign-in." : "Opening Google sign-in.");

    try {
      await oauthStart(providerId);
      setOAuthState(
        providerId,
        "waiting",
        providerId === "openai"
          ? "Waiting for ChatGPT authorization to complete."
          : "Waiting for Google authorization to complete."
      );
    } catch (connectError) {
      setOAuthState(providerId, "error", errorMessage(connectError));
    } finally {
      setIsMutating(false);
    }
  }, [setOAuthState]);

  const connectOpenAiOAuth = useCallback(() => connectOAuth("openai"), [connectOAuth]);
  const connectGoogleOAuth = useCallback(() => connectOAuth("google"), [connectOAuth]);

  const disconnectAccount = useCallback(async (accountId: string) => {
    setIsMutating(true);
    setError(null);

    try {
      await accountDisconnect(accountId);
      setAccounts((currentAccounts) =>
        currentAccounts.map((account) =>
          account.id === accountId
            ? { ...account, status: "revoked", isDefault: false }
            : account
        )
      );
    } catch (disconnectError) {
      setError(errorMessage(disconnectError));
    } finally {
      setIsMutating(false);
    }
  }, []);

  useEffect(() => {
    void reloadAccounts();
  }, [reloadAccounts]);

  useEffect(() => {
    let isActive = true;
    const cleanupFns: Array<() => void> = [];

    async function bindOAuthEvents() {
      try {
        cleanupFns.push(
          await listen("oauth_complete", (event) => {
            const payload = toOAuthCompleteEvent(event.payload);

            if (!payload || !oauthProviderIds.includes(payload.providerId) || !isActive) {
              return;
            }
            const providerId = payload.providerId as OAuthProviderId;

            setOAuthState(
              providerId,
              "connected",
              payload.label ? `${payload.label} connected.` : `${providerId} account connected.`
            );
            void reloadAccounts({ silent: true });
          })
        );

        cleanupFns.push(
          await listen("oauth_error", (event) => {
            const payload = toOAuthErrorEvent(event.payload);

            if (!payload || !oauthProviderIds.includes(payload.providerId) || !isActive) {
              return;
            }
            const providerId = payload.providerId as OAuthProviderId;

            setOAuthState(providerId, "error", payload.message ?? payload.errorCode ?? "OAuth failed.");
          })
        );

        cleanupFns.push(
          await listen("account_created", (event) => {
            if (!isActive) {
              return;
            }

            try {
              const account = toAccountDto(event.payload);
              if (!oauthProviderIds.includes(account.providerId) || account.status !== "active") {
                return;
              }
              const providerId = account.providerId as OAuthProviderId;

              setAccounts((currentAccounts) => {
                const withoutExisting = currentAccounts.filter((entry) => entry.id !== account.id);
                const normalized = account.isDefault
                  ? withoutExisting.map((entry) =>
                      entry.providerId === account.providerId
                        ? { ...entry, isDefault: false }
                        : entry
                    )
                  : withoutExisting;
                return [...normalized, account];
              });
              setOAuthState(providerId, "connected", `${account.label} connected.`);
            } catch {
              void reloadAccounts({ silent: true });
            }
          })
        );

        cleanupFns.push(
          await listen("account_status_changed", () => {
            if (isActive) {
              void reloadAccounts({ silent: true });
            }
          })
        );
      } catch (bindError) {
        setError(errorMessage(bindError));
        setOAuthState("openai", "error", "Could not subscribe to OAuth events.");
        setOAuthState("google", "error", "Could not subscribe to OAuth events.");
      }
    }

    void bindOAuthEvents();

    return () => {
      isActive = false;
      cleanupFns.forEach((cleanup) => cleanup());
    };
  }, [reloadAccounts, setOAuthState]);

  return {
    accounts,
    providers: visibleProviders,
    isLoading,
    isMutating,
    error,
    oauthStatuses,
    oauthMessages,
    createApiKeyAccount,
    connectOpenAiOAuth,
    connectGoogleOAuth,
    disconnectAccount,
    reloadAccounts
  };
}
