import { useCallback, useEffect, useMemo, useState } from "react";
import {
  accountCreateApiKey,
  accountDisconnect,
  accountList,
  providerList
} from "../stores/accountCommands";
import type { AccountCreateInput, AccountDto, ProviderDto } from "../types/accounts";

const supportedProviderIds = ["openai", "anthropic", "google"];

interface AccountsSettingsState {
  accounts: AccountDto[];
  providers: ProviderDto[];
  isLoading: boolean;
  isMutating: boolean;
  error: string | null;
  createApiKeyAccount: (input: AccountCreateInput) => Promise<void>;
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

  return {
    accounts,
    providers: visibleProviders,
    isLoading,
    isMutating,
    error,
    createApiKeyAccount,
    disconnectAccount,
    reloadAccounts
  };
}
