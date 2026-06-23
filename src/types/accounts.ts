export type AccountAuthType = "oauth" | "api_key";
export type AccountStatus = "active" | "expired" | "revoked" | "error";
export type ProviderAuthMode = "oauth" | "api_key" | "none" | "local";

export interface AccountDto {
  id: string;
  providerId: string;
  label: string;
  authType: AccountAuthType;
  externalEmail: string | null;
  status: AccountStatus;
  tokenExpiresAt: string | null;
  lastUsedAt: string | null;
  isDefault: boolean;
}

export type OAuthConnectionStatus = "idle" | "starting" | "waiting" | "connected" | "error";

export interface OAuthStartResult {
  authUrl: string | null;
}

export interface OAuthCompleteEvent {
  accountId: string;
  providerId: string;
  label?: string;
}

export interface OAuthErrorEvent {
  providerId: string;
  errorCode?: string;
  message?: string;
}

export interface ProviderDto {
  id: string;
  providerType: string;
  displayName: string;
  enabled: boolean;
  authMode: ProviderAuthMode;
}

export interface AccountCreateInput {
  providerId: string;
  label: string;
  apiKey: string;
  isDefault: boolean;
}
