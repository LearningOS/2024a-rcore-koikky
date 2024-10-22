//! Process management syscalls

// use alloc::vec::{self, Vec};

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::{VirtAddr,MapPermission,VPNRange,VirtPageNum}, task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, TASK_MANAGER,
    }, timer::get_time_us
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
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
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
    let addr = _ts as usize;
    let vir_addr:VirtAddr = addr.into();
    let us = get_time_us();
    let inner = TASK_MANAGER.inner.exclusive_access();
    let pte = inner.tasks[inner.current_task].memory_set.translate(vir_addr.floor()).unwrap();
    let ppn:usize = pte.ppn().into();
    let phy_addr = (ppn << 12) | (addr & (PAGE_SIZE - 1));
    let ptr = phy_addr as *mut usize;
    unsafe {
        *ptr = us / 1_000_000;
        *ptr.add(1) = us % 1_000_000;
    }
    0

}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    let task = &mut inner.tasks[num];
    //task.task_info.syscall_times[169] +=4;
    let addr = _ti as usize;
    let vir_addr:VirtAddr = addr.into();
    let pte = task.memory_set.translate(vir_addr.floor()).unwrap();
    
    let ppn:usize = pte.ppn().into();
    let phy_addr = (ppn << 12) | (addr & (PAGE_SIZE - 1));
    let ptr = phy_addr as *mut TaskInfo;
    unsafe {
        (*ptr).status = task.task_info.status;
        (*ptr).syscall_times = task.task_info.syscall_times;
        (*ptr).time = task.task_info.time;
    }
    drop(inner);
    0

}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    if _start % 4096 != 0 {
        return -1
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    let va_s:VirtAddr = _start.into();
    let va_e:VirtAddr = (_start + _len).into();
    let range = VPNRange::new(va_s.floor(), va_e.ceil());
    for va in range {
        if let Some(pte) = inner.tasks[num].memory_set.page_table.translate(va) {
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
    inner.tasks[num].memory_set.insert_framed_area(va_s,va_e,j);   
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % 4096 != 0 {
        return -1
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    let va_s:VirtAddr = _start.into();
    let va_e:VirtAddr = (_start + _len).into();
    let vpn_s:VirtPageNum = va_s.floor();
    let vpn_e:VirtPageNum = va_e.ceil();
    let set = &mut inner.tasks[num].memory_set;
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
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
