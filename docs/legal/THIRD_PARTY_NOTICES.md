# Third-Party Notices

Phase 26 establishes the release notice format and requires every shipped dependency,
asset and managed external binary to have a known license/provenance record before
public V1 distribution.

Current release-preparation inventory is generated from workspace dependency audit
records and persisted in schema v17. No copied donor repository code is recorded as
shipped runtime material in this batch.

Batch 3 admits Tree-sitter parser crates, LSP protocol types, and serde/serde_json
for real code-intelligence parsing and LSP JSON-RPC payload handling. See
`docs/policy/dependency-admissions/DEP-ADM-002-code-intel-real-engines.md`.

Batch 4 admits tungstenite, base64 and tempfile for the real Chrome DevTools
Protocol browser backend. See
`docs/policy/dependency-admissions/DEP-ADM-003-browser-cdp-backend.md`.

Required fields per shipped component:

- Name
- Version
- License
- Source
- Checksum or package integrity record
- Notice requirement
- Distribution decision

Unknown or incompatible license status blocks public release.
