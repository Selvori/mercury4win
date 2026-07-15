// mercury4win-linux/src-tauri/agent/runtime.rs
// Agent task queue + state machine — mirrors Swift Actor pattern
// Design: 1 active slot + 1 waiting slot per task type, latest-only replacement

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Agent task types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    Summary,
    Translation,
    Tagging,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Summary => "summary",
            TaskType::Translation => "translation",
            TaskType::Tagging => "tagging",
        }
    }
}

/// State of an agent task slot.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotState {
    Idle,
    Running { entry_id: i64 },
    RunningWithWaiting { running: i64, waiting: i64 },
}

/// The agent runtime engine — one per task type.
pub struct AgentRuntimeEngine {
    task_type: TaskType,
    state: SlotState,
}

impl AgentRuntimeEngine {
    pub fn new(task_type: TaskType) -> Self {
        Self {
            task_type,
            state: SlotState::Idle,
        }
    }

    /// Submit a task. Returns the replaced waiting entry_id, if any.
    /// Rules:
    /// - If idle → Running
    /// - If Running → RunningWithWaiting
    /// - If RunningWithWaiting → replace waiting, return old waiting
    pub fn submit(&mut self, entry_id: i64) -> Option<i64> {
        match &self.state {
            SlotState::Idle => {
                self.state = SlotState::Running { entry_id };
                None
            }
            SlotState::Running { .. } => {
                // Clone current running, transition to RunningWithWaiting
                let new_state = match self.state {
                    SlotState::Running { entry_id: running } => {
                        SlotState::RunningWithWaiting { running, waiting: entry_id }
                    }
                    _ => unreachable!(),
                };
                self.state = new_state;
                None // No old waiting to replace
            }
            SlotState::RunningWithWaiting { running, waiting } => {
                let replaced = *waiting;
                self.state = SlotState::RunningWithWaiting {
                    running: *running,
                    waiting: entry_id,
                };
                Some(replaced)
            }
        }
    }

    /// Mark running task complete. Promotes waiting → running if present.
    /// Returns completed entry_id and optionally the promoted entry_id.
    pub fn complete(&mut self) -> (Option<i64>, Option<i64>) {
        match self.state {
            SlotState::Running { entry_id } => {
                self.state = SlotState::Idle;
                (Some(entry_id), None)
            }
            SlotState::RunningWithWaiting { running, waiting } => {
                self.state = SlotState::Running { entry_id: waiting };
                (Some(running), Some(waiting))
            }
            SlotState::Idle => (None, None),
        }
    }

    pub fn state(&self) -> &SlotState {
        &self.state
    }
}

/// Global agent runtime managing all task types.
pub struct AgentRuntime {
    engines: HashMap<TaskType, AgentRuntimeEngine>,
}

impl AgentRuntime {
    pub fn new() -> Self {
        let mut engines = HashMap::new();
        for tt in &[TaskType::Summary, TaskType::Translation, TaskType::Tagging] {
            engines.insert(*tt, AgentRuntimeEngine::new(*tt));
        }
        Self { engines }
    }

    pub fn submit(&mut self, entry_id: i64, task_type: TaskType) -> Option<i64> {
        self.engines
            .get_mut(&task_type)
            .and_then(|e| e.submit(entry_id))
    }

    pub fn complete(&mut self, task_type: TaskType) -> (Option<i64>, Option<i64>) {
        self.engines
            .get_mut(&task_type)
            .map(|e| e.complete())
            .unwrap_or((None, None))
    }

    pub fn get_state(&self, task_type: TaskType) -> Option<&SlotState> {
        self.engines.get(&task_type).map(|e| e.state())
    }
}

pub type SharedAgentRuntime = Arc<Mutex<AgentRuntime>>;

pub fn create_runtime() -> SharedAgentRuntime {
    Arc::new(Mutex::new(AgentRuntime::new()))
}

// ── Agent status for IPC ──

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentStatusInfo {
    pub agent_type: String,
    pub state: String,
    pub current_entry_id: Option<i64>,
    pub queue_depth: usize,
}

impl AgentRuntime {
    pub fn get_status(&self, task_type: TaskType) -> AgentStatusInfo {
        let state = self.get_state(task_type);
        let (state_str, current_id, queue_depth) = match state {
            Some(SlotState::Idle) => ("idle", None, 0),
            Some(SlotState::Running { entry_id }) => ("running", Some(*entry_id), 0),
            Some(SlotState::RunningWithWaiting { running, .. }) => {
                ("running_waiting", Some(*running), 1)
            }
            None => ("idle", None, 0),
        };
        AgentStatusInfo {
            agent_type: task_type.as_str().to_string(),
            state: state_str.to_string(),
            current_entry_id: current_id,
            queue_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_to_running() {
        let mut engine = AgentRuntimeEngine::new(TaskType::Summary);
        assert!(engine.submit(42).is_none());
        assert_eq!(*engine.state(), SlotState::Running { entry_id: 42 });
    }

    #[test]
    fn test_running_to_running_with_waiting() {
        let mut engine = AgentRuntimeEngine::new(TaskType::Summary);
        engine.submit(42);
        assert!(engine.submit(99).is_none());
        assert_eq!(
            *engine.state(),
            SlotState::RunningWithWaiting { running: 42, waiting: 99 }
        );
    }

    #[test]
    fn test_replace_waiting() {
        let mut engine = AgentRuntimeEngine::new(TaskType::Summary);
        engine.submit(42);
        engine.submit(99);
        let replaced = engine.submit(77);
        assert_eq!(replaced, Some(99));
        assert_eq!(
            *engine.state(),
            SlotState::RunningWithWaiting { running: 42, waiting: 77 }
        );
    }

    #[test]
    fn test_complete_promotes_waiting() {
        let mut engine = AgentRuntimeEngine::new(TaskType::Summary);
        engine.submit(42);
        engine.submit(99);
        let (completed, promoted) = engine.complete();
        assert_eq!(completed, Some(42));
        assert_eq!(promoted, Some(99));
        assert_eq!(*engine.state(), SlotState::Running { entry_id: 99 });
    }

    #[test]
    fn test_complete_returns_to_idle() {
        let mut engine = AgentRuntimeEngine::new(TaskType::Summary);
        engine.submit(42);
        let (completed, promoted) = engine.complete();
        assert_eq!(completed, Some(42));
        assert_eq!(promoted, None);
        assert_eq!(*engine.state(), SlotState::Idle);
    }

    #[test]
    fn test_runtime_multiple_types() {
        let runtime = create_runtime();
        let mut rt = runtime.blocking_lock();
        rt.submit(42, TaskType::Summary);
        rt.submit(99, TaskType::Translation);

        let summary_status = rt.get_status(TaskType::Summary);
        assert_eq!(summary_status.state, "running");
        assert_eq!(summary_status.current_entry_id, Some(42));

        let trans_status = rt.get_status(TaskType::Translation);
        assert_eq!(trans_status.state, "running");
        assert_eq!(trans_status.current_entry_id, Some(99));
    }
}
