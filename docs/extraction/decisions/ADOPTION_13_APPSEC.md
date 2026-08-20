# WP13 AppSec Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| Semgrep rule/target scan pipeline | WRAP | Run through Tool Broker; normalize findings into SecurityFinding. |
| Semgrep finding ids/hashes/git refs | TAKE | Required for dedupe, stale checks, and evidence links. |
| Semgrep autofix path | REJECT | AppSec may propose remediation; Edit Engine applies. |
| Gitleaks git/directory/stdin modes | TAKE | Support repo, worktree, and staged-content secret scans. |
| Gitleaks symlink following | REJECT | Off by default under sandbox policy. |
| Trivy target kinds | TAKE | Normalize fs/repo/sbom/image scans under a typed target model. |
| Trivy DB/module updates | WRAP | Must be explicit managed-tool setup with cached provenance. |
| ZAP active scans | WRAP | Require explicit scope/approval; never default autonomous action. |
| Scanner PASS as Kernel truth | REJECT | Kernel decides from evidence and policy gates. |

Security implications: scanner output may contain secrets/vulnerable code. Evidence storage requires redaction, retention, and access controls.
