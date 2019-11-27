// process.rs
// Kernel and user processes
// Stephen Marz
// 27 Nov 2019

use crate::cpu::TrapFrame;
use crate::page::{alloc, dealloc};
use alloc::collections::linked_list::LinkedList;

// Here, we store a process list. It uses the global allocator
// that we made before and its job is to store all processes.
// We will have this list OWN the process. So, anytime we want
// the process, we will consult the process list.
static mut PROCESS_LIST: LinkedList<Process> = LinkedList::new();
static mut CURRENT_0: u16 = 0;

/// We will eventually move this function out of here, but its
/// job is just to take a slot in the process list.
fn init_process() {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

/// Add a process given a function address and then
/// push it onto the LinkedList. Uses Process::new_default
/// to create a new stack, etc.
pub fn add_process_default(proc: fn()) {
    unsafe {
        let p = Process::new_default(proc);
        PROCESS_LIST.push_back(p);
    }
}

pub fn init() {
    add_process_default(init_process);
    unsafe {
        let p = PROCESS_LIST.back();
        if let Some(pd) = p {
            CURRENT_0 = pd.pid;
        }
    }
}

// Let's represent this in C ABI. We do this
// because we need to access some of the fields
// in assembly. Rust gets to choose how it orders
// the fields unless we represent the structure in
// C-style ABI.
#[repr(C)]
pub struct Process {
    frame: TrapFrame,
    stack: *mut u8,
    program_counter: usize,
    pid: u16,
    data: ProcessData,
}

impl Process {
    pub fn new_default(func: fn()) -> Self {
        let pd;
        unsafe {
            let plb = PROCESS_LIST.back();
            if let Some(p) = plb {
                pd = p.pid + 1;
            }
            else {
                pd = 1
            }
        } 
        Process {
            frame: TrapFrame::zero(),
            stack: alloc(1),
            program_counter: func as usize,
            pid: pd,
            data: ProcessData::zero()
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        dealloc(self.stack);
    }
}

// The private data in a process contains information
// that is relevant to where we are, including the path
// and open file descriptors.
pub struct ProcessData {
    cwd_path: [u8; 128],
}

impl ProcessData {
    pub fn zero() -> Self {
        ProcessData {
            cwd_path: [0; 128],
        }
    }
}

