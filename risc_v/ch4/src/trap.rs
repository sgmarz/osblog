// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::cpu::TrapFrame;

#[no_mangle]
extern "C" fn m_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     status: usize,
                     frame: &mut TrapFrame)
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
	let return_pc = if is_async {
		// Asynchronous trap
		match cause_num {
			3 => {
				// Machine software
				epc
			},
			7 => {
				// Machine timer
				epc
			},
			11 => {
				// Machine external (interrupt from Platform Interrupt Controller (PLIC))
				epc
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
			},
			// Page faults
			12 => {
				// Instruction page fault
				println!("Instruction page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				epc + 4
			},
			13 => {
				// Load page fault
				println!("Load page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				epc + 4
			},
			15 => {
				// Store page fault
				println!("Store page fault CPU#{} -> 0x{:08x}: 0x{:08x}", hart, epc, tval);
				epc + 4
			},
			_ => {
				panic!("Unhandled sync trap CPU#{} -> {}\n", hart, cause_num);
			}
		}
	};
	// Finally, return the updated program counter
	return_pc
}
