// fs.rs
// Filesystem for SOS
// Stephen Marz
// 16 March 2020

use alloc::string::String;

pub trait FileSystem {
    fn init(bdev: usize) -> bool;
    fn open(path: &String) -> Result<Descriptor, FsError>;
    fn read(desc: &Descriptor, buffer: *mut u8, offset: u32, size: u32) -> u32;
    fn write(desc: &Descriptor, buffer: *const u8, offset: u32, size: u32) -> u32;
    fn close(desc: &mut Descriptor);
    fn stat(desc: &Descriptor) -> Stat;
}

/// Stats on a file. This generally mimics an inode
/// since that's the information we want anyway.
/// However, inodes are filesystem specific, and we
/// want a more generic stat.
pub struct Stat {
    pub mode: u16,
    pub size: u32,
    pub uid: u16,
    pub gid: u16,
}

/// A file descriptor
pub struct Descriptor {
    pub blockdev: usize,
    pub node: u32,
    pub loc: u32,
    pub size: u32,
}

pub enum FsError {
    Success,
    FileNotFound,
    Permission,
    IsFile,
    IsDirectory,
}
