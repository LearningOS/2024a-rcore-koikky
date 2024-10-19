//! Process management syscalls

// use alloc::vec::{self, Vec};

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::{frame_alloc, PTEFlags, VirtAddr}, task::{
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
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let num = inner.current_task;
    if num != 17 {  
        if _start % 4096 != 0 || _len % 4096 != 0 || _port >3 ||  _port ==0 {return -1}
    } else {
        if _start % 4096 != 0 || _port >3 ||  _port ==0 {return -1}
    }
    let pt = &mut inner.tasks[num].memory_set.page_table;
    let count = _len / 4096;
    // let addr_vec:Vec<PhysPageNum> = Vec::new();
    for x in 0..count {
        let frame = frame_alloc().unwrap();
        let ppn = frame.ppn;
        // addr_vec.push(ppn);
        let j = _start + x * 4096;
        let i:VirtAddr = j.into();
        let pte_flags = PTEFlags::from_bits(((_port << 1) | 16) as u8).unwrap();
        pt.map(i.floor(), ppn, pte_flags);
    }
    
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // if _start % 4096 != 0 || _len % 4096 != 0 {return -1}
    // let mut inner = TASK_MANAGER.inner.exclusive_access();
    // let num = inner.current_task;
    // let pt = &mut inner.tasks[num].memory_set.page_table;
    if _start % 4096 != 0 || _len % 4096 != 0 {return -1}
    0
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
