//! File and filesystem-related syscalls
use alloc::borrow::ToOwned;
use alloc::sync::Arc;
use alloc::task::Wake;

use crate::fs::{open_file, OpenFlags, Stat, StatMode, OSInode, File, id_find_indirect};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer, translated_refmut};
use crate::task::{current_task, current_user_token};


pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    let token = current_user_token();
    let task_cur = current_task().unwrap();
    let mut inner = task_cur.inner_exclusive_access();
    let fstat = translated_refmut(token,_st);
    if let Some(file) = &inner.fd_table[_fd] {
        let file = file.clone();       
        file.fd_stat(fstat); 
        //println!("fstat.nlink: {}",fstat.nlink);
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        return  0;
    } else {
        return -1;
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();
    let old_path = translated_str(token, _old_name);
    let new_path = translated_str(token, _new_name);
    if let Some(old_inode) = open_file(old_path.as_str(), OpenFlags::RDONLY) {
        if old_path == new_path {
            return 0;
        }
        if let Some(_) = open_file(new_path.as_str(), OpenFlags::RDONLY) {
            return -1;
        }
        old_inode.fd_link(new_path.as_str());
        let fd_inner = old_inode.inner.exclusive_access();
        let mut flag:[bool;2] = [false;2];
        for fd in 0..inner.fd_table.len() {
            if let Some(file) = &inner.fd_table[fd] {
                if file.fd_identity(fd_inner.status.ino as usize, &mut flag) {
                    inner.fd_table[fd].take();
                    let flags = match (flag[0],flag[1]) {
                        (true, true) => OpenFlags::RDWR,
                        (true, false) => OpenFlags::RDONLY,
                        (false, true) => OpenFlags::WRONLY,
                        _ => OpenFlags::RDWR,
                    };
                    if let Some(xx) = open_file(old_path.as_str(), flags) {
                        inner.fd_table[fd] = Some(xx);
                    }
                    //println!("inner.status.nlink: {}",fd_inner.status.nlink);
                }
            } 
        }
        return 0;
    } else {
        return -1;
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let path = translated_str(token, _name);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let fd_inner = inode.inner.exclusive_access();
        let id: u64 = fd_inner.status.ino;
        let inner_inode = fd_inner.inode.clone();
        drop(fd_inner);
        if inode.fd_unlink(path.as_str()) < 0 {
            return -1;
        } else {
            let mut flag:[bool;2] = [false;2];
            for fd in 0..inner.fd_table.len() {
                if let Some(file) = &inner.fd_table[fd] {
                    if file.fd_identity(id as usize, &mut flag) {
                        inner.fd_table[fd].take();
                        let flags = match (flag[0],flag[1]) {
                            (true, true) => OpenFlags::RDWR,
                            (true, false) => OpenFlags::RDONLY,
                            (false, true) => OpenFlags::WRONLY,
                            _ => OpenFlags::RDWR,
                        };
                        if let Some(xx) = id_find_indirect(id as usize, flags, inner_inode.clone()) {
                            inner.fd_table[fd] = Some(xx);
                        }
                        //println!("inner.status.nlink: {}",fd_inner.status.nlink);
                    }
                } 
            }
            return 0;
        }
    } else {
        return -1;
    }
}
