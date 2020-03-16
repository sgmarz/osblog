// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::cpu::{CONTEXT_SWITCH_TIME, TrapFrame};
use crate::plic;
use crate::syscall::do_syscall;
use crate::sched::schedule;
use crate::rust_switch_to_user;

#[no_mangle]
/// The m_trap stands for "machine trap". Right now, we are handling
/// all traps at machine mode. In this mode, we can figure out what's
/// going on and send a trap where it needs to be. Remember, in machine
/// mode and in this trap, interrupts are disabled and the MMU is off.
extern "C" fn m_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     _status: usize,
                     frame: *mut TrapFrame)
                     -> usize
{
	// We're going to handle all traps in machine mode. RISC-V lets
	// us delegate to supervisor mode, but switching out SATP (virtual memory)
	// gets hairy.
	let is_async = {
		if cause >> 63 & 1 == 1 {
			true
		}
		else {
			false
		}
	};
	// The cause contains the type of trap (sync, async) as well as the cause
	// number. So, here we narrow down just the cause number.
	let cause_num = cause & 0xfff;
	let mut return_pc = epc;
	if is_async {
		// Asynchronous trap
		match cause_num {
			3 => {
				// Machine software
				println!("Machine software interrupt CPU #{}", hart);
			},
			7 => {
				// This is the context-switch timer.
				// We would typically invoke the scheduler here to pick another
				// process to run.
				// Machine timer
				// println!("CTX");
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			},
			11 => {
				// Machine external (interrupt from Platform Interrupt Controller (PLIC))
				// println!("Machine external interrupt CPU#{}", hart);
				// We will check the next interrupt. If the interrupt isn't available, this will
				// give us None. However, that would mean we got a spurious interrupt, unless we
				// get an interrupt from a non-PLIC source. This is the main reason that the PLIC
				// hardwires the id 0 to 0, so that we can use it as an error case.
				plic::handle_interrupt();
			},
			_ => {
				panic!("Unhandled async trap CPU#{} -> {}\n", hart, cause_num);
			}
		}
	}
	else {
		// Synchronous trap
		match cause_num {
			2 => {
				// Illegal instruction
				panic!("Illegal instruction CPU#{} -> 0x{:08x}: 0x{:08x}\n", hart, epc, tval);
				// We need while trues here until we have a functioning "delete from scheduler"
				// I use while true because Rust will warn us that it looks stupid.
				// This is what I want so that I remember to remove this and replace
				// them later.
				while true {}
			},
			8 => {
				// Environment (system) call from User mode
				// println!("E-call from User mode! CPU#{} -> 0x{:08x}", hart, epc);
				return_pc = do_syscall(return_pc, frame);
			},
			9 => {
				// Environment (system) call from Supervisor mode
				println!("E-call from Supervisor mode! CPU#{} -> 0x{:08x}", hart, epc);
				return_pc = do_syscall(return_pc, frame);
			},
			11 => {
				// Environment (system) call from Machine mode
				panic!("E-call from Machine mode! CPU#{} -> 0x{:08x}\n", hart, epc);
			},
			// Page faults
			12 => {
				// Instruction page fault
				println!("Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				// We need while trues here until we have a functioning "delete from scheduler"
				while true {}
				return_pc += 4;
			},
			13 => {
				// Load page fault
				println!("Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				// We need while trues here until we have a functioning "delete from scheduler"
				while true {}
				return_pc += 4;
			},
			15 => {
				// Store page fault
				println!("Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				// We need while trues here until we have a functioning "delete from scheduler"
				while true {}
				return_pc += 4;
			},
			_ => {
				panic!("Unhandled sync trap CPU#{} -> {}\n", hart, cause_num);
			}
		}
	};
	// Finally, return the updated program counter
	return_pc
}

pub const MMIO_MTIMECMP: *mut u64 = 0x0200_4000usize as *mut u64;
pub const MMIO_MTIME: *const u64 = 0x0200_BFF8 as *const u64;

pub fn schedule_next_context_switch(qm: u16) {
	// This is much too slow for normal operations, but it gives us
	// a visual of what's happening behind the scenes.
	unsafe {
		MMIO_MTIMECMP.write_volatile(MMIO_MTIME.read_volatile().wrapping_add(CONTEXT_SWITCH_TIME * qm as u64));
	}
}
