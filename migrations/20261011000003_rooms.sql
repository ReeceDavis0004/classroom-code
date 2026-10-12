CREATE TABLE rooms (
  id                  uuid PRIMARY KEY,
  professor_id        uuid NOT NULL REFERENCES professors ON DELETE CASCADE,
  title               text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
  mode                text NOT NULL CHECK (mode IN ('assignment', 'game')),
  status              text NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'closed')),
  accepting_joins     boolean NOT NULL DEFAULT true,
  share_token         text NOT NULL UNIQUE,
  game_state          text CHECK (game_state IN ('lobby', 'live', 'judging', 'reveal', 'podium')),
  current_question_id uuid,
  created_at          timestamptz NOT NULL DEFAULT now(),
  closed_at           timestamptz,
  purge_after         timestamptz,
  CHECK ((mode = 'game') = (game_state IS NOT NULL)),
  CHECK (mode = 'game' OR current_question_id IS NULL)
);
CREATE INDEX rooms_by_professor ON rooms (professor_id, created_at DESC);
CREATE INDEX rooms_purge ON rooms (purge_after) WHERE purge_after IS NOT NULL;
