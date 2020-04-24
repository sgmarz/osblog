// test.rs

use crate::syscall::{syscall_fs_read, syscall_exit};

pub fn test_block() {
    // Let's test the block driver!
    let buffer = crate::kmem::kmalloc(1024);
    println!("Started test block process, buffer is at {:p}.", buffer);
    unsafe {
        println!("Before FS read");
        syscall_fs_read(8, 1, buffer, 0, 1024);
        println!("After FS read");
        for i in 0..32 {
            print!("{:02x}  ", buffer.add(i).read());
            if (i+1) % 16 == 0 {
                println!();
            }
        }
    }
    println!();
    crate::kmem::kfree(buffer);
    println!("Test block finished");
    syscall_exit();
}
