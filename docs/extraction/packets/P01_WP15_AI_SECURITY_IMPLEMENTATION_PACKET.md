# P01-WP15 AI Security Implementation Packet

## Future Interfaces

```text
AISecurityRunner.run(EvalPlan) -> AISecurityReport
AttackCaseGenerator.generate(policy, scope) -> AttackCaseSet
AIScorer.score(responses, rubrics) -> ScoreReport
```

## Ownership

AI Security owns eval plans, attack cases, scoring, and reports. Provider Fabric owns model calls. Kernel owns acceptance/block decisions.

## State/Data

Persist model id, provider config hash, prompts, generated attack cases, responses, grader id, scores, token usage, seeds, and stale conditions.

## Integration Points

Provider Fabric, Tool Broker, Evidence Store, Security policy, Verification Engine.

## Security Constraints

Generated prompts are untrusted; no production targets without explicit scope; bounded concurrency/cost; scorer outputs are evidence only.

## Unresolved Questions

Which AI risk categories are REQUIRED_V1? What repeat count/seed policy is enough? Which models can grade security results?
