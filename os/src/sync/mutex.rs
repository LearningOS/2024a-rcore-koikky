//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next, current_process};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self) -> isize;
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return 0 ;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexBlocking::lock");
        // let mut mutex_inner = self.inner.exclusive_access();
        // if mutex_inner.locked {
        //     mutex_inner.wait_queue.push_back(current_task().unwrap());
        //     drop(mutex_inner);
        //     block_current_and_run_next();
        // } else {
        //     mutex_inner.locked = true;
        // }
        // return 0;
        
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        if process_inner.deadlock_detect.inner_exclusive_access().detect_flag {
            //println!("lock+1");
            let mut mutex_inner = self.inner.exclusive_access();
            if mutex_inner.locked {
                return -57005;
            } else {
                mutex_inner.locked = true;
            }
            return 0;
        } else {
            drop(process_inner);
            let mut mutex_inner = self.inner.exclusive_access();
            if mutex_inner.locked {
                mutex_inner.wait_queue.push_back(current_task().unwrap());
                drop(mutex_inner);
                block_current_and_run_next();
            } else {
                mutex_inner.locked = true;
            }
            return 0;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        // let mut mutex_inner = self.inner.exclusive_access();
        // assert!(mutex_inner.locked);
        // if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
        //     wakeup_task(waking_task);
        // } else {
        //     mutex_inner.locked = false;
        // }
        
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        if process_inner.deadlock_detect.inner_exclusive_access().detect_flag {
            let mut mutex_inner = self.inner.exclusive_access();
            mutex_inner.locked = false;
        } else {
            drop(process_inner);
            let mut mutex_inner = self.inner.exclusive_access();
            assert!(mutex_inner.locked);
            if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
                wakeup_task(waking_task);
            } else {
                mutex_inner.locked = false;
            }
        }
    }
}
