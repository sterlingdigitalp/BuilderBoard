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

interface AccountsSettingsState {
  accounts: AccountDto[];
  providers: ProviderDto[];
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  oauthStatus: OAuthConnectionStatus;
  oauthMessage: string | null;
  createApiKeyAccount: (input: AccountCreateInput) => Promise<void>;
  connectGoogleOAuth: () => Promise<void>;
  disconnectAccount: (accountId: string) => Promise<void>;
  reloadAccounts: () => Promise<void>;
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
  const [oauthStatus, setOAuthStatus] = useState<OAuthConnectionStatus>("idle");
  const [oauthMessage, setOAuthMessage] = useState<string | null>(null);

  const visibleProviders = useMemo(() => supportedProviders(providers), [providers]);

  const reloadAccounts = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const [loadedProviders, loadedAccounts] = await Promise.all([providerList(), accountList()]);
      setProviders(loadedProviders);
      setAccounts(loadedAccounts);
    } catch (loadError) {
      setError(errorMessage(loadError));
      setProviders([]);
      setAccounts([]);
    } finally {
      setIsLoading(false);
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

  const connectGoogleOAuth = useCallback(async () => {
    setIsMutating(true);
    setError(null);
    setOAuthStatus("starting");
    setOAuthMessage("Opening Google sign-in.");

    try {
      await oauthStart("google");
      setOAuthStatus("waiting");
      setOAuthMessage("Waiting for Google authorization to complete.");
    } catch (connectError) {
      setOAuthStatus("error");
      setOAuthMessage(errorMessage(connectError));
    } finally {
      setIsMutating(false);
    }
  }, []);

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
      cleanupFns.push(
        await listen("oauth_complete", (event) => {
          const payload = toOAuthCompleteEvent(event.payload);

          if (!payload || payload.providerId !== "google" || !isActive) {
            return;
          }

          setOAuthStatus("connected");
          setOAuthMessage("Google account connected.");
          void reloadAccounts();
        })
      );

      cleanupFns.push(
        await listen("oauth_error", (event) => {
          const payload = toOAuthErrorEvent(event.payload);

          if (!payload || payload.providerId !== "google" || !isActive) {
            return;
          }

          setOAuthStatus("error");
          setOAuthMessage(payload.message ?? payload.errorCode ?? "Google OAuth failed.");
        })
      );

      cleanupFns.push(
        await listen("account_created", (event) => {
          const account = toAccountDto(event.payload);

          if (account.providerId !== "google" || !isActive) {
            return;
          }

          void reloadAccounts();
        })
      );

      cleanupFns.push(
        await listen("account_status_changed", () => {
          if (isActive) {
            void reloadAccounts();
          }
        })
      );
    }

    void bindOAuthEvents();

    return () => {
      isActive = false;
      cleanupFns.forEach((cleanup) => cleanup());
    };
  }, [reloadAccounts]);

  return {
    accounts,
    providers: visibleProviders,
    isLoading,
    isMutating,
    error,
    oauthStatus,
    oauthMessage,
    createApiKeyAccount,
    connectGoogleOAuth,
    disconnectAccount,
    reloadAccounts
  };
}
