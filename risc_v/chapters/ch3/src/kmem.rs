// kmem.rs
// Sub-page level: malloc-like allocation system
// Stephen Marz
// 7 October 2019

use crate::page::{align_val, zalloc, Table, PAGE_SIZE};
use core::{mem::size_of, ptr::null_mut};

#[repr(usize)]
enum AllocListFlags {
	Taken = 1 << 63,
}
impl AllocListFlags {
	pub fn val(self) -> usize {
		self as usize
	}
}

struct AllocList {
	pub flags_size: usize,
}
impl AllocList {
	pub fn is_taken(&self) -> bool {
		self.flags_size & AllocListFlags::Taken.val() != 0
	}

	pub fn is_free(&self) -> bool {
		!self.is_taken()
	}

	pub fn set_taken(&mut self) {
		self.flags_size |= AllocListFlags::Taken.val();
	}

	pub fn set_free(&mut self) {
		self.flags_size &= !AllocListFlags::Taken.val();
	}

	pub fn set_size(&mut self, sz: usize) {
		let k = self.is_taken();
		self.flags_size = sz & !AllocListFlags::Taken.val();
		if k {
			self.flags_size |= AllocListFlags::Taken.val();
		}
	}

	pub fn get_size(&self) -> usize {
		self.flags_size & !AllocListFlags::Taken.val()
	}
}

// This is the head of the allocation. We start here when
// we search for a free memory location.
static mut KMEM_HEAD: *mut AllocList = null_mut();
// In the future, we will have on-demand pages
// so, we need to keep track of our memory footprint to
// see if we actually need to allocate more.
static mut KMEM_ALLOC: usize = 0;
static mut KMEM_PAGE_TABLE: *mut Table = null_mut();


// These functions are safe helpers around an unsafe
// operation.
pub fn get_head() -> *mut u8 {
	unsafe { KMEM_HEAD as *mut u8 }
}

pub fn get_page_table() -> *mut Table {
	unsafe { KMEM_PAGE_TABLE as *mut Table }
}

pub fn get_num_allocations() -> usize {
	unsafe { KMEM_ALLOC }
}

/// Initialize kernel's memory
/// This is not to be used to allocate memory
/// for user processes. If that's the case, use
/// alloc/dealloc from the page crate.
pub fn init() {
	unsafe {
		// Allocate 64 kernel pages (64 * 4096 = 262 KiB)
		let k_alloc = zalloc(64);
		assert!(!k_alloc.is_null());
		KMEM_ALLOC = 64;
		KMEM_HEAD = k_alloc as *mut AllocList;
		(*KMEM_HEAD).set_free();
		(*KMEM_HEAD).set_size(KMEM_ALLOC * PAGE_SIZE);
		KMEM_PAGE_TABLE = zalloc(1) as *mut Table;
	}
}

/// Allocate sub-page level allocation based on bytes and zero the memory
pub fn kzmalloc(sz: usize) -> *mut u8 {
	let size = align_val(sz, 3);
	let ret = kmalloc(size);

	if !ret.is_null() {
		for i in 0..size {
			unsafe {
				(*ret.add(i)) = 0;
			}
		}
	}
	ret
}

/// Allocate sub-page level allocation based on bytes
pub fn kmalloc(sz: usize) -> *mut u8 {
	unsafe {
		let size = align_val(sz, 3) + size_of::<AllocList>();
		let mut head = KMEM_HEAD;
		// .add() uses pointer arithmetic, so we type-cast into a u8
		// so that we multiply by an absolute size (KMEM_ALLOC *
		// PAGE_SIZE).
		let tail = (KMEM_HEAD as *mut u8).add(KMEM_ALLOC * PAGE_SIZE)
		           as *mut AllocList;

		while head < tail {
			if (*head).is_free() && size <= (*head).get_size() {
				let chunk_size = (*head).get_size();
				let rem = chunk_size - size;
				(*head).set_taken();
				if rem > size_of::<AllocList>() {
					let next = (head as *mut u8).add(size)
					           as *mut AllocList;
					// There is space remaining here.
					(*next).set_free();
					(*next).set_size(rem);
					(*head).set_size(size);
				}
				else {
					// If we get here, take the entire chunk
					(*head).set_size(chunk_size);
				}
				return head.add(1) as *mut u8;
			}
			else {
				// If we get here, what we saw wasn't a free
				// chunk, move on to the next.
				head = (head as *mut u8).add((*head).get_size())
				       as *mut AllocList;
			}
		}
	}
	// If we get here, we didn't find any free chunks--i.e. there isn't
	// enough memory for this. TODO: Add on-demand page allocation.
	null_mut()
}

