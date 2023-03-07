//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_TASK_INFO, SYSCALL_WRITE, SYSCALL_YIELD},
    task::{exit_current_and_run_next, get_current_info, suspend_current_and_run_next, TaskStatus},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let task_info = get_current_info();
    if let Some(start_time) = task_info.start_time_us {
        unsafe {
            *_ti = TaskInfo {
                // current task must be 'Running'
                status: TaskStatus::Running,
                syscall_times: {
                    let mut syscall_info: [u32; MAX_SYSCALL_NUM] = [0; MAX_SYSCALL_NUM];
                    syscall_info[SYSCALL_WRITE] = task_info.syscall_times[0];
                    syscall_info[SYSCALL_EXIT] = task_info.syscall_times[1];
                    syscall_info[SYSCALL_YIELD] = task_info.syscall_times[2];
                    syscall_info[SYSCALL_GET_TIME] = task_info.syscall_times[3];
                    syscall_info[SYSCALL_TASK_INFO] = task_info.syscall_times[4];
                    syscall_info
                },
                time: (get_time_us() - start_time) / 1000,
            };
        }
        0
    } else {
        -1
    }
}
