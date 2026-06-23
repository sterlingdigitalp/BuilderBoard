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
