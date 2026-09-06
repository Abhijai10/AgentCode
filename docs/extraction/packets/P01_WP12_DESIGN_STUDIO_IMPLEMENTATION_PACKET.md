# P01-WP12 Design Studio Implementation Packet

## Future Interfaces

```text
DesignStudio.analyze(UIEvidenceSet) -> DesignBrief
DesignStudio.propose(DesignBrief, Constraints) -> DesignChangeSet
DesignStudio.review(before, after, gates) -> DesignReviewReport
```

## Ownership

- Design Studio owns briefs, critiques, proposals, and iteration records.
- Edit Engine owns applying ChangeSets.
- Browser/QA owns screenshots and flow verification.
- Kernel owns approval and task truth.

## State and Data

Persist `DesignBrief`, `DesignGrammar`, component map, constraints, screenshots, critique, ChangeSet ids, and verification results. Generated design docs are derived state.

## Integration Points

Code Intelligence for component discovery; Browser/QA for screenshots; Tools/Edit Engine for patches; Evidence Store for visual artifacts.

## Security Constraints

No direct mutation; no unauthorized pixel/asset copying; redact screenshots; reference material requires provenance/license status.

## Unresolved Questions

What minimum accessibility matrix is REQUIRED_V1? How are design tokens represented? Which visual diff threshold is acceptable?
