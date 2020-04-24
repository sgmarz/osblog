// process.rs
// Kernel and user processes
// Stephen Marz
// 27 Nov 2019

use crate::{cpu::{build_satp, get_mtime, satp_fence_asid, CpuMode, SatpMode, TrapFrame},
            page::{alloc, dealloc, map, unmap, zalloc, EntryBits, Table, PAGE_SIZE}};
use alloc::collections::vec_deque::VecDeque;
use core::ptr::null_mut;

// How many pages are we going to give a process for their
// stack?
const STACK_PAGES: usize = 5;
// We want to adjust the stack to be at the bottom of the memory allocation
// regardless of where it is on the kernel heap.
const STACK_ADDR: usize = 0x1_0000_0000;
// All processes will have a defined starting point in virtual memory.
// We will use this later when we load processes from disk.
// const PROCESS_STARTING_ADDR: usize = 0x2000_0000;

// Here, we store a process list. It uses the global allocator
// that we made before and its job is to store all processes.
// We will have this list OWN the process. So, anytime we want
// the process, we will consult the process list.
// Using an Option here is one method of creating a "lazy static".
// Rust requires that all statics be initialized, but all
// initializations must be at compile-time. We cannot allocate
// a VecDeque at compile time, so we are somewhat forced to
// do this.
pub static mut PROCESS_LIST: Option<VecDeque<Process>> = None;
// We can search through the process list to get a new PID, but
// it's probably easier and faster just to increase the pid:
static mut NEXT_PID: u16 = 1;

/// Set a process' state to running. This doesn't do any checks.
/// If this PID is not found, this returns false. Otherwise, it
/// returns true.
pub fn set_running(pid: u16) -> bool {
	// Yes, this is O(n). A better idea here would be a static list
	// of process pointers.
	let mut retval = false;
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			for proc in pl.iter_mut() {
				if proc.pid == pid {
					proc.set_state(ProcessState::Running);
					retval = true;
					break;
				}
			}
			// Now, we no longer need the owned Deque, so we hand it
			// back by replacing the PROCESS_LIST's None with the
			// Some(pl).
			PROCESS_LIST.replace(pl);
		}
	}
	retval
}

/// Set a process' state to waiting. This doesn't do any checks.
/// If this PID is not found, this returns false. Otherwise, it
/// returns true.
pub fn set_waiting(pid: u16) -> bool {
	// Yes, this is O(n). A better idea here would be a static list
	// of process pointers.
	let mut retval = false;
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			for proc in pl.iter_mut() {
				if proc.pid == pid {
					proc.set_state(ProcessState::Waiting);
					retval = true;
					break;
				}
			}
			// Now, we no longer need the owned Deque, so we hand it
			// back by replacing the PROCESS_LIST's None with the
			// Some(pl).
			PROCESS_LIST.replace(pl);
		}
	}
	retval
}

/// Sleep a process
pub fn set_sleeping(pid: u16, duration: usize) -> bool {
	// Yes, this is O(n). A better idea here would be a static list
	// of process pointers.
	let mut retval = false;
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			for proc in pl.iter_mut() {
				if proc.pid == pid {
					proc.set_state(ProcessState::Sleeping);
					proc.set_sleep_until(get_mtime() + duration);
					retval = true;
					break;
				}
			}
			// Now, we no longer need the owned Deque, so we hand it
			// back by replacing the PROCESS_LIST's None with the
			// Some(pl).
			PROCESS_LIST.replace(pl);
		}
	}
	retval
}

/// Delete a process given by pid. If this process doesn't exist,
/// this function does nothing.
pub fn delete_process(pid: u16) {
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			for i in 0..pl.len() {
				let p = pl.get_mut(i).unwrap();
				if p.get_pid() == pid {
					// When the structure gets dropped, all of the
					// allocations get deallocated.
					pl.remove(i);
					break;
				}
			}
			// Now, we no longer need the owned Deque, so we hand it
			// back by replacing the PROCESS_LIST's None with the
			// Some(pl).
			PROCESS_LIST.replace(pl);
		}
	}
}

/// Get a process by PID. Since we leak the process list, this is
/// unsafe since the process can be deleted and we'll still have a pointer.
pub unsafe fn get_by_pid(pid: u16) -> *mut Process {
	let mut ret = null_mut();
	if let Some(mut pl) = PROCESS_LIST.take() {
		for i in pl.iter_mut() {
			if i.get_pid() == pid {
				ret = i as *mut Process;
				break;
			}
		}
		PROCESS_LIST.replace(pl);
	}

	ret
}

