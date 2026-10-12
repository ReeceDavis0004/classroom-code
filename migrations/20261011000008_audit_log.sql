CREATE TABLE audit_log (
  id                 uuid PRIMARY KEY,
  at                 timestamptz NOT NULL DEFAULT now(),
  actor_professor_id uuid,
  action             text NOT NULL,
  room_id            uuid,
  target_id          uuid,
  detail             jsonb NOT NULL DEFAULT '{}'
);
CREATE INDEX audit_log_by_room ON audit_log (room_id, at);
