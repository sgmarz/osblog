// minixfs.rs
// Minix 3 Filesystem Implementation
// Stephen Marz
// 16 March 2020

use crate::{fs::{Descriptor, FileSystem, FsError, Stat},
            kmem::{kfree, kmalloc, talloc, tfree},
            process::{add_kernel_process_args, get_by_pid, set_running, set_waiting},
            syscall::syscall_block_read};

use crate::cpu::memcpy;
use alloc::string::String;
use core::{mem::size_of, ptr::null_mut};

pub const MAGIC: u16 = 0x4d5a;
pub const BLOCK_SIZE: u32 = 1024;
pub const NUM_IPTRS: u32 = BLOCK_SIZE / 4;

/// The superblock describes the file system on the disk. It gives
/// us all the information we need to read the file system and navigate
/// the file system, including where to find the inodes and zones (blocks).
#[repr(C)]
pub struct SuperBlock {
	pub ninodes:         u32,
	pub pad0:            u16,
	pub imap_blocks:     u16,
	pub zmap_blocks:     u16,
	pub first_data_zone: u16,
	pub log_zone_size:   u16,
	pub pad1:            u16,
	pub max_size:        u32,
	pub zones:           u32,
	pub magic:           u16,
	pub pad2:            u16,
	pub block_size:      u16,
	pub disk_version:    u8,
}

/// An inode stores the "meta-data" to a file. The mode stores the permissions
/// AND type of file. This is how we differentiate a directory from a file. A file
/// size is in here too, which tells us how many blocks we need to read. Finally, the
/// zones array points to where we can find the blocks, which is where the data
/// is contained for the file.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Inode {
	pub mode:   u16,
	pub nlinks: u16,
	pub uid:    u16,
	pub gid:    u16,
	pub size:   u32,
	pub atime:  u32,
	pub mtime:  u32,
	pub ctime:  u32,
	pub zones:  [u32; 10],
}

/// Notice that an inode does not contain the name of a file. This is because
/// more than one file name may refer to the same inode. These are called "hard links"
/// Instead, a DirEntry essentially associates a file name with an inode as shown in
/// the structure below.
#[repr(C)]
pub struct DirEntry {
	pub inode: u32,
	pub name:  [u8; 60],
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

/// The MinixFileSystem implements the FileSystem trait for the VFS.
pub struct MinixFileSystem;

impl MinixFileSystem {
	/// Inodes are the meta-data of a file, including the mode (permissions and type) and
	/// the file's size. They are stored above the data zones, but to figure out where we
	/// need to go to get the inode, we first need the superblock, which is where we can
	/// find all of the information about the filesystem itself.
	pub fn get_inode(desc: &Descriptor, inode_num: u32) -> Option<Inode> {
		// When we read, everything needs to be a multiple of a sector (512 bytes)
		// So, we need to have memory available that's at least 512 bytes, even if
		// we only want 10 bytes or 32 bytes (size of an Inode).
		let mut buffer = BlockBuffer::new(512);

		// Here is a little memory trick. We have a reference and it will refer to the
		// top portion of our buffer. Since we won't be using the super block and inode
		// simultaneously, we can overlap the memory regions.
		let super_block = unsafe { &*(buffer.get_mut() as *mut SuperBlock) };
		let inode = unsafe { &*(buffer.get_mut() as *mut Inode) };
		// Read from the block device. The size is 1 sector (512 bytes) and our offset is past
		// the boot block (first 1024 bytes). This is where the superblock sits.
		syc_read(desc, buffer.get_mut(), 512, 1024);
		if super_block.magic == MAGIC {
			// If we get here, we successfully read what we think is the super block.
			// The math here is 2 - one for the boot block, one for the super block. Then we
			// have to skip the bitmaps blocks. We have a certain number of inode map blocks (imap)
			// and zone map blocks (zmap).
			// The inode comes to us as a NUMBER, not an index. So, we need to subtract 1.
			let inode_offset = (2 + super_block.imap_blocks + super_block.zmap_blocks) as usize
			                   * BLOCK_SIZE as usize + (inode_num as usize - 1) * size_of::<Inode>();

			// Now, we read the inode itself.
			syc_read(desc, buffer.get_mut(), 512, inode_offset as u32);
			return Some(*inode);
		}
		// If we get here, some result wasn't OK. Either the super block
		// or the inode itself.
		None
	}
}

impl FileSystem for MinixFileSystem {
	fn init(_bdev: usize) -> bool {
		false
	}

	fn open(_path: &String) -> Result<Descriptor, FsError> {
		Err(FsError::FileNotFound)
	}

