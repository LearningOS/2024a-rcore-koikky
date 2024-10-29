use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use core::cell::RefMut;
use alloc::collections::BTreeMap;
// use alloc::vec::Vec;

pub struct DeadlockDetect {
    inner: UPSafeCell<DeadlockDetectInner>,
}

pub struct DeadlockDetectInner {
    pub detect_flag: bool,
}

impl DeadlockDetect {
    pub fn new() -> Arc<DeadlockDetect> {
        let inner = DeadlockDetectInner {
            detect_flag: false,
        };
        Arc::new(DeadlockDetect{ inner: unsafe{ UPSafeCell::new(inner) } }) 
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, DeadlockDetectInner> {
        self.inner.exclusive_access()
    }

    pub fn enable_detect(&self) {
        self.inner_exclusive_access().detect_flag = true;
    }

    pub fn disable_detect(&self) {
        self.inner_exclusive_access().detect_flag = false;
    }
}

















