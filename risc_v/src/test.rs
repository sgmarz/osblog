// test.rs

use crate::{elf,
			buffer::Buffer,
			fs::MinixFileSystem,
            process::{PROCESS_LIST,
					  PROCESS_LIST_MUTEX}};

/// Test block will load raw binaries into memory to execute them. This function
/// will load ELF files and try to execute them.
pub fn test() {
	// This won't be necessary after we connect this to the VFS, but for now, we need it.
	const BDEV: usize = 8;
	// This could be better. We should see what our probe gave us, and it if is
	// a block device, init the filesystem.
	MinixFileSystem::init(BDEV);
	let file_to_read = "/helloworld.elf";
	let desc = MinixFileSystem::open(BDEV, &file_to_read).ok();
	if desc.is_none() {
		println!("Error reading {}", file_to_read);
		return;
	}
	let ino = desc.unwrap();
	// The bytes to read would usually come from the inode, but we are in an
	// interrupt context right now, so we cannot pause. Usually, this would
	// be done by an exec system call.
	let mut buffer = Buffer::new(ino.size);
	// Read the file from the disk. I got the inode by mounting
	// the harddrive as a loop on Linux and stat'ing the inode.

	let bytes_read = MinixFileSystem::read(BDEV, &ino, buffer.get_mut(), ino.size, 0);
	// After compiling our program, I manually looked and saw it was 18,360
	// bytes. So, to make sure we got the right one, I do a manual check
	// here.
	if bytes_read != ino.size {
		println!(
		         "Unable to load program, which should \
		          be {} bytes, got {}",
		         ino.size, bytes_read
		);
		return;
	}
	// Let's get this program running!
	// Everything is "page" based since we're going to map pages to
	// user space. So, we need to know how many program pages we
	// need. Each page is 4096 bytes.
	let my_proc = elf::File::load_proc(&buffer, bytes_read as usize);
	if my_proc.is_err() {
		println!("Unable to load process");
		return;
	}
	let my_proc = my_proc.ok().unwrap();
	// I took a different tact here than in process.rs. In there I created
	// the process while holding onto the process list. It might
	// matter since this is asynchronous--it is being ran as a kernel process.
	unsafe {
		PROCESS_LIST_MUTEX.sleep_lock();
	}
	if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
		// As soon as we push this process on the list, it'll be
		// schedule-able.
		println!(
		         "Added user process to the scheduler...get ready \
		          for take-off!"
		);
		pl.push_back(my_proc);
		unsafe {
			PROCESS_LIST.replace(pl);
		}
	}
	else {
		println!("Unable to spawn process.");
		// Since my_proc couldn't enter the process list, it
		// will be dropped and all of the associated allocations
		// will be deallocated through the process' Drop trait.
	}
	unsafe {
		PROCESS_LIST_MUTEX.unlock();
	}
	println!();
}

