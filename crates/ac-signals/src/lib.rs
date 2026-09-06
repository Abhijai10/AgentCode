//! Graceful-shutdown signal registration (F7 of the final audit).
//!
//! The workspace forbids `unsafe` everywhere; this tiny crate is the single
//! sanctioned home for the one unavoidable unsafe operation in AgentCode:
//! registering signal(2) handlers for SIGTERM/SIGINT so the daemon can stop
//! cleanly (reap children, remove its socket) instead of dying mid-request.
//!
//! Scope: exactly two registrations per process.  The handler we install
//! only performs an atomic store — async-signal-safe by construction.

use std::sync::atomic::{AtomicBool, Ordering};

static REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn mark_shutdown(_signal: i32) {
    // Async-signal-safe: a single atomic store, nothing else.
    REQUESTED.store(true, Ordering::SeqCst);
}

/// Register SIGTERM + SIGINT handlers that set the shutdown flag.
/// Idempotent; safe to call once at daemon startup.
pub fn install_shutdown_handler() {
    // SAFETY: `libc::signal` registers a handler; the handler pointer is a
    // plain extern "C" fn whose body is async-signal-safe (atomic store
    // only).  `signal` itself is thread-safe with respect to handler
    // registration at startup before worker threads exist.
    unsafe {
        libc::signal(
            libc::SIGTERM,
            mark_shutdown as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            mark_shutdown as *const () as libc::sighandler_t,
        );
    }
}

/// Whether SIGTERM or SIGINT has been received.
pub fn shutdown_requested() -> bool {
    REQUESTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    #[test]
    fn flag_starts_clear_and_reads() {
        assert!(!super::shutdown_requested());
    }
}
