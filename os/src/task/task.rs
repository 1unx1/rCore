//! Types related to task management

use super::TaskContext;

/// Info about syscall times and start time in microsecond of a task,
/// refer to hint of https://learningos.github.io/rCore-Tutorial-Guide-2023S/chapter3/5exercise.html
#[derive(Copy, Clone)]
pub struct TaskInnerInfo {
    /// Times of syscall called by task
    pub syscall_times: [u32; 5],
    /// Start running time in microsecond of task
    pub start_time_us: Option<usize>,
}

impl TaskInnerInfo {
    /// Zero initialization
    pub fn zero_init() -> Self {
        Self {
            syscall_times: [0; 5],
            // the task has not started when zero_init()
            start_time_us: Option::None,
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
    /// The task information, including syscall times and start time
    pub task_info: TaskInnerInfo,
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
