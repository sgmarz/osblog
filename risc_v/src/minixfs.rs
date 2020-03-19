// minixfs.rs
// Minix 3 Filesystem Implementation
// Stephen Marz
// 16 March 2020

use crate::fs::{Descriptor, FileSystem, Stat, FsError};
use crate::block;
use alloc::string::String;

pub const MAGIC: u16 = 0x4d5a;

#[repr(C)]
pub struct SuperBlock {
    ninodes: u32,
    pad0: u16,
    imap_blocks: u16,
    zmap_blocks: u16,
    first_data_zone: u16,
    log_zone_size: u16,
    pad1: u16,
    max_size: u32,
    zones: u32,
    magic: u16,
    pad2: u16,
    block_size: u16,
    disk_version: u8,
}

#[repr(C)]
pub struct Inode {
    mode: u16,
    nlinks: u16,
    uid: u16,
    gid: u16,
    size: u32,
    atime: u32,
    mtime: u32,
    ctime: u32,
    zones: [u32; 10],
}

#[repr(C)]
pub struct DirEntry {
    inode: u32,
    name: [u8; 60],
}

pub struct MinixFileSystem {
    sb: SuperBlock
}

impl FileSystem for MinixFileSystem {
    fn init(bdev: usize) -> bool {
        false
    }
    fn open(path: &String) -> Result<Descriptor, FsError> {
        Err(FsError::FileNotFound)
    }
    fn read(desc: &Descriptor, buffer: *mut u8, offset: u32, size: u32) -> u32 {
        0
    }
    fn write(desc: &Descriptor, buffer: *const u8, offset: u32, size: u32) -> u32 {
        0
    }
    fn close(desc: &mut Descriptor) {

    }
    fn stat(desc: &Descriptor) -> Stat {
        Stat {
            mode: 0,
            size: 0,
            uid: 0,
            gid: 0,
        }
    }
}

