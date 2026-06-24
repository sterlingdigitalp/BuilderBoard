-- Align OpenAI account linking with ChatGPT/Codex login instead of user-created OAuth apps.

UPDATE providers
SET oauth_config_json = '{
  "authorization_url": "https://auth.openai.com/oauth/authorize",
  "token_url": "https://auth.openai.com/oauth/token",
  "revocation_url": "https://auth.openai.com/oauth/revoke",
  "scopes": [
    "openid",
    "profile",
    "email",
    "offline_access"
  ],
  "userinfo_url": ""
}',
    updated_at = '2026-06-23T00:00:00Z'
WHERE id = 'openai';
