//! One executing request and one latest waiting request per agent kind.
//! A permit covers routing and the entire executor. Dropping the executor
//! (including every buffered translation request) precedes slot promotion.
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    Summary,
    Translation,
    Tagging,
    Topic,
    SearchFilter,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Translation => "translation",
            Self::Tagging => "tagging",
            Self::Topic => "topic",
            Self::SearchFilter => "search_filter",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "summary" => Ok(Self::Summary),
            "translation" => Ok(Self::Translation),
            "tagging" => Ok(Self::Tagging),
            "topic" => Ok(Self::Topic),
            "search_filter" => Ok(Self::SearchFilter),
            _ => Err("Unknown task type".into()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PermitState {
    Waiting,
    Running,
    Cancelled(&'static str),
}

struct Request {
    id: String,
    entry_id: i64,
    signal: watch::Sender<PermitState>,
    fence: CommitFence,
}

/// SQLite interact jobs can outlive their async owner. Only this fence decides
/// whether a transaction may commit; cancellation and commit are linearized.
#[derive(Clone)]
pub struct CommitFence(Arc<Mutex<bool>>);
impl CommitFence {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(true)))
    }
    pub fn invalidate(&self) {
        *self.0.lock().unwrap() = false;
    }
    pub fn commit(&self, tx: rusqlite::Transaction<'_>) -> Result<(), String> {
        let valid = self.0.lock().unwrap();
        if !*valid {
            return Err("Agent task cancelled: persistence fenced".into());
        }
        tx.commit().map_err(|e| e.to_string())
    }
}
tokio::task_local! { static COMMIT_FENCE: CommitFence; }
pub fn current_commit_fence() -> CommitFence {
    COMMIT_FENCE
        .try_with(Clone::clone)
        .unwrap_or_else(|_| CommitFence::new())
}

#[derive(Default)]
struct Slots {
    active: Option<Request>,
    waiting: Option<Request>,
}

#[derive(Default)]
pub struct AgentRuntime {
    slots: HashMap<TaskType, Slots>,
    // Immediate unmount may dispatch cancellation before submit reaches Rust.
    abandoned: HashMap<(TaskType, String), Instant>,
}

pub type SharedAgentRuntime = Arc<Mutex<AgentRuntime>>;
pub fn create_runtime() -> SharedAgentRuntime {
    Arc::new(Mutex::new(AgentRuntime::default()))
}

impl AgentRuntime {
    fn submit(
        &mut self,
        task_type: TaskType,
        entry_id: i64,
        request_id: &str,
    ) -> Result<watch::Receiver<PermitState>, String> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("Invalid agent request ID".into());
        }
        self.abandoned
            .retain(|_, at| at.elapsed() < Duration::from_secs(300));
        if self
            .abandoned
            .remove(&(task_type, request_id.into()))
            .is_some()
        {
            return Err("Agent task cancelled: abandoned before registration".into());
        }
        let slots = self.slots.entry(task_type).or_default();
        if [&slots.active, &slots.waiting]
            .iter()
            .any(|r| r.as_ref().is_some_and(|r| r.id == request_id))
        {
            return Err("Duplicate agent request ID".into());
        }
        let state = if slots.active.is_none() {
            PermitState::Running
        } else {
            PermitState::Waiting
        };
        let (signal, receiver) = watch::channel(state);
        let request = Request {
            id: request_id.into(),
            entry_id,
            signal,
            fence: CommitFence::new(),
        };
        if state == PermitState::Running {
            slots.active = Some(request);
        } else if let Some(old) = slots.waiting.replace(request) {
            old.signal
                .send_replace(PermitState::Cancelled("replaced_by_latest"));
        }
        Ok(receiver)
    }

    /// waiting_only is used on article/panel leave and never aborts active work.
    pub fn cancel(
        &mut self,
        task_type: TaskType,
        request_id: &str,
        waiting_only: bool,
    ) -> Result<(), String> {
        if request_id.is_empty() || request_id.len() > 128 {
            return Err("Invalid agent request ID".into());
        }
        let slots = self.slots.entry(task_type).or_default();
        if let Some(active) = slots.active.as_ref().filter(|r| r.id == request_id) {
            if !waiting_only {
                active.fence.invalidate();
                // Occupied until execute() drops its actual work.
                active
                    .signal
                    .send_replace(PermitState::Cancelled("user_cancelled"));
            }
            return Ok(());
        }
        if slots.waiting.as_ref().is_some_and(|r| r.id == request_id) {
            let waiting = slots.waiting.take().unwrap();
            waiting
                .signal
                .send_replace(PermitState::Cancelled(if waiting_only {
                    "abandoned_by_entry_switch"
                } else {
                    "user_cancelled"
                }));
        } else {
            self.abandoned
                .retain(|_, at| at.elapsed() < Duration::from_secs(300));
            self.abandoned
                .insert((task_type, request_id.into()), Instant::now());
        }
        Ok(())
    }

    fn finish(&mut self, task_type: TaskType, request_id: &str) {
        let slots = self.slots.entry(task_type).or_default();
        if slots.active.as_ref().is_some_and(|r| r.id == request_id) {
            slots.active.as_ref().unwrap().fence.invalidate();
            slots.active = None;
            if let Some(next) = slots.waiting.take() {
                next.signal.send_replace(PermitState::Running);
                slots.active = Some(next);
            }
        } else if slots.waiting.as_ref().is_some_and(|r| r.id == request_id) {
            slots.waiting = None;
        }
        // Stale completion of replaced waiting work cannot release active work.
    }

    pub fn get_status(&self, task_type: TaskType) -> crate::db::models::AgentStatus {
        let slots = self.slots.get(&task_type);
        let active = slots.and_then(|s| s.active.as_ref());
        let waiting = slots.and_then(|s| s.waiting.as_ref());
        let state = match active {
            None => "idle",
            Some(r) if matches!(*r.signal.borrow(), PermitState::Cancelled(_)) => "cancelling",
            Some(_) if waiting.is_some() => "running_waiting",
            Some(_) => "running",
        };
        crate::db::models::AgentStatus {
            agent_type: task_type.as_str().into(),
            state: state.into(),
            current_entry_id: active.map(|r| r.entry_id),
            current_request_id: active.map(|r| r.id.clone()),
            waiting_entry_id: waiting.map(|r| r.entry_id),
            waiting_request_id: waiting.map(|r| r.id.clone()),
            queue_depth: usize::from(waiting.is_some()),
        }
    }
}

