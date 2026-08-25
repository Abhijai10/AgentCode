# DEP-ADM-020 — desktop UI shell dependencies

Need:
  Tauri 2 desktop shell and React/Vite TypeScript frontend for the standalone AgentCode desktop UI shell.

Proposed dependency:
  `tauri`/`tauri-build` 2.x from crates.io; `@tauri-apps/api`/`@tauri-apps/cli` 2.x,
  React 18.3.1, TypeScript 5.6.3, Vite 6.0.3, React Vite plugin 4.3.4 and ESLint 9.17.0
  from npm. Exact resolved versions are pinned in their lockfiles.

Existing alternative check:
  The repository contains only `apps/desktop-placeholder`, a Rust executable without
  a UI runtime. It cannot implement the requested Tauri/React shell.

License:
  Tauri, React, TypeScript, Vite, and ESLint are MIT or Apache-2.0/MIT licensed.
  License-file inspection and third-party manifest update remain required before a
  release admission is accepted.

Security status:
  Registry lockfiles are required. No network clients, backend IPC, or provider
  SDKs are introduced by this shell.

Maintenance state:
  Established upstream projects with active release maintenance.

Runtime/bundle cost:
  Tauri host plus a small React/Vite renderer. No large UI framework, Monaco, xterm,
  or runtime icon package is added.

Install/postinstall behavior:
  Standard Cargo/npm dependency resolution and build scripts only.

Why existing components are insufficient:
  There is no web renderer, desktop window integration, TypeScript compiler, or UI
  component layer in the current placeholder.

License gate result: PENDING
Admission decision: DEFER (shell implementation only; complete legal review before release)
Reviewer + date + evidence refs: UI-shell implementation batch, 2026-08-25.
