//! Types related to task management

use crate::timer::get_time_us;

use super::TaskContext;
use alloc::collections::BTreeMap;

/// Info about syscall times and start time in usec of a task
#[derive(Clone)]
pub struct TaskInnerInfo {
    /// Times of syscall called by task
    pub syscall_times: BTreeMap<usize, u32>,
    /// Start running time in usec of task
    pub start_time_us: Option<usize>,
}

impl TaskInnerInfo {
    /// Initialize `start_time_us` with `None`,
    /// which means the task hasn't started
    pub fn zero_init() -> Self {
        Self {
            syscall_times: BTreeMap::new(),
            start_time_us: None,
        }
    }
    /// Save start time in usec
    pub fn save_start_time_us(&mut self) {
        self.start_time_us = Some(get_time_us());
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
