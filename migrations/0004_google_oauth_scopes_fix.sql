-- Phase 3B fix: remove invalid generative-language scope from Google OAuth config.
-- Account identity requires openid + email only; Gemini API scopes deferred to Phase 4.
UPDATE providers
SET oauth_config_json = '{
  "authorization_url": "https://accounts.google.com/o/oauth2/v2/auth",
  "token_url": "https://oauth2.googleapis.com/token",
  "revocation_url": "https://oauth2.googleapis.com/revoke",
  "scopes": [
    "openid",
    "email"
  ],
  "userinfo_url": "https://www.googleapis.com/oauth2/v3/userinfo"
}',
    updated_at = '2026-06-23T00:00:00Z'
WHERE id = 'google' AND auth_mode = 'oauth';