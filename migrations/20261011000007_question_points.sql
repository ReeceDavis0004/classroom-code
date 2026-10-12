CREATE TABLE question_points (
  question_id    uuid NOT NULL REFERENCES questions ON DELETE CASCADE,
  participant_id uuid NOT NULL REFERENCES participants ON DELETE CASCADE,
  submission_id  uuid NOT NULL REFERENCES submissions ON DELETE CASCADE,
  points         int NOT NULL CHECK (points BETWEEN 0 AND 1000),
  PRIMARY KEY (question_id, participant_id)
);
CREATE INDEX question_points_by_participant ON question_points (participant_id);
CREATE INDEX question_points_by_submission ON question_points (submission_id);
