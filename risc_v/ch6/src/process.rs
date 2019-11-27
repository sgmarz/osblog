// process.rs
// Kernel and user processes
// Stephen Marz
// 27 Nov 2019

use crate::cpu::TrapFrame;

// Let's represent this in C ABI. We do this
// because we need to access some of the fields
// in assembly. Rust gets to choose how it orders
// the fields unless we represent the structure in
// C-style ABI.
#[repr(C)]
pub struct Process {
    frame: TrapFrame,
    stack: *mut u8,
}



