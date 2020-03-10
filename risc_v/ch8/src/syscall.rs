// syscall.rs
// System calls
// Stephen Marz
// 3 Jan 2020

use crate::cpu::TrapFrame;

pub fn do_syscall(mepc: usize, frame: *mut TrapFrame) -> usize {
    let syscall_number;
    unsafe {
        // A0 is X10, so it's register number 10.
        syscall_number = (*frame).regs[10];
        // for i in 0..32 {
        //     print!("regs[{:02}] = 0x{:08x}    ", i, (*frame).regs[i]);
        //     if (i+1) % 4 == 0 {
        //         println!();
        //     }
        // }    
    }
    match syscall_number {
        0 => {
            // Exit
            mepc + 4
        },
        1 => {
            println!("Test syscall");
            mepc + 4
        },
        _ => {
            println!("Unknown syscall number {}", syscall_number);
            mepc + 4
        }
    }
}