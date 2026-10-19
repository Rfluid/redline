CREATE TABLE artifacts (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  uri TEXT,
  kind TEXT NOT NULL,
  session TEXT,
  token TEXT NOT NULL,
  created BIGINT NOT NULL
);

CREATE TABLE threads (
  id TEXT PRIMARY KEY NOT NULL,
  artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  anchor TEXT NOT NULL,
  status TEXT NOT NULL,
  created BIGINT NOT NULL,
  updated BIGINT NOT NULL
);

CREATE TABLE messages (
  seq INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  id TEXT NOT NULL UNIQUE,
  thread_id TEXT NOT NULL REFERENCES threads(id),
  author TEXT NOT NULL,
  body TEXT NOT NULL,
  created BIGINT NOT NULL
);

CREATE TABLE events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  thread_id TEXT NOT NULL REFERENCES threads(id),
  kind TEXT NOT NULL,
  author TEXT NOT NULL,
  at BIGINT NOT NULL
);

CREATE INDEX threads_by_artifact ON threads(artifact_id);
CREATE INDEX messages_by_thread ON messages(thread_id);
CREATE INDEX events_by_artifact ON events(artifact_id, seq);
