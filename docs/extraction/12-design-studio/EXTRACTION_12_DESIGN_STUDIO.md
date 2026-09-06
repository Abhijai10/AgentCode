# WP12 Design Studio Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| Onlook | `423e2e924366` | `packages/parser/src/code-edit/index.ts`; `packages/parser/src/code-edit/style.ts`; `packages/code-provider/src/types.ts`; `packages/code-provider/src/providers.ts` |
| Bolt.diy | `2e254ac19a69` | `app/components/chat/Artifact.tsx`; `app/components/chat/ScreenshotStateManager.tsx` |
| Dyad | `22de43cf0d16` | `e2e-tests/app_screenshot.spec.ts`; `e2e-tests/edit_code.spec.ts` |

## Mechanisms Discovered

### Onlook AST-backed design edits

Sources: `packages/parser/src/code-edit/index.ts`; `style.ts`.

Important symbols: exports `group`, `image`, `insert`, `layout`, `move`, `remove`, `style`, `text`, `transform`; functions `addClassToNode`, `replaceNodeClasses`, `updateNodeProp`, `customTwMerge`.

Control flow: code-edit package exposes focused AST transform modules. Style helpers locate JSX `className`, merge or replace Tailwind-style class strings, insert missing attributes, and update JSX props by primitive type.

Failure behavior: unsupported/undefined prop values return without mutation; non-string class expressions only support call-expression appends.

Useful pattern: design changes become structured code-edit operations.

Limitations: direct mutation of AST nodes must be mediated by AgentCode ChangeSet and verification.

### Onlook provider boundary

Sources: `packages/code-provider/src/types.ts`; `providers.ts`.

Important symbols: `Provider`, `WriteFileInput`, `ReadFileInput`, `WatchFilesInput`, `ProviderTerminal`, `GitStatusOutput`, `CodeProvider`.

Control flow: provider interface exposes JSON-serializable file/terminal/git/session operations across sandbox providers such as CodeSandbox, E2B, Daytona, VercelSandbox, Modal, NodeFs.

Failure behavior: interface shape lacks detailed error taxonomy; AgentCode must add unavailable/degraded/error states.

Useful pattern: design tooling should target provider-neutral file/watch/terminal contracts.

Limitations: providers include direct write operations; AgentCode Design Studio must propose ChangeSets, not write directly.

### Bolt artifact/workbench feedback

Sources: `app/components/chat/Artifact.tsx`; `ScreenshotStateManager.tsx`.

Important symbols: `Artifact`, `ActionList`, `openArtifactInWorkbench`, `workbenchStore`, `ActionState`, `ScreenshotStateManager`.

Control flow: artifact UI reads workbench artifacts/actions, displays running/complete status, opens workbench/code view, and exposes screenshot upload state to browser globals for chat-to-image workflows.

Failure behavior: UI status depends on action status; screenshot globals are cleaned up on unmount.

Useful pattern: design iteration needs visible action state and screenshot inputs.

Limitations: browser globals for screenshot state are too ad hoc for AgentCode; use Evidence Store ids.

### Dyad visual/e2e verification

Sources: `e2e-tests/app_screenshot.spec.ts`; `edit_code.spec.ts`.

Important symbols: `expect.toPass`, `.dyad/screenshot`, `selectPreviewMode("code")`, `replaceEditorContent`, `expectFileContent`.

Control flow: design output is verified by screenshot generation, visible preview image, code-mode edits, save action, and filesystem polling.

Failure behavior: rapid-switch test catches stale editor selection.

Useful pattern: Design Studio iterations require screenshot plus functional preservation.

Limitations: no automatic accessibility or layout critique in inspected files.

## AgentCode Constraints

Design Studio provides understanding and proposals. Flow must be `Design understanding -> Component proposal -> ChangeSet -> Verification -> Kernel approval`. It cannot mutate application state directly.
