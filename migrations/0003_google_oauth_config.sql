-- Phase 3B: Google OAuth provider configuration (public metadata only)
UPDATE providers
SET oauth_config_json = '{
  "authorization_url": "https://accounts.google.com/o/oauth2/v2/auth",
  "token_url": "https://oauth2.googleapis.com/token",
  "revocation_url": "https://oauth2.googleapis.com/revoke",
  "scopes": [
    "openid",
    "email",
    "https://www.googleapis.com/auth/generative-language"
  ],
  "userinfo_url": "https://www.googleapis.com/oauth2/v3/userinfo"
}',
    updated_at = '2026-06-23T00:00:00Z'
WHERE id = 'google' AND auth_mode = 'oauth';