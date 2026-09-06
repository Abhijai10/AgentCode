# WP-P12-WP16 — Mailboxes

- **Phase:** P12
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** a9736c9
- **Accepted commit:** d004543
- **Objective:** Store durable agent coordination messages as data objects.
- **Implementation summary:** Added `MailboxMessage`, message types and DB mailbox rows.
- **Affected files:** `crates/ac-runtime/src/lib.rs`, `crates/ac-db/src/lib.rs`, `migrations/0007_full_autonomy_kernel.sql`
- **Tests:** `phase12_leases_recovery_progress_roles_and_replan_work`; `phase12_autonomy_state_survives_reopen`
- **Migration impact:** `autonomy_mailbox_messages`
- **Acceptance status:** ACCEPTED; P12-G14 PASS.
