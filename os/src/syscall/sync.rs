use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        process_inner.mutex_avail[id] = Some(1);
        for i in 0..process_inner.tasks.len() {
            if process_inner.tasks[i].is_some() {
                process_inner.mutex_alloc[i].as_mut().unwrap()[id] = Some(0);
                process_inner.mutex_need[i].as_mut().unwrap()[id] = Some(0);
            }
        }
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_avail.push(Some(1));
        for i in 0..process_inner.tasks.len() {
            if process_inner.tasks[i].is_some() {
                process_inner.mutex_alloc[i].as_mut().unwrap().push(Some(0));
                process_inner.mutex_need[i].as_mut().unwrap().push(Some(0));
            }
        }
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());

    // set need
    *(process_inner.mutex_need[tid].as_mut().unwrap()[mutex_id]
        .as_mut()
        .unwrap()) += 1;

    let enable = process_inner.en_deadlock_detect;
    drop(process_inner);

    if enable == false || process.detect_deadlock_for_mutex() {
        // safe request
        drop(process);
        mutex.lock();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        *(process_inner.mutex_avail[mutex_id].as_mut().unwrap()) -= 1;
        *(process_inner.mutex_alloc[tid].as_mut().unwrap()[mutex_id]
            .as_mut()
            .unwrap()) += 1;
        *(process_inner.mutex_need[tid].as_mut().unwrap()[mutex_id]
            .as_mut()
            .unwrap()) -= 1;
        0
    } else {
        // unsafe request
        -0xDEAD
    }
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    *(process_inner.mutex_avail[mutex_id].as_mut().unwrap()) += 1;
    *(process_inner.mutex_alloc[tid].as_mut().unwrap()[mutex_id]
        .as_mut()
        .unwrap()) -= 1;
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        process_inner.sem_avail[id] = Some(res_count as isize);
        for i in 0..process_inner.tasks.len() {
            if process_inner.tasks[i].is_some() {
                process_inner.sem_alloc[i].as_mut().unwrap()[id] = Some(0);
                process_inner.sem_need[i].as_mut().unwrap()[id] = Some(0);
            }
        }
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.sem_avail.push(Some(res_count as isize));
        for i in 0..process_inner.tasks.len() {
            if process_inner.tasks[i].is_some() {
                process_inner.sem_alloc[i].as_mut().unwrap().push(Some(0));
                process_inner.sem_need[i].as_mut().unwrap().push(Some(0));
            }
        }
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    *(process_inner.sem_avail[sem_id].as_mut().unwrap()) += 1;
    *(process_inner.sem_alloc[tid].as_mut().unwrap()[sem_id]
        .as_mut()
        .unwrap()) -= 1;
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());

    // set need
    *(process_inner.sem_need[tid].as_mut().unwrap()[sem_id]
        .as_mut()
        .unwrap()) += 1;

    let enable = process_inner.en_deadlock_detect;
    drop(process_inner);

    if enable == false || process.detect_deadlock_for_semaphore() {
        // safe request
        sem.down();
        let mut process_inner = process.inner_exclusive_access();
        *(process_inner.sem_avail[sem_id].as_mut().unwrap()) -= 1;
        *(process_inner.sem_alloc[tid].as_mut().unwrap()[sem_id]
            .as_mut()
            .unwrap()) += 1;
        *(process_inner.sem_need[tid].as_mut().unwrap()[sem_id]
            .as_mut()
            .unwrap()) -= 1;
        0
    } else {
        // unsafe request
        -0xDEAD
    }
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_enable_deadlock_detect",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    if _enabled == 1 || _enabled == 0 {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.en_deadlock_detect = if _enabled == 1 { true } else { false };
        0
    } else {
        return -1;
    }
}
