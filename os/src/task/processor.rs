//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.

use super::__switch;
use super::task::TaskInnerInfo;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, VirtAddr};
use crate::sync::UPSafeCell;
use crate::syscall::{
    SYSCALL_CLOSE, SYSCALL_EXEC, SYSCALL_EXIT, SYSCALL_FORK, SYSCALL_FSTAT, SYSCALL_GETPID,
    SYSCALL_GET_TIME, SYSCALL_LINKAT, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_OPEN, SYSCALL_READ,
    SYSCALL_SBRK, SYSCALL_SET_PRIORITY, SYSCALL_SPAWN, SYSCALL_TASK_INFO, SYSCALL_UNLINKAT,
    SYSCALL_WAITPID, SYSCALL_WRITE, SYSCALL_YIELD,
};
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;

/// Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,

    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            task_inner.task_inner_info.save_start_time_us();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}
/// Update syscall times of current `Running` task
pub fn update_current_syscall_times(syscall_id: usize) {
    let index = match syscall_id {
        SYSCALL_UNLINKAT => 0,
        SYSCALL_LINKAT => 1,
        SYSCALL_OPEN => 2,
        SYSCALL_CLOSE => 3,
        SYSCALL_READ => 4,
        SYSCALL_WRITE => 5,
        SYSCALL_FSTAT => 6,
        SYSCALL_EXIT => 7,
        SYSCALL_YIELD => 8,
        SYSCALL_SET_PRIORITY => 9,
        SYSCALL_GET_TIME => 10,
        SYSCALL_GETPID => 11,
        SYSCALL_SBRK => 12,
        SYSCALL_MUNMAP => 13,
        SYSCALL_FORK => 14,
        SYSCALL_EXEC => 15,
        SYSCALL_MMAP => 16,
        SYSCALL_WAITPID => 17,
        SYSCALL_SPAWN => 18,
        SYSCALL_TASK_INFO => 19,
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .task_inner_info
        .syscall_times[index] += 1;
}

/// Get inner info of current `Running` task
pub fn get_current_inner_info() -> TaskInnerInfo {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .task_inner_info
}

/// Try mapping a contiguous piece of virtual memory from `start`, with length = `len`,
/// both `start` and `len` are aligned to `PAGE_SIZE`
pub fn mmap(start: usize, len: usize, port: usize) -> isize {
    if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let binding = current_task().unwrap();
    let mut inner = binding.inner_exclusive_access();
    // VA: [start_va, end_va) <-> VPN: [start_va.floor(), end_va.ceil())
    let start_va: VirtAddr = start.into();
    let end_va: VirtAddr = (start + len).into();
    // check if this piece of memory has been mapped
    for vpn in start_va.floor().0..end_va.ceil().0 {
        if let Some(pte) = inner.memory_set.translate(vpn.into()) {
            if pte.is_valid() {
                return -1;
            }
        }
    }
    inner.memory_set.insert_framed_area(start_va, end_va, {
        let mut map_perm = MapPermission::U;
        if port & 1 != 0 {
            map_perm |= MapPermission::R;
        }
        if port & 2 != 0 {
            map_perm |= MapPermission::W;
        }
        if port & 4 != 0 {
            map_perm |= MapPermission::X;
        }
        map_perm
    });
    0
}

/// Try unmapping a contiguous piece of virtual memory from `start`, with length = `len`
/// `start` must be aligned to `PAGE_SIZE`
pub fn munmap(start: usize, len: usize) -> isize {
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    let binding = current_task().unwrap();
    let mut inner = binding.inner_exclusive_access();
    // VA: [start_va, end_va) <-> VPN: [start_va.floor(), end_va.ceil())
    let start_va: VirtAddr = start.into();
    let end_va: VirtAddr = (start + len).into();
    // check if this piece of memory has not been mapped
    for vpn in start_va.floor().0..end_va.ceil().0 {
        if let Some(pte) = inner.memory_set.translate(vpn.into()) {
            if pte.is_valid() == false {
                return -1;
            }
        } else {
            return -1;
        }
    }
    inner.memory_set.unmap_from_to(start_va, end_va);
    0
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
