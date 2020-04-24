// test.rs

extern "C" {
    fn make_syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize);
}

pub fn test_block() {
    // Let's test the block driver!
    let buffer = crate::kmem::kmalloc(1024);
    println!("Started test block process, buffer is at {:p}.", buffer);
    unsafe {
        make_syscall(63, 8, buffer as usize, 1024, 1024);
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
    unsafe {
        make_syscall(93, 0, 0, 0, 0);
    }
}
