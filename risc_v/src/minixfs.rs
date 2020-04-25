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

		// For Rust-ers, I'm showing two ways here. The first way is to get a reference
		// from a pointer. You will see the &* a lot in Rust for references. Rust
		// makes dereferencing a pointer cumbersome, which lends to not using them.
		let super_block = unsafe { &*(buffer.get_mut() as *mut SuperBlock) };
		// I opted for a pointer here instead of a reference because we will be offsetting the inode by a certain amount.
		let inode = buffer.get_mut() as *mut Inode;
		// Read from the block device. The size is 1 sector (512 bytes) and our offset is past
		// the boot block (first 1024 bytes). This is where the superblock sits.
		syc_read(desc, buffer.get_mut(), 512, 1024);
		if super_block.magic == MAGIC {
			// If we get here, we successfully read what we think is the super block.
			// The math here is 2 - one for the boot block, one for the super block. Then we
			// have to skip the bitmaps blocks. We have a certain number of inode map blocks (imap)
			// and zone map blocks (zmap).
			// The inode comes to us as a NUMBER, not an index. So, we need to subtract 1.
			let inode_offset = (2 + super_block.imap_blocks + super_block.zmap_blocks) as usize * BLOCK_SIZE as usize;

			// Now, we read the inode itself.
			// The block driver requires that our offset be a multiple of 512. We do that with the
			// inode_offset. However, we're going to be reading a group of inodes.
			syc_read(desc, buffer.get_mut(), 512, inode_offset as u32);

			// There are 512 / size_of<Inode>() inodes in each read that we can do. However, we need to figure out which inode in that group we need to read. We just take the % of this to find out.
			let read_this_node = (inode_num as usize - 1) % (512 / size_of::<Inode>());

			// We copy the inode over. This might not be the best thing since the Inode will
			// eventually have to change after writing.
			return unsafe { Some(*(inode.add(read_this_node))) };
		}
		// If we get here, some result wasn't OK. Either the super block
		// or the inode itself.
		None
	}
}

impl FileSystem for MinixFileSystem {
	/// Init is where we would cache the superblock and inode to avoid having to read
	/// it over and over again, like we do for read right now.
	fn init(_bdev: usize) -> bool {
		false
	}

	/// The goal of open is to traverse the path given by path. If we cache the inodes
	/// in RAM, it might make this much quicker. For now, this doesn't do anything since
	/// we're just testing read based on if we know the Inode we're looking for.
	fn open(_path: &String) -> Result<Descriptor, FsError> {
		Err(FsError::FileNotFound)
	}

