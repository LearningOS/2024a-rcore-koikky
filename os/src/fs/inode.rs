//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use super::{File, Stat, StatMode};
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode, DiskInode, BLOCK_SZ};
use lazy_static::*;

/// inode in memory
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    pub inner: UPSafeCell<OSInodeInner>,
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    pub status: Stat,
    pub inode: Arc<Inode>,
}

impl OSInode {
    /// create a new inode in memory
    pub fn new(readable: bool, writable: bool, inode_id: usize, mode: StatMode, nlink: u32, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0,status: Stat{
                dev: 0,
                ino: inode_id as u64,
                mode,
                nlink,
                pad: [0u64; 7],} 
                , inode }) },
        }
    }
    /// read all data from the inode
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

/// List all apps in the root directory
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    ///  The flags argument to the open() system call is constructed by ORing together zero or more of the following values:
    pub struct OpenFlags: u32 {
        /// readyonly
        const RDONLY = 0;
        /// writeonly
        const WRONLY = 1 << 0;
        /// read and write
        const RDWR = 1 << 1;
        /// create new file
        const CREATE = 1 << 9;
        /// truncate file size to 0
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

// extra
pub fn id_find_indirect(id: usize, flags: OpenFlags, inode: Arc<Inode>) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    let link_count = ROOT_INODE.check_count_indirect(id as u32);
    Some(Arc::new(OSInode::new(readable, writable, id, StatMode::FILE, link_count as u32, inode)))
}

/// Open a file
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();
    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            let sized = core::mem::size_of::<DiskInode>();
            let id = ((inode.block_id - inode.fs.lock().inode_area_start_block as usize) * (BLOCK_SZ / sized)) + (inode.block_offset / sized);
            let link_count = ROOT_INODE.check_count_indirect(id as u32);
            Some(Arc::new(OSInode::new(readable, writable, id, StatMode::FILE, link_count as u32, inode)))
        } else {
            // create file
            ROOT_INODE
                .create(name)
                .map(|inode| {
                    let sized = core::mem::size_of::<DiskInode>();
                    let id: usize = ((inode.block_id - inode.fs.lock().inode_area_start_block as usize) * (BLOCK_SZ / sized)) + (inode.block_offset / sized);
                    Arc::new(OSInode::new(readable, writable, id, StatMode::FILE, 1, inode))
                })
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            let sized = core::mem::size_of::<DiskInode>();
            let id = ((inode.block_id - inode.fs.lock().inode_area_start_block as usize) * (BLOCK_SZ / sized)) + (inode.block_offset / sized);
            let link_count = ROOT_INODE.check_count_indirect(id as u32);
            Arc::new(OSInode::new(readable, writable, id, StatMode::FILE, link_count as u32, inode))
        })
    }
}

impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
    
    fn fd_stat(&self, stat: &mut Stat) -> usize {
        let inner = self.inner.exclusive_access();
        stat.dev = inner.status.dev;
        stat.ino = inner.status.ino;
        stat.mode = inner.status.mode;
        stat.nlink = inner.status.nlink; 
        // println!("stat.nlink: {}",inner.status.nlink);
        0
    }
    fn fd_link(&self, name: &str) -> isize {
        let mut inner = self.inner.exclusive_access();
        let old_inode_id = inner.status.ino;
        ROOT_INODE.inode_insert_indirectory(name, old_inode_id as usize);
        // println!("inner.status.nlink: {}",inner.status.nlink);
        0
    }

    fn fd_unlink(&self, name: &str) -> isize {
        let mut inner = self.inner.exclusive_access();
        let inode_id = inner.status.ino;
        let link_count = ROOT_INODE.check_count_indirect(inode_id as u32);
        // didn't consider recycle the data area when link_count < 2 
        if link_count < 2 {
            ROOT_INODE.delete_insert_indirectory(name)
        } else {
            ROOT_INODE.delete_insert_indirectory(name)
        }
    }

    fn fd_identity(&self, id: usize, flag: &mut [bool]) -> bool {
        let inner = self.inner.exclusive_access();
        flag[0] = self.readable;
        flag[1] = self.writable;
        if inner.status.ino == (id as u64) {
            true 
        } else {
            false
        }
    }
}
