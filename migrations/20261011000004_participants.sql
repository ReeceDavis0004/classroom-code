CREATE TABLE participants (
  id           uuid PRIMARY KEY,
  room_id      uuid NOT NULL REFERENCES rooms ON DELETE CASCADE,
  display_name text NOT NULL,
  name_key     text NOT NULL,
  avatar_id    smallint NOT NULL,
  token_hash   bytea NOT NULL UNIQUE,
  status       text NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'kicked')),
  joined_at    timestamptz NOT NULL DEFAULT now(),
  last_seen_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX participants_active_name ON participants (room_id, name_key)
  WHERE status = 'active';
CREATE INDEX participants_by_room ON participants (room_id);

CREATE TABLE rejoin_tokens (
  token_hash     bytea PRIMARY KEY,
  participant_id uuid NOT NULL REFERENCES participants ON DELETE CASCADE,
  created_at     timestamptz NOT NULL DEFAULT now(),
  expires_at     timestamptz NOT NULL,
  used_at        timestamptz
);
CREATE INDEX rejoin_tokens_by_participant ON rejoin_tokens (participant_id);
