//! Durable orphan-implementation guard (Doc 06 audit: "uncalled
//! implementations" are forbidden; §6 No Fake Implementations).
//!
//! The ac-agent `DiscussMode` (ac-agent/src/discuss.rs) and `DesignStudio`
//! (ac-agent/src/design.rs) in-memory mode implementations were orphaned
//! parallel models — never called by the daemon — duplicating the
//! conversation-persisted production paths in ac-daemon
//! (conversation.rs / discuss_plan.rs / design.rs).  They were deleted and
//! their behavior consolidated into the daemon.
//!
//! This test guards the consolidation durably: if anyone reintroduces the
//! orphan files or a mode entry point on ac-agent, this fails with a clear
//! message pointing at the daemon paths that own the behavior.  A
//! source-tree scan is a supplement: the primary guard is that the
//! production paths live in ac-daemon and are exercised by the
//! daemon/integration tests; a regression would have to re-add dead
//! public API to ac-agent to pass its own callers, which this detects.

use std::path::Path;

#[test]
fn no_orphan_mode_implementations_reappear_in_ac_agent() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let agent_src = manifest.join("../../crates/ac-agent/src");
    assert!(
        agent_src.is_dir(),
        "ac-agent/src must exist at {}",
        agent_src.display()
    );

    // The orphan files must stay deleted.
    for orphan in ["discuss.rs", "design.rs"] {
        let path = agent_src.join(orphan);
        assert!(
            !path.is_file(),
            "orphan reappeared: {}. Discuss/Design production behavior lives in \
             ac-daemon (crates/ac-daemon/src/conversation.rs, discuss_plan.rs, \
             design.rs) and is persisted via SQLite; do not reintroduce a \
             parallel in-memory model in ac-agent.",
            path.display()
        );
    }

    // No include! may wire an orphan mode file back in.
    let lib = std::fs::read_to_string(agent_src.join("lib.rs"))
        .expect("ac-agent/src/lib.rs must be readable");
    for orphan in ["include!(\"discuss.rs\")", "include!(\"design.rs\")"] {
        assert!(
            !lib.contains(orphan),
            "lib.rs re-includes the orphan mode file ({orphan}); the daemon \
             owns Discuss/Design production paths."
        );
    }
    // The orphan mode entry points must not be re-declared anywhere in
    // ac-agent sources.
    for entry in walk(&agent_src) {
        let src = match std::fs::read_to_string(&entry) {
            Ok(src) => src,
            Err(_) => continue,
        };
        for banned in [
            "pub struct DiscussMode",
            "pub struct DesignStudio",
            "impl DiscussMode",
            "impl DesignStudio",
        ] {
            assert!(
                !src.contains(banned),
                "orphan mode implementation reappeared in {}: contains '{banned}'. \
                 Discuss/Design behavior is owned by ac-daemon; consolidate there.",
                entry.display()
            );
        }
    }
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    out
}
