//! Process management syscalls
// use core::marker::Tuple;

use alloc::sync::Arc;
use riscv::paging::PTE;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str, MapPermission, VirtAddr,VPNRange,VirtPageNum},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, suspend_current_and_run_next, TaskControlBlock, TaskStatus, BIGSTRIDE
    },
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
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
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
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
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
    trace!("kernel: sys_get_time");
    let addr = _ts as usize;
    let vir_addr:VirtAddr = addr.into();
    let us = get_time_us();
    let task_cur = current_task().unwrap();
    let inner = task_cur.inner_exclusive_access();
    let phy_addr = inner.memory_set.translate_va(vir_addr).unwrap();
    let ptr = phy_addr.0 as *mut TimeVal;
    unsafe {
        (*ptr).sec = us / 1_000_000;
        (*ptr).usec = us % 1_000_000;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    if _start % 4096 != 0 {
        return -1
    }
    let task_cur = current_task().unwrap();
    let inner = & mut task_cur.inner_exclusive_access();
    let va_s:VirtAddr = _start.into();
    let va_e:VirtAddr = (_start + _len).into();
    let range = VPNRange::new(va_s.floor(), va_e.ceil());
    for va in range {
        if let Some(pte) = inner.memory_set.page_table.translate(va) {
            if pte.is_valid() == true {
                return -1;
            } 
        } 
    }
    let i:MapPermission = MapPermission::U ;
    let mut j:MapPermission = i;
    match _port {
        1 => j = i | MapPermission::R,
        2 => j = i | MapPermission::W,
        3 => j = i | MapPermission::R | MapPermission::W,
        4 => j = i | MapPermission::X,
        5 => j = i | MapPermission::X | MapPermission::R,
        6 => j = i | MapPermission::X | MapPermission::W,
        7 => j = i | MapPermission::X | MapPermission::R | MapPermission::W,
        _ => return -1,
    };
    inner.memory_set.insert_framed_area(va_s,va_e,j);   
    0
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % 4096 != 0 {
        return -1
    }
    let task_cur = current_task().unwrap();
    let mut inner = & mut task_cur.inner_exclusive_access();
    let va_s:VirtAddr = _start.into();
    let va_e:VirtAddr = (_start + _len).into();
    let vpn_s:VirtPageNum = va_s.floor();
    let vpn_e:VirtPageNum = va_e.ceil();
    let set = &mut inner.memory_set;
    let mut x = &mut set.areas;
    let mut pt = &mut set.page_table;
    for area in x.iter_mut() {
        if vpn_e <=  area.vpn_range.get_end() && vpn_s >= area.vpn_range.get_start() {
            for i in vpn_s.0..vpn_e.0 {
                area.unmap_one(&mut pt, VirtPageNum(i));
            }
            return 0;
        } else {
            continue;
        }
    }
    -1
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
    let token = current_user_token();
    let path = translated_str(token, _path);
    let task_cur = current_task().unwrap();
    let mut inner = task_cur.inner_exclusive_access();
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = TaskControlBlock::new(data);
        let pid= new_task.getpid();
        let arc_new_task = Arc::new(new_task);
        inner.children.push(arc_new_task.clone());
        add_task(arc_new_task);
        // println!("{}",pid);
        pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    if _prio <= 1  {
        return -1;
    } 
    let task_cur = current_task().unwrap();
    let mut inner = & mut task_cur.inner_exclusive_access();
    unsafe {
        inner.pass = BIGSTRIDE/(_prio as usize);
    }
    _prio
}
