# WP12 Design Studio Adoption Decision

| Mechanism | Classification | AgentCode decision |
|---|---:|---|
| AST-backed JSX class/prop edits | ADAPT | Design Studio emits structured ChangeSet proposals; Edit Engine applies after Kernel approval. |
| Tailwind class merge | TAKE | Use conflict-aware class merging in design transforms. |
| Provider-neutral file/watch contract | ADAPT | Keep provider abstraction but route writes/commands through Tool Broker and ChangeSet journal. |
| Direct provider file writes from design layer | REJECT | Violates Design Studio suggestion-only constraint. |
| Artifact action status UI | TAKE | Persist design iteration status, action list, and verification state. |
| Screenshot globals | REJECT | Replace with Evidence Store attachments and typed design inputs. |
| Screenshot/e2e feedback loop | TAKE | Every visual iteration needs screenshot evidence plus functional regression checks. |
| Visual praise as success | REJECT | Accessibility/function/regression gates outrank subjective critique. |

Security implications: design references and screenshots may include proprietary UI/secrets. Store with evidence policy, not prompt-only state.
