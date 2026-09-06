# AgentCode SBOM

Schema: AgentCode release SBOM v1

Required component fields:

- `name`
- `version`
- `license`
- `source`
- `checksum`
- `security_status`
- `vulnerability_refs`
- `release_blocking`

Phase 26 production API: `SecurityHardeningReview::generate_sbom`.

Release rule: an SBOM with zero components, `UNKNOWN` license, incompatible license,
or release-blocking vulnerability cannot approve public V1 packaging.
