//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_current_inner_info,
        mmap, munmap, suspend_current_and_run_next, TaskStatus,
    },
    timer::get_time_us,
};
use core::{mem::size_of, ptr::copy};

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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let mut buffers =
        translated_byte_buffer(current_user_token(), _ts as *const u8, size_of::<TimeVal>());
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let src = &time_val as *const TimeVal as *const u8;
    unsafe {
        copy(src, buffers[0].as_mut_ptr(), buffers[0].len());
    }
    if buffers.len() != 1 {
        // splitted by 2 pages
        unsafe {
            copy(
                src.add(buffers[0].len()),
                buffers[1].as_mut_ptr(),
                buffers[1].len(),
            );
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let inner_info = get_current_inner_info();
    if let Some(start_time) = inner_info.1 {
        let mut buffers = translated_byte_buffer(
            current_user_token(),
            _ti as *const u8,
            size_of::<TaskInfo>(),
        );
        let task_info = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: {
                let mut syscall_times = [0; MAX_SYSCALL_NUM];
                for (syscall_id, times) in &inner_info.0 {
                    syscall_times[*syscall_id] = *times;
                }
                syscall_times
            },
            time: (get_time_us() - start_time) / 1000,
        };
        let src = &task_info as *const TaskInfo as *const u8;
        unsafe {
            copy(src, buffers[0].as_mut_ptr(), buffers[0].len());
        }
        if buffers.len() != 1 {
            // splitted by 2 pages
            unsafe {
                copy(
                    src.add(buffers[0].len()),
                    buffers[1].as_mut_ptr(),
                    buffers[1].len(),
                );
            }
        }
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if _start % PAGE_SIZE == 0 && _port & !0x7 == 0 && _port & 0x7 != 0 {
        mmap(_start, _len, _port)
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _start % PAGE_SIZE == 0 {
        munmap(_start, _len)
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
