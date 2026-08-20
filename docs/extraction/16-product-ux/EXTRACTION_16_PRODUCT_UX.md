# WP16 Product UX Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| Dyad | `22de43cf0d16` | `e2e-tests/chat_panel_toggle.spec.ts`; `e2e-tests/chat_history.spec.ts`; `e2e-tests/app_screenshot.spec.ts`; `e2e-tests/edit_code.spec.ts` |
| Bolt.diy | `2e254ac19a69` | `app/components/chat/Artifact.tsx`; `app/components/chat/ScreenshotStateManager.tsx` |
| Continue | `5522c6f44ca0` | `core/context/providers/index.ts`; `core/context/providers/_context-providers.vitest.ts`; `core/autocomplete/templating/renderPrompt.vitest.ts` file inventory |

## Mechanisms Discovered

### Dyad UX regression coverage

Sources inspected include screenshot and edit-code tests; related UX e2e tests were identified by path.

Important symbols from inspected tests: `po.setUp`, `po.sendPrompt`, `previewPanel`, `selectPreviewMode`, `expectFileContent`, `Timeout`.

Control flow: tests drive product through page objects, generate an app, toggle preview/code modes, edit files, save, verify persistent content, and assert screenshot artifacts.

Useful pattern: product UX acceptance needs page-object flows that reflect user workflows, not isolated unit helpers.

Limitations: inspected tests focus on generated-app workflow; AgentCode product UX must cover mission/session/provider/tool/security surfaces.

### Bolt visible action/status model

Source: `app/components/chat/Artifact.tsx`.

Important symbols: `Artifact`, `ActionList`, `ActionState`, `workbenchStore`, `openArtifactInWorkbench`.

Control flow: user sees creation/restoration progress, action list status, workbench toggle, selected file navigation.

Useful pattern: long-running agent work should expose progress and actionable artifact links.

Limitations: status text is UI-local; AgentCode must derive it from Kernel events.

### Continue context-provider product model

Source inventory: `core/context/providers/index.ts`, provider tests, autocomplete prompt rendering tests.

Important symbols identified by file paths: context providers for current file, diff, terminal, problems, open files, repo map, rules, URL/web/docs, MCP, GitHub/GitLab/Jira.

Useful pattern: product UX exposes context sources as named, composable capabilities.

Limitations: context providers are not authority and may expose sensitive data.

## AgentCode Constraints

Product UX must show Kernel-owned truth, evidence provenance, permission boundaries, degraded states, and recovery actions. UI cannot invent state from transcript.
