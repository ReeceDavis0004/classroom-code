CREATE TABLE professors (
  id            uuid PRIMARY KEY,
  email         citext NOT NULL UNIQUE CHECK (length(email) <= 254),
  password_hash text NOT NULL,
  display_name  text NOT NULL CHECK (length(display_name) BETWEEN 1 AND 100),
  status        text NOT NULL DEFAULT 'unverified'
                  CHECK (status IN ('unverified', 'active', 'disabled')),
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE professor_sessions (
  id_hash      bytea PRIMARY KEY,
  public_id    uuid NOT NULL UNIQUE,
  professor_id uuid NOT NULL REFERENCES professors ON DELETE CASCADE,
  csrf_token   text NOT NULL,
  created_at   timestamptz NOT NULL DEFAULT now(),
  last_seen_at timestamptz NOT NULL DEFAULT now(),
  expires_at   timestamptz NOT NULL,
  revoked_at   timestamptz
);
CREATE INDEX professor_sessions_by_professor ON professor_sessions (professor_id);
CREATE INDEX professor_sessions_by_expiry ON professor_sessions (expires_at);

CREATE TABLE professor_invites (
  code_hash  bytea PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL,
  used_at    timestamptz
);

CREATE TABLE email_tokens (
  token_hash   bytea PRIMARY KEY,
  professor_id uuid NOT NULL REFERENCES professors ON DELETE CASCADE,
  purpose      text NOT NULL CHECK (purpose IN ('verify', 'reset')),
  created_at   timestamptz NOT NULL DEFAULT now(),
  expires_at   timestamptz NOT NULL,
  used_at      timestamptz
);
CREATE INDEX email_tokens_by_professor ON email_tokens (professor_id);