/// Free a sub-page level allocation
pub fn kfree(ptr: *mut u8) {
	unsafe {
		if !ptr.is_null() {
			let p = (ptr as *mut AllocList).offset(-1);
			if (*p).is_taken() {
				(*p).set_free();
			}
			// After we free, see if we can combine adjacent free
			// spots to see if we can reduce fragmentation.
			coalesce();
		}
	}
}

/// Merge smaller chunks into a bigger chunk
pub fn coalesce() {
	unsafe {
		let mut head = KMEM_HEAD;
		let tail = (KMEM_HEAD as *mut u8).add(KMEM_ALLOC * PAGE_SIZE)
		           as *mut AllocList;

		while head < tail {
			let next = (head as *mut u8).add((*head).get_size())
			           as *mut AllocList;
			if (*head).get_size() == 0 {
				// If this happens, then we have a bad heap
				// (double free or something). However, that
				// will cause an infinite loop since the next
				// pointer will never move beyond the current
				// location.
				break;
			}
			else if next >= tail {
				// We calculated the next by using the size
				// given as get_size(), however this could push
				// us past the tail. In that case, the size is
				// wrong, hence we break and stop doing what we
				// need to do.
				break;
			}
			else if (*head).is_free() && (*next).is_free() {
				// This means we have adjacent blocks needing to
				// be freed. So, we combine them into one
				// allocation.
				(*head).set_size(
				                 (*head).get_size()
				                 + (*next).get_size(),
				);
			}
			// If we get here, we might've moved. Recalculate new
			// head.
			head = (head as *mut u8).add((*head).get_size())
			       as *mut AllocList;
		}
	}
}

/// For debugging purposes, print the kmem table
pub fn print_table() {
	unsafe {
		let mut head = KMEM_HEAD;
		let tail = (KMEM_HEAD as *mut u8).add(KMEM_ALLOC * PAGE_SIZE)
		           as *mut AllocList;
		while head < tail {
			println!(
			         "{:p}: Length = {:<10} Taken = {}",
			         head,
			         (*head).get_size(),
			         (*head).is_taken()
			);
			head = (head as *mut u8).add((*head).get_size())
			       as *mut AllocList;
		}
	}
}

// ///////////////////////////////////
// / GLOBAL ALLOCATOR
// ///////////////////////////////////

// The global allocator allows us to use the data structures
// in the core library, such as a linked list or B-tree.
// We want to use these sparingly since we have a coarse-grained
// allocator.
use core::alloc::{GlobalAlloc, Layout};

// The global allocator is a static constant to a global allocator
// structure. We don't need any members because we're using this
// structure just to implement alloc and dealloc.
struct OsGlobalAlloc;

unsafe impl GlobalAlloc for OsGlobalAlloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// We align to the next page size so that when
		// we divide by PAGE_SIZE, we get exactly the number
		// of pages necessary.
		kzmalloc(layout.size())
	}

	unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
		// We ignore layout since our allocator uses ptr_start -> last
		// to determine the span of an allocation.
		kfree(ptr);
	}
}

#[global_allocator]
/// Technically, we don't need the {} at the end, but it
/// reveals that we're creating a new structure and not just
/// copying a value.
static GA: OsGlobalAlloc = OsGlobalAlloc {};

#[alloc_error_handler]
/// If for some reason alloc() in the global allocator gets null_mut(),
/// then we come here. This is a divergent function, so we call panic to
/// let the tester know what's going on.
pub fn alloc_error(l: Layout) -> ! {
	panic!(
	       "Allocator failed to allocate {} bytes with {}-byte alignment.",
	       l.size(),
	       l.align()
	);
}
