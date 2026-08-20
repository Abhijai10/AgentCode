# WP14 Cloud Security Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| Checkov runner registry | ADAPT | Use typed scanner capability registry for IaC/static cloud artifacts. |
| Checkov custom policies | WRAP | Custom policy source must be admitted and hashed. |
| Checkov platform API coupling | REJECT | Cloud SaaS integration cannot be assumed for AgentCode V1. |
| Prowler provider/check filters | TAKE | Cloud scans need provider/scope/check/severity/compliance filters. |
| Prowler credential printing | REJECT | Never expose credential details to model/UI without redaction policy. |
| Prowler compliance outputs | ADAPT | Store compliance mapping as report metadata. |
| Trivy target kinds | TAKE | Reuse artifact target model for repo/fs/k8s/sbom. |
| Autonomous broad cloud scan | REJECT | Requires explicit scope, credentials, rate limits, and approval. |

Security implications: cloud scans touch live accounts and credentials; isolate credentials and record least-privilege scope.
