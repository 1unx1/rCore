//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_byte_buffer, translated_refmut, translated_str},
    syscall::{
        SYSCALL_CLOSE, SYSCALL_EXEC, SYSCALL_EXIT, SYSCALL_FORK, SYSCALL_FSTAT, SYSCALL_GETPID,
        SYSCALL_GET_TIME, SYSCALL_LINKAT, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_OPEN, SYSCALL_READ,
        SYSCALL_SBRK, SYSCALL_SET_PRIORITY, SYSCALL_SPAWN, SYSCALL_TASK_INFO, SYSCALL_UNLINKAT,
        SYSCALL_WAITPID, SYSCALL_WRITE, SYSCALL_YIELD,
    },
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        get_current_inner_info, mmap, munmap, suspend_current_and_run_next, TaskStatus,
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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
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
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task().unwrap().pid.0
    );
    let inner_info = get_current_inner_info();
    if let Some(start_time) = inner_info.start_time_us {
        let mut buffers = translated_byte_buffer(
            current_user_token(),
            _ti as *const u8,
            size_of::<TaskInfo>(),
        );
        let task_info = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: {
                let mut syscall_times = [0; MAX_SYSCALL_NUM];
                syscall_times[SYSCALL_UNLINKAT] = inner_info.syscall_times[0];
                syscall_times[SYSCALL_LINKAT] = inner_info.syscall_times[1];
                syscall_times[SYSCALL_OPEN] = inner_info.syscall_times[2];
                syscall_times[SYSCALL_CLOSE] = inner_info.syscall_times[3];
                syscall_times[SYSCALL_READ] = inner_info.syscall_times[4];
                syscall_times[SYSCALL_WRITE] = inner_info.syscall_times[5];
                syscall_times[SYSCALL_FSTAT] = inner_info.syscall_times[6];
                syscall_times[SYSCALL_EXIT] = inner_info.syscall_times[7];
                syscall_times[SYSCALL_YIELD] = inner_info.syscall_times[8];
                syscall_times[SYSCALL_SET_PRIORITY] = inner_info.syscall_times[9];
                syscall_times[SYSCALL_GET_TIME] = inner_info.syscall_times[10];
                syscall_times[SYSCALL_GETPID] = inner_info.syscall_times[11];
                syscall_times[SYSCALL_SBRK] = inner_info.syscall_times[12];
                syscall_times[SYSCALL_MUNMAP] = inner_info.syscall_times[13];
                syscall_times[SYSCALL_FORK] = inner_info.syscall_times[14];
                syscall_times[SYSCALL_EXEC] = inner_info.syscall_times[15];
                syscall_times[SYSCALL_MMAP] = inner_info.syscall_times[16];
                syscall_times[SYSCALL_WAITPID] = inner_info.syscall_times[17];
                syscall_times[SYSCALL_SPAWN] = inner_info.syscall_times[18];
                syscall_times[SYSCALL_TASK_INFO] = inner_info.syscall_times[19];
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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    mmap(_start, _len, _port)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    munmap(_start, _len)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(all_data.as_slice());
        let new_pid = new_task.pid.0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if _prio < 2 {
        return -1;
    }
    current_task().unwrap().inner_exclusive_access().priority = _prio;
    _prio
}
