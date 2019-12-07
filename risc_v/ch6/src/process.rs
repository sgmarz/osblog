// process.rs
// Kernel and user processes
// Stephen Marz
// 27 Nov 2019

use crate::{cpu::{build_satp,
                  mscratch_write,
                  satp_fence_asid,
                  satp_write,
                  SatpMode,
                  TrapFrame},
            page::{alloc,
                   dealloc,
                   map,
                   unmap,
                   zalloc,
                   EntryBits,
                   Table,
                   PAGE_SIZE}};
use alloc::collections::vec_deque::VecDeque;

// How many pages are we going to give a process for their
// stack?
const STACK_PAGES: usize = 2;
// We want to adjust the stack to be at the bottom of the memory allocation
// regardless of where it is on the kernel heap.
const STACK_ADDR_ADJ: usize = 0x3f_0000_0000;
// const STACK_ADDR_ADJ: usize = 0;
// All processes will have a defined starting point in virtual memory.
const PROCESS_STARTING_ADDR: usize = 0x2000_0000;
// const PROCESS_STARTING_ADDR: usize = 0;

// Here, we store a process list. It uses the global allocator
// that we made before and its job is to store all processes.
// We will have this list OWN the process. So, anytime we want
// the process, we will consult the process list.
static mut PROCESS_LIST: Option<VecDeque<Process>> = None;
// We can search through the process list to get a new PID, but
// it's probably easier and faster just to increase the pid:
static mut NEXT_PID: u16 = 1;
// CURRENT will store the PID of the process on a given hart. I'm
// statically allocating a slot per CPU, but we could easily create
// a vector here based on the number of CPUs.
static mut CURRENT: [u16; 2] = [0; 2];

/// We will eventually move this function out of here, but its
/// job is just to take a slot in the process list.
fn init_process() {
	// We can't do much here until we have system calls because
	// we're running in User space.
	loop {}
}

/// Add a process given a function address and then
/// push it onto the LinkedList. Uses Process::new_default
/// to create a new stack, etc.
pub fn add_process_default(pr: fn()) {
	unsafe {
		// This is the Rust-ism that really trips up C++ programmers.
		// PROCESS_LIST is wrapped in an Option<> enumeration, which
		// means that the Option owns the Deque. We can only borrow from
		// it or move ownership to us. In this case, we choose the
		// latter, where we move ownership to us, add a process, and
		// then move ownership back to the PROCESS_LIST.
		// This allows mutual exclusion as anyone else trying to grab
		// the process list will get None rather than the Deque.
		if let Some(mut pl) = PROCESS_LIST.take() {
			// .take() will replace PROCESS_LIST with None and give
			// us the only copy of the Deque.
			let p = Process::new_default(pr);
			pl.push_back(p);
			// Now, we no longer need the owned Deque, so we hand it
			// back by replacing the PROCESS_LIST's None with the
			// Some(pl).
			PROCESS_LIST.replace(pl);
		}
		// TODO: When we get to multi-hart processing, we need to keep
		// trying to grab the process list. We can do this with an
		// atomic instruction. but right now, we're a single-processor
		// computer.
	}
}

/// This should only be called once, and its job is to create
/// the init process. Right now, this process is in the kernel,
/// but later, it should call the shell.
pub fn init() -> usize {
	unsafe {
		PROCESS_LIST = Some(VecDeque::with_capacity(5));
		add_process_default(init_process);
		CURRENT[0] = 1;
		// Ugh....Rust is giving me fits over here!
		// I just want a memory address to the trap frame, but
		// due to the borrow rules of Rust, I'm fighting here. So,
		// instead, let's move the value out of PROCESS_LIST, get
		// the address, and then move it right back in.
		let pl = PROCESS_LIST.take().unwrap();
		let p = pl.front().unwrap().frame;
		let frame = &p as *const TrapFrame as usize;
		mscratch_write(frame);
		satp_write(build_satp(
			SatpMode::Sv39,
			1,
			pl.front().unwrap().root as usize,
		),);
		// Synchronize PID 1. We use ASID as the PID.
		satp_fence_asid(1);
		// Put the process list back in the global.
		PROCESS_LIST.replace(pl);
		// Return the first instruction's address to execute.
		// Since we use the MMU, all start here.
		PROCESS_STARTING_ADDR
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
	Dead,
}

// Let's represent this in C ABI. We do this
// because we need to access some of the fields
// in assembly. Rust gets to choose how it orders
// the fields unless we represent the structure in
// C-style ABI.
#[repr(C)]
pub struct Process {
	frame:           TrapFrame,
	stack:           *mut u8,
	program_counter: usize,
	pid:             u16,
	root:            *mut Table,
	state:           ProcessState,
	data:            ProcessData,
}

impl Process {
	pub fn new_default(func: fn()) -> Self {
		let func_addr = func as usize;
		// We will convert NEXT_PID below into an atomic increment when
		// we start getting into multi-hart processing. For now, we want
		// a process. Get it to work, then improve it!
		let mut ret_proc =
			Process { frame:           TrapFrame::zero(),
			          stack:           alloc(STACK_PAGES),
			          program_counter: PROCESS_STARTING_ADDR,
			          pid:             unsafe { NEXT_PID },
			          root:            zalloc(1) as *mut Table,
			          state:           ProcessState::Waiting,
			          data:            ProcessData::zero(), };
		unsafe {
			NEXT_PID += 1;
		}
		// Now we move the stack pointer to the bottom of the
		// allocation. The spec shows that register x2 (2) is the stack
		// pointer.
		// We could use ret_proc.stack.add, but that's an unsafe
		// function which would require an unsafe block. So, convert it
		// to usize first and then add PAGE_SIZE is better.
		// We also need to set the stack adjustment so that it is at the
		// bottom of the memory and far away from heap allocations.
		ret_proc.frame.regs[2] = ret_proc.stack as usize
		                         + STACK_ADDR_ADJ + PAGE_SIZE
		                                            * STACK_PAGES;
		// Map the stack on the MMU
		let pt;
		unsafe {
			pt = &mut *ret_proc.root;
		}
		let saddr = ret_proc.stack as usize;
		// We need to map the stack onto the user process' virtual
		// memory This gets a little hairy because we need to also map
		// the function code too.
		for i in 0..STACK_PAGES {
			let addr = saddr + i * PAGE_SIZE;
			map(
			    pt,
			    addr + STACK_ADDR_ADJ,
			    addr,
			    EntryBits::UserReadWrite.val(),
			    0,
			);
		}
		// Map the program counter on the MMU
		map(
		    pt,
		    PROCESS_STARTING_ADDR,
		    func_addr,
		    EntryBits::UserReadExecute.val(),
		    0,
		);
		map(
		    pt,
		    PROCESS_STARTING_ADDR + 0x1001,
		    func_addr + 0x1001,
		    EntryBits::UserReadExecute.val(),
		    0,
		);
		ret_proc
	}
}

impl Drop for Process {
	/// Since we're storing ownership of a Process in the linked list,
	/// we can cause it to deallocate automatically when it is removed.
	fn drop(&mut self) {
		// We allocate the stack as a page.
		dealloc(self.stack);
		// This is unsafe, but it's at the drop stage, so we won't
		// be using this again.
		unsafe {
			// Remember that unmap unmaps all levels of page tables
			// except for the root. It also deallocates the memory
			// associated with the tables.
			unmap(&mut *self.root);
		}
		dealloc(self.root as *mut u8);
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
		ProcessData { cwd_path: [0; 128], }
	}
}
