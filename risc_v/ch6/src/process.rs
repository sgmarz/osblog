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
static mut CURRENT: [u16; 2] = [0; 2];

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
pub fn add_process_default(pr: fn()) {
    unsafe {
        let p = Process::new_default(pr);
        PROCESS_LIST.push_back(p);
    }
}

// This should only be called once, and its job is to create
// the init process. Right now, this process is in the kernel,
// but later, it should call the shell.
pub fn init() {
    add_process_default(init_process);
    unsafe {
        let p = PROCESS_LIST.back();
        if let Some(pd) = p {
            // Put the initial process on the first CPU.
            CURRENT[0] = pd.pid;
        }
    }
}

// Our process must be able to sleep, wait, or run.
// Running - means that when the scheduler finds this process, it can run it.
// Sleeping - means that the process is waiting on a certain amount of time.
// Waiting - means that the process is waiting on I/O
// Dead - We should never get here, but we can flag a process as Dead and clean
//        it out of the list later.
pub enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Dead
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
    state: ProcessState,
    data: ProcessData,
}

impl Process {
    pub fn new_default(func: fn()) -> Self {
        // This probably shouldn't go here, but we need to calculate
        // a new PID. For now, this just takes the bottom of the list
        // and adds one to the PID. We assume that we're sorting the
        // list in increasing PID order.
        let pd;
        unsafe {
            let plb = PROCESS_LIST.back();
            if let Some(p) = plb {
                pd = p.pid + 1;
            }
            else {
                // If the list is empty, we allocate pid 1.
                pd = 1
            }
        } 
        // Now that we have a PID, let's create a new process.
        // We set the process as waiting so that whomever called us
        // can wake it up themselves. This allows us to take our time
        // and allocate the process.
        Process {
            frame: TrapFrame::zero(),
            stack: alloc(1),
            program_counter: func as usize,
            pid: pd,
            state: ProcessState::Waiting,
            data: ProcessData::zero()
        }
    }
}

// Since we're storing ownership of a Process in the linked list,
// we can cause it to deallocate automatically when it is removed.
impl Drop for Process {
    fn drop(&mut self) {
        // We allocate the stack as a page.
        dealloc(self.stack);
    }
}

// The private data in a process contains information
// that is relevant to where we are, including the path
// and open file descriptors.
pub struct ProcessData {
    cwd_path: [u8; 128],
}

// This is private data that we can query with system calls.
// If we want to implement CFQ (completely fair queuing), which
// is a per-process block queuing algorithm, we can put that here.
impl ProcessData {
    pub fn zero() -> Self {
        ProcessData {
            cwd_path: [0; 128],
        }
    }
}

