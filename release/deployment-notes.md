# Deployment Notes

V1 deployment must use the exact artifact hash recorded in the final release manifest.
A rebuild after approval is a different artifact and must re-enter the RC gate.

Before publication:

- Verify schema v18 migration on fresh install and upgrade.
- Verify artifact checksum and provenance.
- Verify SBOM, notices, limitations and security policy.
- Verify rollback and incident-response references exist in the evidence bundle.
