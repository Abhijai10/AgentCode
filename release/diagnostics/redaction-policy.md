# Diagnostics Redaction Policy

Diagnostics may include platform, AgentCode version, daemon state, database version,
provider route names, local model status, LSP/tool availability, browser runtime and
disk paths.

Diagnostics must not include secret values, provider credentials, Git credentials,
cloud credentials, raw private source excerpts or unredacted security canaries.

Phase 27 production API: `ReleaseEngineer::diagnostics_report`.