	fn read(desc: &Descriptor, buffer: *mut u8, size: u32, offset: u32) -> u32 {
		// Our strategy here is to use blocks to see when we need to start reading
		// based on the offset. That's offset_block. Then, the actual byte within
		// that block that we need is offset_byte.
		let mut blocks_seen = 0u32;
		let offset_block = offset / BLOCK_SIZE;
		let mut offset_byte = offset % BLOCK_SIZE;
		let num_indirect_pointers = BLOCK_SIZE as usize / 4;
		let inode_result = Self::get_inode(desc, desc.node);
		if inode_result.is_none() {
			// The inode couldn't be read, for some reason.
			return 0;
		}
		// We've already checked is_none() above, so we can safely unwrap here.
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
		let mut bytes_read = 0u32;
		// The block buffer automatically drops when we quit early due to an error or we've read enough. This will be the holding port when we go out and read a block. Recall that even if we want 10 bytes, we have to read the entire block (really only 512 bytes of the block) first. So, we use the block_buffer as the middle man, which is then copied into the buffer.
		let mut block_buffer = BlockBuffer::new(BLOCK_SIZE);
		// Triply indirect zones point to a block of pointers (BLOCK_SIZE / 4). Each one of those pointers points to another block of pointers (BLOCK_SIZE / 4). Each one of those pointers yet again points to another block of pointers (BLOCK_SIZE / 4). This is why we have indirect, iindirect (doubly), and iiindirect (triply).
		let mut indirect_buffer = BlockBuffer::new(BLOCK_SIZE);
		let mut iindirect_buffer = BlockBuffer::new(BLOCK_SIZE);
		let mut iiindirect_buffer = BlockBuffer::new(BLOCK_SIZE);
		// I put the pointers *const u32 here. That means we will allocate the indirect, doubly indirect, and triply indirect even for small files. I initially had these in their respective scopes, but that required us to recreate the indirect buffer for doubly indirect and both the indirect and doubly indirect buffers for the triply indirect. Not sure which is better, but I probably wasted brain cells on this.
		let izones = indirect_buffer.get() as *const u32;
		let iizones = iindirect_buffer.get() as *const u32;
		let iiizones = iiindirect_buffer.get() as *const u32;

		// ////////////////////////////////////////////
		// // DIRECT ZONES
		// ////////////////////////////////////////////
		// In Rust, our for loop automatically "declares" i from 0 to < 7. The syntax
		// 0..7 means 0 through to 7 but not including 7. If we want to include 7, we
		// would use the syntax 0..=7.
		for i in 0..7 {
			// There are 7 direct zones in the Minix 3 file system. So, we can just read them one by one. Any zone that has the value 0 is skipped and we check the next zones. This might happen as we start writing and truncating.
			if inode.zones[i] == 0 {
				continue;
			}
			// We really use this to keep track of when we need to actually start reading
			// But an if statement probably takes more time than just incrementing it.
			if offset_block <= blocks_seen {
				// If we get here, then our offset is within our window that we want to see.
				// We need to go to the direct pointer's index. That'll give us a block INDEX.
				// That makes it easy since all we have to do is multiply the block size
				// by whatever we get. If it's 0, we skip it and move on.
				let zone_offset = inode.zones[i] * BLOCK_SIZE;
				// We read the zone, which is where the data is located. The zone offset is simply the block
				// size times the zone number. This makes it really easy to read!
				syc_read(desc, block_buffer.get_mut(), BLOCK_SIZE, zone_offset);

				// There's a little bit of math to see how much we need to read. We don't want to read
				// more than the buffer passed in can handle, and we don't want to read if we haven't
				// taken care of the offset. For example, an offset of 10000 with a size of 2 means we
				// can only read bytes 10,000 and 10,001.
				let read_this_many = if BLOCK_SIZE - offset_byte > bytes_left {
					bytes_left
				}
				else {
					BLOCK_SIZE - offset_byte
				};
				// Once again, here we actually copy the bytes into the final destination, the buffer. This memcpy
				// is written in cpu.rs.
				unsafe {
					memcpy(
					       buffer.add(bytes_read as usize,),
					       block_buffer.get().add(offset_byte as usize,),
					       read_this_many as usize,
					);
				}
				// Regardless of whether we have an offset or not, we reset the offset byte back to 0. This
				// probably will get set to 0 many times, but who cares?
				offset_byte = 0;
				// Reset the statistics to see how many bytes we've read versus how many are left.
				bytes_read += read_this_many;
				bytes_left -= read_this_many;
				// If no more bytes are left, then we're done.
				if bytes_left == 0 {
					return bytes_read;
				}
			}
			// The blocks_seen is for the offset. We need to skip a certain number of blocks FIRST before getting
			// to the offset. The reason we need to read the zones is because we need to skip zones of 0, and they
			// do not contribute as a "seen" block.
			blocks_seen += 1;
		}
		// ////////////////////////////////////////////
		// // SINGLY INDIRECT ZONES
		// ////////////////////////////////////////////
		// Each indirect zone is a list of pointers, each 4 bytes. These then
		// point to zones where the data can be found. Just like with the direct zones,
		// we need to make sure the zone isn't 0. A zone of 0 means skip it.
		if inode.zones[7] != 0 {
			syc_read(desc, indirect_buffer.get_mut(), BLOCK_SIZE, BLOCK_SIZE * inode.zones[7]);
			let izones = indirect_buffer.get() as *const u32;
			for i in 0..num_indirect_pointers {
				// Where do I put unsafe? Dereferencing the pointers and memcpy are the unsafe functions.
				unsafe {
					if izones.add(i).read() != 0 {
						if offset_block <= blocks_seen {
							syc_read(
							         desc,
							         block_buffer.get_mut(),
							         BLOCK_SIZE,
							         BLOCK_SIZE * izones.add(i,).read(),
							);
							let read_this_many = if BLOCK_SIZE - offset_byte > bytes_left {
								bytes_left
							}
							else {
								BLOCK_SIZE - offset_byte
							};
							memcpy(
							       buffer.add(bytes_read as usize,),
							       block_buffer.get().add(offset_byte as usize,),
							       read_this_many as usize,
							);
							bytes_read += read_this_many;
							bytes_left -= read_this_many;
							offset_byte = 0;
							if bytes_left == 0 {
								return bytes_read;
							}
						}
						blocks_seen += 1;
					}
				}
			}
		}
		// ////////////////////////////////////////////
		// // DOUBLY INDIRECT ZONES
		// ////////////////////////////////////////////
		if inode.zones[8] != 0 {
			syc_read(desc, indirect_buffer.get_mut(), BLOCK_SIZE, BLOCK_SIZE * inode.zones[8]);
			unsafe {
				for i in 0..num_indirect_pointers {
					if izones.add(i).read() != 0 {
						syc_read(desc, iindirect_buffer.get_mut(), BLOCK_SIZE, BLOCK_SIZE * izones.add(i).read());
						for j in 0..num_indirect_pointers {
							if iizones.add(j).read() != 0 {
								// Notice that this inner code is the same for all end-zone pointers. I'm thinking about
								// moving this out of here into a function of its own, but that might make it harder
								// to follow.
								if offset_block <= blocks_seen {
									syc_read(
									         desc,
									         block_buffer.get_mut(),
									         BLOCK_SIZE,
									         BLOCK_SIZE * iizones.add(j,).read(),
									);
									let read_this_many = if BLOCK_SIZE - offset_byte > bytes_left {
										bytes_left
									}
									else {
										BLOCK_SIZE - offset_byte
									};
									memcpy(
									       buffer.add(bytes_read as usize,),
									       block_buffer.get().add(offset_byte as usize,),
									       read_this_many as usize,
									);
									bytes_read += read_this_many;
									bytes_left -= read_this_many;
									offset_byte = 0;
									if bytes_left == 0 {
										return bytes_read;
									}
								}
								blocks_seen += 1;
							}
						}
					}
				}
			}
		}
		// ////////////////////////////////////////////
		// // TRIPLY INDIRECT ZONES
		// ////////////////////////////////////////////
		if inode.zones[9] != 0 {
			syc_read(desc, indirect_buffer.get_mut(), BLOCK_SIZE, BLOCK_SIZE * inode.zones[9]);
			unsafe {
				for i in 0..num_indirect_pointers {
					if izones.add(i).read() != 0 {
						syc_read(desc, iindirect_buffer.get_mut(), BLOCK_SIZE, BLOCK_SIZE * izones.add(i).read());
						for j in 0..num_indirect_pointers {
							if iizones.add(j).read() != 0 {
								syc_read(
								         desc,
								         iiindirect_buffer.get_mut(),
								         BLOCK_SIZE,
								         BLOCK_SIZE * iizones.add(j,).read(),
								);
								for k in 0..num_indirect_pointers {
									if iiizones.add(k).read() != 0 {
										// Hey look! This again.
										if offset_block <= blocks_seen {
											syc_read(
											         desc,
											         block_buffer.get_mut(),
											         BLOCK_SIZE,
											         BLOCK_SIZE * iiizones.add(k,).read(),
											);
											let read_this_many =
												if BLOCK_SIZE - offset_byte > bytes_left {
													bytes_left
												}
												else {
													BLOCK_SIZE - offset_byte
												};
											memcpy(
											       buffer.add(bytes_read as usize,),
											       block_buffer.get()
											                   .add(offset_byte as usize,),
											       read_this_many as usize,
											);
											bytes_read += read_this_many;
											bytes_left -= read_this_many;
											offset_byte = 0;
											if bytes_left == 0 {
												return bytes_read;
											}
										}
										blocks_seen += 1;
									}
								}
							}
						}
					}
				}
			}
		}
		// Anyone else love this stairstep style? I probably should put the pointers in a function by themselves,
		// but I think that'll make it more difficult to see what's actually happening.

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

/// This is a wrapper function around the syscall_block_read. This allows me to do
/// other things before I call the system call (or after). However, all the things I
/// wanted to do are no longer there, so this is a worthless function.
fn syc_read(desc: &Descriptor, buffer: *mut u8, size: u32, offset: u32) -> u8 {
	syscall_block_read(desc.blockdev, buffer, size, offset)
}

// We have to start a process when reading from a file since the block
// device will block. We only want to block in a process context, not an
// interrupt context.
struct ProcArgs {
	pub pid:    u16,
	pub dev:    usize,
	pub buffer: *mut u8,
	pub size:   u32,
	pub offset: u32,
	pub node:   u32,
}

// This is the actual code ran inside of the read process.
fn read_proc(args_addr: usize) {
	let args_ptr = args_addr as *mut ProcArgs;
	let args = unsafe { args_ptr.as_ref().unwrap() };

	// The descriptor will come from the user after an open() call. However,
	// for now, all we really care about is args.dev, args.node, and args.pid.
	let desc = Descriptor { blockdev: args.dev,
	                        node:     args.node,
	                        loc:      0,
	                        size:     500,
	                        pid:      args.pid, };

	// Start the read! Since we're in a kernel process, we can block by putting this
	// process into a waiting state and wait until the block driver returns.
	let bytes = MinixFileSystem::read(&desc, args.buffer, args.size, args.offset);

	// Let's write the return result into regs[10], which is A0.
	unsafe {
		let ptr = get_by_pid(args.pid);
		if !ptr.is_null() {
			(*(*ptr).get_frame_mut()).regs[10] = bytes as usize;
		}
	}
	// This is the process making the system call. The system itself spawns another process
	// which goes out to the block device. Since we're passed the read call, we need to awaken
	// the process and get it ready to go. The only thing this process needs to clean up is the
	// tfree(), but the user process doesn't care about that.
	set_running(args.pid);

	// tfree() is used to free a pointer created by talloc.
	tfree(args_ptr);
}

/// System calls will call process_read, which will spawn off a kernel process to read
/// the requested data.
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
