use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ac_common::{AcError, AcResult, StableId, TimestampMillis};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSessionState {
    Created,
    Running,
    Cancelling,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEventKind {
    SessionCreated,
    SessionStarted,
    WorkItemProcessed(String),
    CancellationRequested,
    SessionStopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEvent {
    pub id: StableId,
    pub session_id: StableId,
    pub kind: RuntimeEventKind,
    pub created_at: TimestampMillis,
}

#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Worker {
    pub id: StableId,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            id: StableId::new("worker"),
        }
    }
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AgentSession {
    id: StableId,
    worker: Worker,
    state: AgentSessionState,
    token: CancellationToken,
    queue: VecDeque<String>,
    events: Vec<RuntimeEvent>,
}

impl AgentSession {
    pub fn new(worker: Worker) -> Self {
        let id = StableId::new("session");
        let mut session = Self {
            id,
            worker,
            state: AgentSessionState::Created,
            token: CancellationToken::new(),
            queue: VecDeque::new(),
            events: Vec::new(),
        };
        session.record(RuntimeEventKind::SessionCreated);
        session
    }

    pub fn id(&self) -> &StableId {
        &self.id
    }

    pub fn worker(&self) -> &Worker {
        &self.worker
    }

    pub fn state(&self) -> AgentSessionState {
        self.state
    }

    pub fn enqueue(&mut self, work_item: impl Into<String>) -> AcResult<()> {
        if self.state == AgentSessionState::Stopped {
            return Err(AcError::conflict(
                "RUNTIME-SESSION_STOPPED",
                "cannot enqueue work into a stopped session",
            ));
        }
        let item = work_item.into();
        if item.trim().is_empty() {
            return Err(AcError::validation(
                "RUNTIME-EMPTY_WORK_ITEM",
                "work item cannot be empty",
            ));
        }
        self.queue.push_back(item);
        Ok(())
    }

    pub fn run_until_idle(&mut self) -> AcResult<()> {
        if self.state == AgentSessionState::Created {
            self.state = AgentSessionState::Running;
            self.record(RuntimeEventKind::SessionStarted);
        }
        if self.state != AgentSessionState::Running {
            return Err(AcError::conflict(
                "RUNTIME-SESSION_NOT_RUNNING",
                "session cannot process work in current state",
            ));
        }
        while let Some(item) = self.queue.pop_front() {
            if self.token.is_cancelled() {
                self.state = AgentSessionState::Cancelling;
                self.record(RuntimeEventKind::CancellationRequested);
                break;
            }
            self.record(RuntimeEventKind::WorkItemProcessed(item));
        }
        Ok(())
    }

    pub fn request_cancel(&self) {
        self.token.cancel();
    }

    pub fn stop(&mut self) -> AcResult<()> {
        match self.state {
            AgentSessionState::Created
            | AgentSessionState::Running
            | AgentSessionState::Cancelling => {
                self.state = AgentSessionState::Stopped;
                self.record(RuntimeEventKind::SessionStopped);
                Ok(())
            }
            AgentSessionState::Stopped => Err(AcError::conflict(
                "RUNTIME-SESSION_ALREADY_STOPPED",
                "session is already stopped",
            )),
        }
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    fn record(&mut self, kind: RuntimeEventKind) {
        self.events.push(RuntimeEvent {
            id: StableId::new("re"),
            session_id: self.id.clone(),
            kind,
            created_at: TimestampMillis::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_processes_events_without_owning_kernel_state() {
        let mut session = AgentSession::new(Worker::new());
        session.enqueue("model-turn-placeholder").unwrap();
        session.run_until_idle().unwrap();
        assert_eq!(session.state(), AgentSessionState::Running);
        assert!(session
            .events()
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::WorkItemProcessed(_))));
    }

    #[test]
    fn cancellation_stops_at_session_boundary() {
        let mut session = AgentSession::new(Worker::new());
        session.enqueue("first").unwrap();
        session.request_cancel();
        session.run_until_idle().unwrap();
        assert_eq!(session.state(), AgentSessionState::Cancelling);
    }
}
