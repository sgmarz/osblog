// buffer.rs
// BlockBuffer is so useful, we put it here instead
// of in the file system.
// Stephen Marz

use crate::{cpu::memcpy, kmem::{kmalloc, kfree}};
use core::{ptr::null_mut, ops::{Index, IndexMut}};
// We need a Buffer that can automatically be created and destroyed
// in the lifetime of our read and write functions. In C, this would entail
// goto statements that "unravel" all of the allocations that we made. Take
// a look at the read() function to see why I thought this way would be better.
pub struct Buffer {
	buffer: *mut u8,
	len: usize
}

impl Buffer {
	pub fn new(sz: usize) -> Self {
		Self { 
			buffer: kmalloc(sz), 
			len: sz
		}
	}

	pub fn get_mut(&mut self) -> *mut u8 {
		self.buffer
	}

	pub fn get(&self) -> *const u8 {
		self.buffer
	}

	pub fn len(&self) -> usize {
		self.len
	}
}

impl Default for Buffer {
	fn default() -> Self {
		Self::new(1024)
	}
}

impl Index<usize> for Buffer {
	type Output = u8;
	fn index(&self, idx: usize) -> &Self::Output {
		unsafe {
			self.get().add(idx).as_ref().unwrap()
		}
	}
}

impl IndexMut<usize> for Buffer {
	fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
		unsafe {
			self.get_mut().add(idx).as_mut().unwrap()
		}
	}

}

impl Clone for Buffer {
	fn clone(&self) -> Self {
		let mut new = Self {
			buffer: kmalloc(self.len()),
			len: self.len()
		};
		unsafe {
			memcpy(new.get_mut(), self.get(), self.len());
		}
		new
	}
}

// This is why we have the Buffer. Instead of having to unwind
// all other buffers, we drop here when the block buffer goes out of scope.
impl Drop for Buffer {
	fn drop(&mut self) {
		if !self.buffer.is_null() {
			kfree(self.buffer);
			self.buffer = null_mut();
		}
	}
}
