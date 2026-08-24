# AgentCode

AgentCode is a local autonomous coding system with Kernel-owned mission authority,
evidence-backed verification, brokered tools, sandboxed execution, provider routing,
context/memory separation and release-gated packaging.

## V1 Release

V1 is released only from the exact artifact and commit that pass the release candidate
and final release gates. The V1 release state is recorded in the control-plane database
with release candidates, validation runs, approval decisions, final manifests and
evidence bundles.

Shipping V1 capabilities:

- Autonomous mission flow with planning, context, provider routing, tools, sandbox,
  ChangeSet application, verification and evidence.
- Discuss Mode and Design Studio core workflows.
- Baseline security, AI/security fixtures where applicable, secret redaction and
  release-blocking security/license checks.
- Durable migration path through schema v18.

Known limitation: public signing/notarization credentials remain prerequisite-bound;
unsigned artifacts must be labeled non-public preview.
