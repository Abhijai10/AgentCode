# AgentCode Security Policy

AgentCode is treated as a privileged local developer tool. Reports should include
affected version, platform, reproduction steps, expected boundary, observed behavior,
and whether source code, credentials, repositories, MCP tools, skills, browser state,
managed binaries, updates, logs or evidence were exposed.

Release-blocking classes:

- Workspace escape or unauthorized repository mutation.
- Secret exposure in model payloads, logs, reports, diagnostics, evidence or UI.
- Tool Broker, sandbox, MCP, Skill or Hook capability bypass.
- Unknown or incompatible shipped dependency license.
- Unverified release artifact or managed binary checksum.

Accepted V1 limitation: public signing/notarization credentials are prerequisite
dependent; unsigned/not-notarized artifacts must be marked as non-public preview.
