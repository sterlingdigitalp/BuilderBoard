-- Phase 5B: OpenAI OAuth provider configuration (public metadata only).

UPDATE providers
SET oauth_config_json = '{
  "authorization_url": "https://auth.openai.com/oauth/authorize",
  "token_url": "https://auth.openai.com/oauth/token",
  "revocation_url": "https://auth.openai.com/oauth/revoke",
  "scopes": [
    "openid",
    "email",
    "offline_access",
    "api"
  ],
  "userinfo_url": "https://api.openai.com/v1/me"
}',
    updated_at = '2026-06-23T00:00:00Z'
WHERE id = 'openai';
