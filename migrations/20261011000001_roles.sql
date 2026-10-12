CREATE EXTENSION IF NOT EXISTS citext;

DO $$
BEGIN
  CREATE ROLE app_api NOLOGIN;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END $$;

DO $$
BEGIN
  CREATE ROLE app_worker NOLOGIN;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END $$;

GRANT USAGE ON SCHEMA public TO app_api, app_worker;
GRANT SELECT ON _sqlx_migrations TO app_api, app_worker;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_api;
