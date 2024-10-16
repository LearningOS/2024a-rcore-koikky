//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next, suspend_current_and_run_next, TaskStatus},
    timer::get_time_us,
};

use crate::task::TASK_MANAGER;

#[repr(C)]
#[derive(Debug)]
/// The
pub struct TimeVal {
    /// Seconds part of the time
    pub sec: usize,
    /// Microseconds part of the time
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
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
    // Your implementation here
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    let task = &mut inner.tasks[num];
    //task.task_info.syscall_times[169] +=4;
    unsafe {
        (*_ti).status = task.task_info.status;
        (*_ti).syscall_times = task.task_info.syscall_times;
        (*_ti).time = task.task_info.time;
        //println!("SYSCALL_GETTIMEOFDAY:{}",(*_ti).syscall_times[169]);
    }
    drop(inner);
    0
}
