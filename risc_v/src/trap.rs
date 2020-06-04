// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::{cpu::{TrapFrame, CONTEXT_SWITCH_TIME},
            plic,
            process::delete_process,
            rust_switch_to_user,
            sched::schedule,
            syscall::do_syscall};

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
				// We will use this to awaken our other CPUs so they can process
				// processes.
				println!("Machine software interrupt CPU #{}", hart);
			}
			7 => {
				// This is the context-switch timer.
				// We would typically invoke the scheduler here to pick another
				// process to run.
				// Machine timer
				let new_frame = schedule();
				schedule_next_context_switch(1);
				if new_frame != 0 {
					rust_switch_to_user(new_frame);
				}
			}
			11 => {
				// Machine external (interrupt from Platform Interrupt Controller (PLIC))
				// println!("Machine external interrupt CPU#{}", hart);
				// We will check the next interrupt. If the interrupt isn't available, this will
				// give us None. However, that would mean we got a spurious interrupt, unless we
				// get an interrupt from a non-PLIC source. This is the main reason that the PLIC
				// hardwires the id 0 to 0, so that we can use it as an error case.
				plic::handle_interrupt();
			}
			_ => {
				panic!("Unhandled async trap CPU#{} -> {}\n", hart, cause_num);
			}
		}
	}
	else {
		// Synchronous trap
		match cause_num {
			2 => unsafe {
				// Illegal instruction
				println!("Illegal instruction CPU#{} -> 0x{:08x}: 0x{:08x}\n", hart, epc, tval);
				// We need while trues here until we have a functioning "delete from scheduler"
				// I use while true because Rust will warn us that it looks stupid.
				// This is what I want so that I remember to remove this and replace
				// them later.
				delete_process((*frame).pid as u16);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			3 => {
				// breakpoint
				println!("BKPT\n\n");
				return_pc += 2;
			}
			7 => unsafe {
				println!("Error with pid {}, at PC 0x{:08x}, mepc 0x{:08x}", (*frame).pid, (*frame).pc, epc);
				delete_process((*frame).pid as u16);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			8 | 9 | 11 => unsafe {
				// Environment (system) call from User, Supervisor, and Machine modes
				// println!("E-call from User mode! CPU#{} -> 0x{:08x}", hart, epc);
				do_syscall(return_pc, frame);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			// Page faults
			12 => unsafe {
				// Instruction page fault
				println!("Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				delete_process((*frame).pid as u16);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			13 => unsafe {
				// Load page fault
				println!("Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				delete_process((*frame).pid as u16);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			15 => unsafe {
				// Store page fault
				println!("Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				delete_process((*frame).pid as u16);
				let frame = schedule();
				schedule_next_context_switch(1);
				rust_switch_to_user(frame);
			}
			_ => {
				panic!(
				       "Unhandled sync trap {}. CPU#{} -> 0x{:08x}: 0x{:08x}\n",
				       cause_num, hart, epc, tval
				);
			}
		}
	};
	// Finally, return the updated program counter
	return_pc
}

pub const MMIO_MTIMECMP: *mut u64 = 0x0200_4000usize as *mut u64;
pub const MMIO_MTIME: *const u64 = 0x0200_BFF8 as *const u64;

pub fn schedule_next_context_switch(qm: u16) {
	unsafe {
		MMIO_MTIMECMP.write_volatile(MMIO_MTIME.read_volatile().wrapping_add(CONTEXT_SWITCH_TIME * qm as u64));
	}
}
