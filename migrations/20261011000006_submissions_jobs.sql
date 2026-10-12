CREATE TABLE submissions (
  id              uuid PRIMARY KEY,
  room_id         uuid NOT NULL REFERENCES rooms ON DELETE CASCADE,
  question_id     uuid NOT NULL REFERENCES questions ON DELETE CASCADE,
  participant_id  uuid NOT NULL REFERENCES participants ON DELETE CASCADE,
  language        text NOT NULL,
  source          text NOT NULL CHECK (octet_length(source) <= 65536),
  idempotency_key uuid NOT NULL,
  received_at     timestamptz NOT NULL DEFAULT now(),
  status          text NOT NULL DEFAULT 'queued' CHECK (status IN
                    ('queued', 'running', 'done', 'superseded', 'cancelled')),
  verdict         text CHECK (verdict IN
                    ('accepted', 'wrong_answer', 'compile_error', 'runtime_error',
                     'time_limit_exceeded', 'memory_limit_exceeded',
                     'output_limit_exceeded', 'internal_error')),
  version         int NOT NULL DEFAULT 1,
  tests_version   int,
  passed_tests    int,
  total_tests     int,
  passed_weight   int,
  total_weight    int,
  compile_output  text CHECK (octet_length(compile_output) <= 16384),
  max_cpu_ms      int,
  max_memory_kb   int,
  judged_at       timestamptz,
  CHECK ((status = 'done') = (verdict IS NOT NULL)),
  UNIQUE (participant_id, idempotency_key)
);
CREATE UNIQUE INDEX submissions_one_queued
  ON submissions (participant_id, question_id) WHERE status = 'queued';
CREATE INDEX submissions_by_question
  ON submissions (question_id, participant_id, received_at DESC);
CREATE INDEX submissions_by_room ON submissions (room_id, received_at DESC);

CREATE TABLE jobs (
  id               uuid PRIMARY KEY,
  submission_id    uuid NOT NULL REFERENCES submissions ON DELETE CASCADE,
  kind             text NOT NULL DEFAULT 'judge' CHECK (kind IN ('judge', 'rejudge')),
  state            text NOT NULL DEFAULT 'queued' CHECK (state IN
                     ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
  priority         smallint NOT NULL DEFAULT 0,
  attempt          smallint NOT NULL DEFAULT 0,
  max_attempts     smallint NOT NULL DEFAULT 3,
  queued_at        timestamptz NOT NULL DEFAULT now(),
  run_after        timestamptz NOT NULL DEFAULT now(),
  started_at       timestamptz,
  finished_at      timestamptz,
  lease_owner      text,
  lease_token      uuid,
  lease_expires_at timestamptz,
  trace_parent     text,
  last_error       text,
  CHECK (state <> 'running'
         OR (lease_token IS NOT NULL AND lease_expires_at IS NOT NULL))
) WITH (fillfactor = 70);
CREATE INDEX jobs_claim ON jobs (priority DESC, queued_at) WHERE state = 'queued';
CREATE INDEX jobs_running ON jobs (id) WHERE state = 'running';
CREATE INDEX jobs_by_submission ON jobs (submission_id);

CREATE TABLE test_results (
  job_id         uuid NOT NULL REFERENCES jobs ON DELETE CASCADE,
  ordinal        int NOT NULL,
  verdict        text NOT NULL CHECK (verdict IN
                   ('accepted', 'wrong_answer', 'runtime_error',
                    'time_limit_exceeded', 'memory_limit_exceeded',
                    'output_limit_exceeded', 'skipped')),
  cpu_ms         int,
  wall_ms        int,
  memory_kb      int,
  exit_code      int,
  signal         text,
  stdout_excerpt bytea CHECK (octet_length(stdout_excerpt) <= 8192),
  stderr_excerpt bytea CHECK (octet_length(stderr_excerpt) <= 4096),
  PRIMARY KEY (job_id, ordinal)
);

GRANT SELECT (id, room_id, question_id, participant_id, language, source, received_at,
              status, verdict, version, tests_version)
  ON submissions TO app_worker;
GRANT UPDATE (status, verdict, version, tests_version, passed_tests, total_tests,
              passed_weight, total_weight, compile_output, max_cpu_ms, max_memory_kb, judged_at)
  ON submissions TO app_worker;
GRANT SELECT, UPDATE ON jobs TO app_worker;
GRANT INSERT ON test_results TO app_worker;
