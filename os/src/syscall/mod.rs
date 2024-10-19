//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;

use crate::{task::TASK_MANAGER, timer::get_time_ms};

mod fs;
pub mod process;

use fs::*;
use process::*;
/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    let task = &mut inner.tasks[num];
    match syscall_id {
        SYSCALL_WRITE => {
            task.task_info.syscall_times[SYSCALL_WRITE] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_write(args[0], args[1] as *const u8, args[2])
        },
        SYSCALL_EXIT => {
            task.task_info.syscall_times[SYSCALL_EXIT] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_exit(args[0] as i32)
        },
        SYSCALL_YIELD => { 
            task.task_info.syscall_times[SYSCALL_YIELD] += 1; 
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_yield()
        },
        SYSCALL_GET_TIME => {
            task.task_info.syscall_times[SYSCALL_GET_TIME] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_get_time(args[0] as *mut TimeVal, args[1])
        },
        SYSCALL_TASK_INFO => {
            task.task_info.syscall_times[SYSCALL_TASK_INFO] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_task_info(args[0] as *mut TaskInfo)
        },
        SYSCALL_MMAP => {
            task.task_info.syscall_times[SYSCALL_MMAP] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_mmap(args[0], args[1], args[2])
        },
        SYSCALL_MUNMAP => {
            task.task_info.syscall_times[SYSCALL_GET_TIME] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_munmap(args[0], args[1])
        },
        SYSCALL_SBRK => {
            task.task_info.syscall_times[SYSCALL_SBRK] += 1;
            task.task_info.time = get_time_ms() - task.task_first_time;
            drop(inner);
            sys_sbrk(args[0] as i32)
        },
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
