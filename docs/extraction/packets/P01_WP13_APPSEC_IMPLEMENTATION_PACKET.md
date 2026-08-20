# P01-WP13 AppSec Implementation Packet

## Future Interfaces

```text
SecurityScanner.run(SecurityScanRequest) -> SecurityScanReport
SecurityFindingStore.upsert(findings, provenance) -> FindingReceipt
SecurityPolicy.evaluate(report, policy) -> SecurityGateResult
```

## Ownership

Scanner Adapter owns invocation/normalization. Evidence Store owns raw reports. Kernel owns acceptance/block decisions. Edit Engine owns remediation.

## State/Data

Persist scan target, scanner version, config/rule hash, commit/worktree, findings, severity, fingerprint, evidence refs, stale state.

## Integration Points

Tool Broker, Sandbox, Code Intelligence, Git/Worktree, Evidence Store, Verification Engine.

## Security Constraints

No uncontrolled active scans; no silent network DB updates; redact secrets; disable symlink traversal; custom rules are untrusted until admitted.

## Unresolved Questions

Which scanners are REQUIRED_V1? What severity blocks acceptance by default? How are false positives/suppressions governed?
