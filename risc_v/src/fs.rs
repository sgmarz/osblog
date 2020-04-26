// fs.rs
// Filesystem for SOS
// Stephen Marz
// 16 March 2020

use alloc::string::String;
use crate::kmem::{kfree, kmalloc};
use core::ptr::null_mut;

pub trait FileSystem {
    fn init(bdev: usize) -> bool;
    fn open(path: &String) -> Result<Descriptor, FsError>;
    fn read(desc: &Descriptor, buffer: *mut u8, size: u32, offset: u32) -> u32;
    fn write(desc: &Descriptor, buffer: *const u8, size: u32, offset: u32) -> u32;
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
    pub pid: u16,
}

pub enum FsError {
    Success,
    FileNotFound,
    Permission,
    IsFile,
    IsDirectory,
}

// We need a BlockBuffer that can automatically be created and destroyed
// in the lifetime of our read and write functions. In C, this would entail
// goto statements that "unravel" all of the allocations that we made. Take
// a look at the read() function to see why I thought this way would be better.
pub struct BlockBuffer {
	buffer: *mut u8,
}

impl BlockBuffer {
	pub fn new(sz: u32) -> Self {
		BlockBuffer { buffer: kmalloc(sz as usize), }
	}

	pub fn get_mut(&mut self) -> *mut u8 {
		self.buffer
	}

	pub fn get(&self) -> *const u8 {
		self.buffer
	}
}

impl Default for BlockBuffer {
	fn default() -> Self {
		BlockBuffer { buffer: kmalloc(1024), }
	}
}

// This is why we have the BlockBuffer. Instead of having to unwind
// all other buffers, we drop here when the block buffer goes out of scope.
impl Drop for BlockBuffer {
	fn drop(&mut self) {
		if !self.buffer.is_null() {
			kfree(self.buffer);
			self.buffer = null_mut();
		}
	}
}
