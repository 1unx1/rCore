//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::mm::{MapPermission, VirtAddr};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskInnerInfo, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
    /// task inner info list, I use Vec of BTreeMap,
    /// due to hint of https://learningos.github.io/rCore-Tutorial-Guide-2023S/chapter3/5exercise.html
    task_inner_info_list: Vec<TaskInnerInfo>,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                    task_inner_info_list: {
                        let mut info_list = Vec::new();
                        for _ in 0..num_app{
                            info_list.push(TaskInnerInfo::zero_init());
                        }
                        info_list
                    },
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        // run the task for the first time
        inner.task_inner_info_list[0].save_start_time_us();
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            // run the task for the first time
            if inner.task_inner_info_list[next].start_time_us.is_none() {
                inner.task_inner_info_list[next].save_start_time_us();
            }
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// Update syscall times of current `Running` task
    fn update_current_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        if let Some(times) = inner.task_inner_info_list[current]
            .syscall_times
            .get_mut(&syscall_id)
        {
            *times += 1;
        } else {
            inner.task_inner_info_list[current]
                .syscall_times
                .insert(syscall_id, 1);
        }
    }

    /// Get inner info of current `Running` task
    fn get_current_inner_info(&self) -> (Vec<(usize, u32)>, Option<usize>) {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let mut syscall_times: Vec<(usize, u32)> = Vec::new();
        for (syscall_id, times) in &inner.task_inner_info_list[current].syscall_times {
            syscall_times.push((*syscall_id, *times));
        }
        (
            syscall_times,
            inner.task_inner_info_list[current].start_time_us,
        )
    }

    /// Try mapping a contiguous piece of virtual memory [`start`, `end`),
    /// both `start` and `end` are aligned to `PAGE_SIZE`
    fn mmap_from_to(&self, start: usize, end: usize, port: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // VA: [start_va, end_va) <-> VPN: [start_va.floor(), end_va.ceil())
        let start_va: VirtAddr = start.into();
        let end_va: VirtAddr = end.into();
        // check if this piece of memory has been mapped
        for vpn in start_va.floor().0..end_va.ceil().0 {
            if let Some(pte) = inner.tasks[current].memory_set.translate(vpn.into()) {
                if pte.is_valid() {
                    return -1;
                }
            }
        }
        inner.tasks[current]
            .memory_set
            .insert_framed_area(start_va, end_va, {
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

    /// Try unmapping a contiguous piece of virtual memory [`start`, `end`),
    /// both `start` and `end` are aligned to `PAGE_SIZE`
    fn munmap_from_to(&self, start: usize, end: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        // VA: [start_va, end_va) <-> VPN: [start_va.floor(), end_va.ceil())
        let start_va: VirtAddr = start.into();
        let end_va: VirtAddr = end.into();
        // check if this piece of memory has not been mapped
        for vpn in start_va.floor().0..end_va.ceil().0 {
            if let Some(pte) = inner.tasks[current].memory_set.translate(vpn.into()) {
                if pte.is_valid() == false {
                    return -1;
                }
            } else {
                return -1;
            }
        }
        inner.tasks[current]
            .memory_set
            .unmap_from_to(start_va, end_va);
        0
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// Update syscall times of current `Running` task
pub fn update_current_syscall_times(syscall_id: usize) {
    TASK_MANAGER.update_current_syscall_times(syscall_id);
}

/// Get inner info of current `Running` task
pub fn get_current_inner_info() -> (Vec<(usize, u32)>, Option<usize>) {
    TASK_MANAGER.get_current_inner_info()
}

/// Try mapping a contiguous piece of virtual memory from `start`, with length = `len`,
/// both `start` and `len` are aligned to `PAGE_SIZE`
pub fn mmap(start: usize, len: usize, port: usize) -> isize {
    TASK_MANAGER.mmap_from_to(start, start + len, port)
}

/// Try unmapping a contiguous piece of virtual memory from `start`, with length = `len`
/// `start` must be aligned to `PAGE_SIZE`
pub fn munmap(start: usize, len: usize) -> isize {
    TASK_MANAGER.munmap_from_to(start, start + len)
}