/// We will eventually move this function out of here, but its
/// job is just to take a slot in the process list.
fn init_process() {
	// We can't do much here until we have system calls because
	// we're running in User space.
	loop {
		// Eventually, this will be a sleep system call.
		unsafe {
			extern "C" {
				fn make_syscall(sysno: usize, duration: usize) -> usize;
			}
			println!("Init is still here :), alright, back to sleep.");
			make_syscall(2, 60000000);
		}
	}
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

/// Add a kernel process.
pub fn add_kernel_process(func: fn()) -> u16 {
	// This is the Rust-ism that really trips up C++ programmers.
	// PROCESS_LIST is wrapped in an Option<> enumeration, which
	// means that the Option owns the Deque. We can only borrow from
	// it or move ownership to us. In this case, we choose the
	// latter, where we move ownership to us, add a process, and
	// then move ownership back to the PROCESS_LIST.
	// This allows mutual exclusion as anyone else trying to grab
	// the process list will get None rather than the Deque.
	if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
		// .take() will replace PROCESS_LIST with None and give
		// us the only copy of the Deque.
		let func_addr = func as usize;
		let func_vaddr = func_addr; //- 0x6000_0000;
			    // println!("func_addr = {:x} -> {:x}", func_addr, func_vaddr);
			    // We will convert NEXT_PID below into an atomic increment when
			    // we start getting into multi-hart processing. For now, we want
				// a process. Get it to work, then improve it!
		let my_pid = unsafe {NEXT_PID};
		let mut ret_proc = Process { frame:       zalloc(1) as *mut TrapFrame,
		                             stack:       zalloc(STACK_PAGES),
		                             pid:         my_pid,
		                             root:        zalloc(1) as *mut Table,
		                             state:       ProcessState::Running,
		                             data:        ProcessData::zero(),
		                             sleep_until: 0, };
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
		unsafe {
			(*ret_proc.frame).pc = func_vaddr;
			(*ret_proc.frame).regs[2] = ret_proc.stack as usize + STACK_PAGES * 4096;
			(*ret_proc.frame).mode = CpuMode::Machine as usize;
			(*ret_proc.frame).pid = ret_proc.pid as usize;
		}
		pl.push_back(ret_proc);
		// Now, we no longer need the owned Deque, so we hand it
		// back by replacing the PROCESS_LIST's None with the
		// Some(pl).
		unsafe { PROCESS_LIST.replace(pl); }
		my_pid
	}
	else {
		// TODO: When we get to multi-hart processing, we need to keep
		// trying to grab the process list. We can do this with an
		// atomic instruction. but right now, we're a single-processor
		// computer.
		0
	}
}

/// This is the same as the add_kernel_process function, except you can pass
/// arguments. Typically, this will be a memory address on the heap where
/// arguments can be found.
pub fn add_kernel_process_args(func: fn(args_ptr: usize), args: usize) -> u16 {
	// This is the Rust-ism that really trips up C++ programmers.
	// PROCESS_LIST is wrapped in an Option<> enumeration, which
	// means that the Option owns the Deque. We can only borrow from
	// it or move ownership to us. In this case, we choose the
	// latter, where we move ownership to us, add a process, and
	// then move ownership back to the PROCESS_LIST.
	// This allows mutual exclusion as anyone else trying to grab
	// the process list will get None rather than the Deque.
	if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
		// .take() will replace PROCESS_LIST with None and give
		// us the only copy of the Deque.
		let func_addr = func as usize;
		let func_vaddr = func_addr; //- 0x6000_0000;
			    // println!("func_addr = {:x} -> {:x}", func_addr, func_vaddr);
			    // We will convert NEXT_PID below into an atomic increment when
			    // we start getting into multi-hart processing. For now, we want
				// a process. Get it to work, then improve it!
		let my_pid = unsafe {NEXT_PID};
		let mut ret_proc = Process { frame:       zalloc(1) as *mut TrapFrame,
		                             stack:       zalloc(STACK_PAGES),
		                             pid:         my_pid,
		                             root:        zalloc(1) as *mut Table,
		                             state:       ProcessState::Running,
		                             data:        ProcessData::zero(),
		                             sleep_until: 0, };
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
		unsafe {
			(*ret_proc.frame).pc = func_vaddr;
			(*ret_proc.frame).regs[10] = args;
			(*ret_proc.frame).regs[2] = ret_proc.stack as usize + STACK_PAGES * 4096;
			(*ret_proc.frame).mode = CpuMode::Machine as usize;
			(*ret_proc.frame).pid = ret_proc.pid as usize;
		}
		pl.push_back(ret_proc);
		// Now, we no longer need the owned Deque, so we hand it
		// back by replacing the PROCESS_LIST's None with the
		// Some(pl).
		unsafe { PROCESS_LIST.replace(pl); }
		my_pid
	}
	else {
		// TODO: When we get to multi-hart processing, we need to keep
		// trying to grab the process list. We can do this with an
		// atomic instruction. but right now, we're a single-processor
		// computer.
		0
	}
}