	fn read(desc: &Descriptor, buffer: *mut u8, size: u32, offset: u32) -> u32 {
		let mut blocks_seen = 0u32;
		let offset_block = offset / BLOCK_SIZE;
		let mut offset_byte = offset % BLOCK_SIZE;

		let inode_result = Self::get_inode(desc, desc.node);
		if inode_result.is_none() {
			// The inode couldn't be read, for some reason.
			return 0;
		}
		let inode = inode_result.unwrap();
		// First, the _size parameter (now in bytes_left) is the size of the buffer, not
		// necessarily the size of the file. If our buffer is bigger than the file, we're OK.
		// If our buffer is smaller than the file, then we can only read up to the buffer size.
		let mut bytes_left = if size > inode.size {
			inode.size
		}
		else {
			size
		};
		println!("Bytes left = {}", bytes_left);
		let mut bytes_read = 0u32;
		let mut block_buffer = BlockBuffer::new(BLOCK_SIZE);
		// In Rust, our for loop automatically "declares" i from 0 to < 7. The syntax
		// 0..7 means 0 through to 7 but not including 7. If we want to include 7, we
		// would use the syntax 0..=7.
		for i in 0..7 {
			// There are 7 direct zones in the Minix 3 file system. So, we can just read them
			// one by one. Any zone that has the value 0 is skipped and we check the next
			// zones. This might happen as we start writing and truncating.

			// We really use this to keep track of when we need to actually start reading
			// But an if statement probably takes more time than just incrementing it.
			if offset_block <= blocks_seen {
				// If we get here, then our offset is within our window that we want to see.
				// We need to go to the direct pointer's index. That'll give us a block INDEX.
				// That makes it easy since all we have to do is multiply the block size
				// by whatever we get. If it's 0, we skip it and move on.
				let zone_num = inode.zones[i];
				if zone_num == 0 {
					continue;
				}
				let zone_offset = zone_num * BLOCK_SIZE;
				println!("Zone #{} -> #{} -> {}", i, zone_num, zone_offset);
				syc_read(desc, block_buffer.get_mut(), BLOCK_SIZE, zone_offset);

				let read_this_many = if BLOCK_SIZE - offset_byte > bytes_left {
					bytes_left
				}
				else {
					BLOCK_SIZE - offset_byte
				};
				println!("Copy {} bytes", read_this_many);
				unsafe {
					memcpy(
					       buffer.add(bytes_read as usize,),
					       block_buffer.get().add(offset_byte as usize,),
					       read_this_many as usize,
					);
				}
				offset_byte = 0;
				bytes_read += read_this_many;
				bytes_left -= read_this_many;
				if bytes_left == 0 {
					return bytes_read;
				}
			}
			blocks_seen += 1;
		}
		bytes_read
	}

	fn write(_desc: &Descriptor, _buffer: *const u8, _offset: u32, _size: u32) -> u32 {
		0
	}

	fn close(_desc: &mut Descriptor) {}

	fn stat(desc: &Descriptor) -> Stat {
		let inode_result = Self::get_inode(desc, desc.node);
		// This could be a little dangerous, but the descriptor should be checked in open().
		let inode = inode_result.unwrap();
		Stat { mode: inode.mode,
		       size: inode.size,
		       uid:  inode.uid,
		       gid:  inode.gid, }
	}
}

pub fn syc_read(desc: &Descriptor, buffer: *mut u8, size: u32, offset: u32) {
	syscall_block_read(desc.blockdev, buffer, size, offset);
}

struct ProcArgs {
	pub pid:    u16,
	pub dev:    usize,
	pub buffer: *mut u8,
	pub size:   u32,
	pub offset: u32,
	pub node:   u32,
}

fn read_proc(args_addr: usize) {
	let args_ptr = args_addr as *mut ProcArgs;
	let args = unsafe { args_ptr.as_ref().unwrap() };

	let desc = Descriptor { blockdev: args.dev,
	                        node:     args.node,
	                        loc:      0,
	                        size:     500,
	                        pid:      args.pid, };

	let bytes = MinixFileSystem::read(&desc, args.buffer, args.size, args.offset);

	// Let's write the return result into regs[10], which is A0.
	let ptr = unsafe { get_by_pid(args.pid) };
	if !ptr.is_null() {
		unsafe {
			(*(*ptr).get_frame()).regs[10] = bytes as usize;
		}
	}
	set_running(args.pid);
	tfree(args_ptr);
}

pub fn process_read(pid: u16, dev: usize, node: u32, buffer: *mut u8, size: u32, offset: u32) {
	// println!("FS read {}, {}, 0x{:x}, {}, {}", pid, dev, buffer as usize, size, offset);
	let args = talloc::<ProcArgs>().unwrap();
	args.pid = pid;
	args.dev = dev;
	args.buffer = buffer;
	args.size = size;
	args.offset = offset;
	args.node = node;
	set_waiting(pid);
	let _ = add_kernel_process_args(read_proc, args as *mut ProcArgs as usize);
}