// Short synchronous locks allow RAII release on external future drop or panic.
// No lock spans an HTTP/SQLite await.
struct PermitGuard<'a> {
    runtime: &'a SharedAgentRuntime,
    task_type: TaskType,
    request_id: &'a str,
}
impl Drop for PermitGuard<'_> {
    fn drop(&mut self) {
        self.runtime
            .lock()
            .unwrap()
            .finish(self.task_type, self.request_id);
    }
}

fn event(channel: &Channel<String>, request_id: &str, state: &str, reason: Option<&str>) {
    let _ = channel.send(serde_json::json!({ "type": "task_state", "request_id": request_id, "state": state, "reason": reason }).to_string());
}

pub async fn execute<T, F, Fut>(
    runtime: &SharedAgentRuntime,
    task_type: TaskType,
    entry_id: i64,
    request_id: &str,
    channel: &Channel<String>,
    work: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let mut signal = runtime
        .lock()
        .unwrap()
        .submit(task_type, entry_id, request_id)?;
    let guard = PermitGuard {
        runtime,
        task_type,
        request_id,
    };
    if *signal.borrow() == PermitState::Waiting {
        event(channel, request_id, "waiting", None);
    }
    loop {
        let state = *signal.borrow_and_update();
        match state {
            PermitState::Running => break,
            PermitState::Cancelled(reason) => {
                event(channel, request_id, "cancelled", Some(reason));
                return Err(format!("Agent task cancelled: {reason}"));
            }
            PermitState::Waiting => {
                signal.changed().await.map_err(|_| "Agent runtime closed")?;
            }
        }
    }
    event(channel, request_id, "running", None);
    let fence = runtime
        .lock()
        .unwrap()
        .slots
        .get(&task_type)
        .unwrap()
        .active
        .as_ref()
        .unwrap()
        .fence
        .clone();
    // Scoped future drops all HTTP/segment work before releasing the slot.
    let result = {
        let future = COMMIT_FENCE.scope(fence, work());
        tokio::pin!(future);
        tokio::select! {
            biased;
            _ = signal.changed() => {
                let reason = match *signal.borrow() { PermitState::Cancelled(reason) => reason, _ => "runtime_closed" };
                event(channel, request_id, "cancelled", Some(reason));
                Err(format!("Agent task cancelled: {reason}"))
            }
            result = &mut future => result,
        }
    };
    drop(guard);
    result
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
