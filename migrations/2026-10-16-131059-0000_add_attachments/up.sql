CREATE TABLE attachments (
  id TEXT PRIMARY KEY NOT NULL,
  artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  mime TEXT NOT NULL,
  data BLOB NOT NULL,
  created BIGINT NOT NULL
);

CREATE INDEX attachments_by_artifact ON attachments(artifact_id);
