-- Phase 5B.5: align OpenAI OAuth metadata with live OIDC discovery.

UPDATE providers
SET oauth_config_json = '{
  "authorization_url": "https://auth.openai.com/authorize",
  "token_url": "https://auth0.openai.com/oauth/token",
  "revocation_url": "https://auth0.openai.com/oauth/revoke",
  "scopes": [
    "openid",
    "email",
    "profile",
    "offline_access"
  ],
  "userinfo_url": "https://auth0.openai.com/userinfo"
}',
    updated_at = '2026-06-23T00:00:00Z'
WHERE id = 'openai';
