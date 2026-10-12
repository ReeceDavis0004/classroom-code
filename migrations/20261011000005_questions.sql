CREATE TABLE questions (
  id              uuid PRIMARY KEY,
  room_id         uuid NOT NULL REFERENCES rooms ON DELETE CASCADE,
  ordinal         int NOT NULL,
  title           text NOT NULL CHECK (length(title) BETWEEN 1 AND 200),
  statement_md    text NOT NULL DEFAULT '' CHECK (octet_length(statement_md) <= 65536),
  languages       text[] NOT NULL,
  time_limit_ms   int NOT NULL CHECK (time_limit_ms BETWEEN 100 AND 10000),
  memory_limit_mb int NOT NULL CHECK (memory_limit_mb BETWEEN 16 AND 1024),
  comparison      text NOT NULL DEFAULT 'whitespace' CHECK (comparison IN ('whitespace', 'exact')),
  scoring         text NOT NULL DEFAULT 'last' CHECK (scoring IN ('last', 'best')),
  status          text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published')),
  opens_at        timestamptz,
  closes_at       timestamptz,
  answer_time_s   int CHECK (answer_time_s BETWEEN 30 AND 3600),
  tests_version   int NOT NULL DEFAULT 1,
  created_at      timestamptz NOT NULL DEFAULT now(),
  UNIQUE (room_id, ordinal),
  CHECK (opens_at IS NULL OR closes_at IS NULL OR opens_at < closes_at)
);

ALTER TABLE rooms ADD CONSTRAINT rooms_current_question_fk
  FOREIGN KEY (current_question_id) REFERENCES questions ON DELETE SET NULL;
CREATE INDEX rooms_current_question ON rooms (current_question_id)
  WHERE current_question_id IS NOT NULL;

CREATE TABLE test_cases (
  question_id     uuid NOT NULL REFERENCES questions ON DELETE CASCADE,
  ordinal         int NOT NULL,
  stdin           bytea NOT NULL CHECK (octet_length(stdin) <= 1048576),
  expected_stdout bytea NOT NULL CHECK (octet_length(expected_stdout) <= 1048576),
  visibility      text NOT NULL CHECK (visibility IN ('sample', 'hidden')),
  weight          int NOT NULL DEFAULT 1 CHECK (weight BETWEEN 1 AND 100),
  PRIMARY KEY (question_id, ordinal)
);

GRANT SELECT (id, room_id, languages, time_limit_ms, memory_limit_mb, comparison, tests_version)
  ON questions TO app_worker;
GRANT SELECT ON test_cases TO app_worker;
