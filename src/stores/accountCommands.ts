import { invoke } from "@tauri-apps/api/core";
import type {
  AccountCreateInput,
  AccountDto,
  AccountStatus,
  AccountAuthType,
  OAuthCompleteEvent,
  OAuthErrorEvent,
  OAuthStartResult,
  ProviderAuthMode,
  ProviderDto
} from "../types/accounts";

const accountStatuses: AccountStatus[] = ["active", "expired", "revoked", "error"];
const accountAuthTypes: AccountAuthType[] = ["oauth", "api_key"];
const providerAuthModes: ProviderAuthMode[] = ["oauth", "api_key", "none", "local"];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function accountStatus(value: unknown): AccountStatus {
  return typeof value === "string" && accountStatuses.includes(value as AccountStatus)
    ? (value as AccountStatus)
    : "error";
}

function accountAuthType(value: unknown): AccountAuthType {
  return typeof value === "string" && accountAuthTypes.includes(value as AccountAuthType)
    ? (value as AccountAuthType)
    : "api_key";
}

function providerAuthMode(value: unknown): ProviderAuthMode {
  return typeof value === "string" && providerAuthModes.includes(value as ProviderAuthMode)
    ? (value as ProviderAuthMode)
    : "none";
}

export function toAccountDto(value: unknown): AccountDto {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.providerId !== "string") {
    throw new Error("Invalid account response from account service.");
  }

  return {
    id: value.id,
    providerId: value.providerId,
    label: typeof value.label === "string" ? value.label : "Untitled account",
    authType: accountAuthType(value.authType),
    externalEmail: nullableString(value.externalEmail),
    status: accountStatus(value.status),
    tokenExpiresAt: nullableString(value.tokenExpiresAt),
    lastUsedAt: nullableString(value.lastUsedAt),
    isDefault: value.isDefault === true
  };
}

export function toProviderDto(value: unknown): ProviderDto {
  if (!isRecord(value) || typeof value.id !== "string") {
    throw new Error("Invalid provider response from provider registry.");
  }

  return {
    id: value.id,
    providerType: typeof value.providerType === "string" ? value.providerType : value.id,
    displayName: typeof value.displayName === "string" ? value.displayName : value.id,
    enabled: value.enabled !== false,
    authMode: providerAuthMode(value.authMode)
  };
}

function toOAuthStartResult(value: unknown): OAuthStartResult {
  if (!isRecord(value)) {
    return { authUrl: null };
  }

  return {
    authUrl: nullableString(value.authUrl)
  };
}

export function toOAuthCompleteEvent(value: unknown): OAuthCompleteEvent | null {
  if (!isRecord(value) || typeof value.accountId !== "string" || typeof value.providerId !== "string") {
    return null;
  }

  return {
    accountId: value.accountId,
    providerId: value.providerId,
    label: typeof value.label === "string" ? value.label : undefined
  };
}

export function toOAuthErrorEvent(value: unknown): OAuthErrorEvent | null {
  if (!isRecord(value) || typeof value.providerId !== "string") {
    return null;
  }

  return {
    providerId: value.providerId,
    errorCode: typeof value.errorCode === "string" ? value.errorCode : undefined,
    message: typeof value.message === "string" ? value.message : undefined
  };
}

// Account command DTOs are metadata-only; secrets are submitted but never returned.
export async function accountList(providerId?: string): Promise<AccountDto[]> {
  const response = await invoke<unknown[]>("account_list", { providerId });
  return response.map(toAccountDto);
}

export async function accountCreateApiKey(input: AccountCreateInput): Promise<AccountDto> {
  const response = await invoke<unknown>("account_create_api_key", {
    providerId: input.providerId,
    label: input.label,
    apiKey: input.apiKey,
    isDefault: input.isDefault
  });
  return toAccountDto(response);
}

export async function accountDisconnect(accountId: string): Promise<void> {
  await invoke("account_disconnect", { accountId });
}

export async function oauthStart(providerId: string): Promise<OAuthStartResult> {
  const response = await invoke<unknown>("oauth_start", { providerId });
  return toOAuthStartResult(response);
}

export async function providerList(): Promise<ProviderDto[]> {
  const response = await invoke<unknown[]>("provider_list");
  return response.map(toProviderDto);
}
