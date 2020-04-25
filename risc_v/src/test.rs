// test.rs

use crate::{kmem::{kfree, kmalloc},
            syscall::syscall_fs_read};

pub fn test_block() {
	// Let's test the block driver!
	let bytes_to_read = 1024 * 50;
	let buffer = kmalloc(bytes_to_read);
	unsafe {
		let bytes_read = syscall_fs_read(8, 5, buffer, bytes_to_read as u32, 0);
		println!("FS Read returned {} bytes", bytes_read);
		for i in 0..16 * 4 {
			print!("{:02x}  ", buffer.add(i).read());
			if (i + 1) % 16 == 0 {
				println!();
			}
		}
	}
	println!();
	kfree(buffer);
}
