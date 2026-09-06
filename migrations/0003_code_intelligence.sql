CREATE TABLE IF NOT EXISTS code_repositories (id TEXT PRIMARY KEY, root TEXT NOT NULL, commit_ref TEXT NOT NULL, indexed_at_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS code_files (repository_id TEXT NOT NULL, path TEXT NOT NULL, content_hash TEXT NOT NULL, language TEXT NOT NULL, metadata_json TEXT NOT NULL, PRIMARY KEY(repository_id, path));
CREATE TABLE IF NOT EXISTS code_symbols (repository_id TEXT NOT NULL, path TEXT NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, line INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS code_imports (repository_id TEXT NOT NULL, source_path TEXT NOT NULL, target TEXT NOT NULL, line INTEGER NOT NULL);
