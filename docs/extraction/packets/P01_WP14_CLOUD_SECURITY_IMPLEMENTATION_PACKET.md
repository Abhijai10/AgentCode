# P01-WP14 Cloud Security Implementation Packet

## Future Interfaces

```text
CloudSecurityScanner.run(CloudScanRequest) -> CloudScanReport
CloudCredentialBroker.issue(scope, purpose) -> CredentialLease
CloudFindingStore.upsert(report) -> FindingReceipt
```

## Ownership

Credential Broker owns secrets. Scanner Adapter owns invocation. Evidence Store owns raw outputs. Kernel owns scope and pass/fail decisions.

## State/Data

Persist provider, account/project scope, scanner version, policy hash, credential lease id, findings, compliance mappings, and stale marker.

## Integration Points

Secrets isolation, Tool Broker, AppSec normalization, Evidence Store, Kernel gate evaluation.

## Security Constraints

No ambient credentials; no broad scans by default; no write/remediate operations; redact account ids where required; rate-limit and timeout all cloud calls.

## Unresolved Questions

Which providers are REQUIRED_V1? Are live cloud scans V1 or only IaC/static scans? What credential lease lifetime is acceptable?
