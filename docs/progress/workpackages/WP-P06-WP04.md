# WP-P06-WP04 — Worker Tool Exposure

- **Phase:** P06
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Owner modules:** `crates/ac-agent`, `crates/ac-tool`

Workers request repository, filesystem, and test tools through Tool Broker; Sandbox controls execution and Evidence Store captures results.
