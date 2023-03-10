//! Types related to task management

use super::TaskContext;
use alloc::collections::BTreeMap;

/// Info about syscall times and start time in microsecond of a task
#[derive(Clone)]
pub struct TaskInnerInfo {
    /// Times of syscall called by task
    pub syscall_times: BTreeMap<usize, u32>,
    /// Start running time in microsecond of task
    pub start_time_us: usize,
}

impl TaskInnerInfo {
    /// Called when a task runs for the first time
    pub fn zero_init(time_us: usize) -> Self {
        Self {
            syscall_times: BTreeMap::new(),
            start_time_us: time_us,
        }
    }
}

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
