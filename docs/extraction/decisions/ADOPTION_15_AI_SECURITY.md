# WP15 AI Security Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| promptfoo provider call queue/rate limits | TAKE | AI security evals need bounded concurrency and provider budgets. |
| promptfoo assertions/grading | ADAPT | Normalize assertions as `AISecurityCheck` with grader provenance. |
| promptfoo redteam plugins/strategies | ADAPT | Use controlled plugin registry; generated cases are untrusted evidence. |
| garak probe/detector/evaluator split | TAKE | Clean separation for attack generation, detection, and scoring. |
| garak plugin cache report entry | TAKE | Persist tested plugin/version metadata. |
| garak modality matching | TAKE | Required for text/image/audio/browser targets. |
| PyRIT converters | TAKE | Use typed prompt mutation/conversion with modality requirements. |
| PyRIT target-bound converters | ADAPT | Validate target requirements against AgentCode provider/tool capabilities. |
| AI eval PASS as proof of safety | REJECT | Kernel treats as bounded evidence only. |
| Unvetted attack plugins | REJECT | Plugins require admission, sandboxing, and provenance. |

Security implications: redteam prompts may be harmful; isolate storage, prevent automatic execution against production systems, and redact sensitive outputs.
