// buffer.rs
// BlockBuffer is so useful, we put it here instead
// of in the file system.
// Stephen Marz

use crate::kmem::{kmalloc, kfree};
use core::ptr::null_mut;
// We need a Buffer that can automatically be created and destroyed
// in the lifetime of our read and write functions. In C, this would entail
// goto statements that "unravel" all of the allocations that we made. Take
// a look at the read() function to see why I thought this way would be better.
pub struct Buffer {
	buffer: *mut u8,
}

impl Buffer {
	pub fn new(sz: u32) -> Self {
		Self { buffer: kmalloc(sz as usize), }
	}

	pub fn get_mut(&mut self) -> *mut u8 {
		self.buffer
	}

	pub fn get(&self) -> *const u8 {
		self.buffer
	}
}

impl Default for Buffer {
	fn default() -> Self {
		Self { buffer: kmalloc(1024), }
	}
}

// This is why we have the BlockBuffer. Instead of having to unwind
// all other buffers, we drop here when the block buffer goes out of scope.
impl Drop for Buffer {
	fn drop(&mut self) {
		if !self.buffer.is_null() {
			kfree(self.buffer);
			self.buffer = null_mut();
		}
	}
}
