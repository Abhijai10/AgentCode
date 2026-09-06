-- AgentCode workspace binding: persist the project workspace root selected by
-- the user with each mission session.  The daemon must execute against the
-- project the user opened, not its own cwd/AGENTCODE_WORKSPACE_ROOT.  Recovery
-- reuses this persisted root so a restarted daemon resumes in the SAME project.

ALTER TABLE agent_sessions ADD COLUMN workspace_root TEXT;