/// This should only be called once, and its job is to create
/// the init process. Right now, this process is in the kernel,
/// but later, it should call the shell.
pub fn init() -> usize {
	unsafe {
		PROCESS_LIST = Some(VecDeque::with_capacity(15));
		// add_process_default(init_process);
		add_kernel_process(init_process);
		// Ugh....Rust is giving me fits over here!
		// I just want a memory address to the trap frame, but
		// due to the borrow rules of Rust, I'm fighting here. So,
		// instead, let's move the value out of PROCESS_LIST, get
		// the address, and then move it right back in.
		let pl = PROCESS_LIST.take().unwrap();
		let p = pl.front().unwrap().frame;
		// let frame = p as *const TrapFrame as usize;
		// println!("Init's frame is at 0x{:08x}", frame);
		// Put the process list back in the global.
		PROCESS_LIST.replace(pl);
		// Return the first instruction's address to execute.
		// Since we use the MMU, all start here.
		(*p).pc
	}
}

// Our process must be able to sleep, wait, or run.
// Running - means that when the scheduler finds this process, it can run it.
// Sleeping - means that the process is waiting on a certain amount of time.
// Waiting - means that the process is waiting on I/O
// Dead - We should never get here, but we can flag a process as Dead and clean
//        it out of the list later.
#[repr(u8)]
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
	frame:       *mut TrapFrame,
	stack:       *mut u8,
	pid:         u16,
	root:        *mut Table,
	state:       ProcessState,
	data:        ProcessData,
	sleep_until: usize,
}

impl Process {
	pub fn get_frame_address(&self) -> usize {
		self.frame as usize
	}

	pub fn get_frame(&mut self) -> *mut TrapFrame {
		self.frame
	}

	pub fn get_program_counter(&self) -> usize {
		unsafe { (*self.frame).pc }
	}

	pub fn get_table_address(&self) -> usize {
		self.root as usize
	}

	pub fn get_state(&self) -> &ProcessState {
		&self.state
	}

	pub fn set_state(&mut self, ps: ProcessState) {
		self.state = ps;
	}

	pub fn get_pid(&self) -> u16 {
		self.pid
	}

	pub fn get_sleep_until(&self) -> usize {
		self.sleep_until
	}

	pub fn set_sleep_until(&mut self, until: usize) {
		self.sleep_until = until;
	}

	pub fn new_default(func: fn()) -> Self {
		let func_addr = func as usize;
		let func_vaddr = func_addr; //- 0x6000_0000;
			    // println!("func_addr = {:x} -> {:x}", func_addr, func_vaddr);
			    // We will convert NEXT_PID below into an atomic increment when
			    // we start getting into multi-hart processing. For now, we want
			    // a process. Get it to work, then improve it!
		let mut ret_proc = Process { frame:       zalloc(1) as *mut TrapFrame,
		                             stack:       alloc(STACK_PAGES),
		                             pid:         unsafe { NEXT_PID },
		                             root:        zalloc(1) as *mut Table,
		                             state:       ProcessState::Running,
		                             data:        ProcessData::zero(),
		                             sleep_until: 0, };
		unsafe {
			satp_fence_asid(NEXT_PID as usize);
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
		let saddr = ret_proc.stack as usize;
		unsafe {
			(*ret_proc.frame).pc = func_vaddr;
			(*ret_proc.frame).regs[2] = STACK_ADDR + PAGE_SIZE * STACK_PAGES;
			(*ret_proc.frame).mode = CpuMode::User as usize;
			(*ret_proc.frame).pid = ret_proc.pid as usize;
		}
		// Map the stack on the MMU
		let pt;
		unsafe {
			pt = &mut *ret_proc.root;
			(*ret_proc.frame).satp =
				build_satp(SatpMode::Sv39, ret_proc.pid as usize, ret_proc.root as usize);
		}
		// We need to map the stack onto the user process' virtual
		// memory This gets a little hairy because we need to also map
		// the function code too.
		for i in 0..STACK_PAGES {
			let addr = i * PAGE_SIZE;
			map(pt, STACK_ADDR + addr, saddr + addr, EntryBits::UserReadWrite.val(), 0);
			// println!("Set stack from 0x{:016x} -> 0x{:016x}", STACK_ADDR + addr, saddr + addr);
		}
		// Map the program counter on the MMU and other bits
		for i in 0..=100 {
			let modifier = i * 0x1000;
			map(pt, func_vaddr + modifier, func_addr + modifier, EntryBits::UserReadWriteExecute.val(), 0);
		}
		// This is the make_syscall function
		// The reason we need this is because we're running a process
		// that is inside of the kernel. When we start loading from a block
		// devices, we can load the instructions anywhere in memory.
		for i in 0..=7 {
			let addr = 0x8000_0000 | i << 12;
			map(pt, addr, addr, EntryBits::UserReadExecute.val(), 0);
		}
		ret_proc
	}
}

impl Drop for Process {
	/// Since we're storing ownership of a Process in the linked list,
	/// we can cause it to deallocate automatically when it is removed.
	fn drop(&mut self) {
		// println!("Dropping process {}", self.get_pid());
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
		dealloc(self.frame as *mut u8);
	}
}

// The private data in a process contains information
// that is relevant to where we are, including the path
// and open file descriptors.
// We will allow dead code for now until we have a need for the
// private process data. This is essentially our resource control block (RCB).
#[allow(dead_code)]
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
