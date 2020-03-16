// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::cpu::TrapFrame;
use crate::{plic, uart};
use crate::syscall::do_syscall;
use crate::sched::schedule;

extern "C" {
	fn switch_to_user(frame: usize, mepc: usize, satp: usize) -> !;
}

#[no_mangle]
/// The m_trap stands for "machine trap". Right now, we are handling
/// all traps at machine mode. In this mode, we can figure out what's
/// going on and send a trap where it needs to be. Remember, in machine
/// mode and in this trap, interrupts are disabled and the MMU is off.
extern "C" fn m_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     status: usize,
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
				println!("Machine software interrupt CPU#{}", hart);
			},
			7 => unsafe {
				// This is the context-switch timer.
				// We would typically invoke the scheduler here to pick another
				// process to run.
				// Machine timer
				// println!("CTX");
				let (frame, mepc, satp) = schedule();
				let mtimecmp = 0x0200_4000 as *mut u64;
				let mtime = 0x0200_bff8 as *const u64;
				// The frequency given by QEMU is 10_000_000 Hz, so this sets
				// the next interrupt to fire one second from now.
				// This is much too slow for normal operations, but it gives us
				// a visual of what's happening behind the scenes.
				mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);
				unsafe {
					switch_to_user(frame, mepc, satp);
				}
			},
			11 => {
				// Machine external (interrupt from Platform Interrupt Controller (PLIC))
				// println!("Machine external interrupt CPU#{}", hart);
				// We will check the next interrupt. If the interrupt isn't available, this will
				// give us None. However, that would mean we got a spurious interrupt, unless we
				// get an interrupt from a non-PLIC source. This is the main reason that the PLIC
				// hardwires the id 0 to 0, so that we can use it as an error case.
				if let Some(interrupt) = plic::next() {
					// If we get here, we've got an interrupt from the claim register. The PLIC will
					// automatically prioritize the next interrupt, so when we get it from claim, it
					// will be the next in priority order.
					match interrupt {
						10 => { // Interrupt 10 is the UART interrupt.
							// We would typically set this to be handled out of the interrupt context,
							// but we're testing here! C'mon!
							// We haven't yet used the singleton pattern for my_uart, but remember, this
							// just simply wraps 0x1000_0000 (UART).
							let mut my_uart = uart::Uart::new(0x1000_0000);
							// If we get here, the UART better have something! If not, what happened??
							if let Some(c) = my_uart.get() {
								// If you recognize this code, it used to be in the lib.rs under kmain(). That
								// was because we needed to poll for UART data. Now that we have interrupts,
								// here it goes!
								match c {
									8 => {
										// This is a backspace, so we
										// essentially have to write a space and
										// backup again:
										print!("{} {}", 8 as char, 8 as char);
									},
									10 | 13 => {
										// Newline or carriage-return
										println!();
									},
									_ => {
										print!("{}", c as char);
									},
								}
							}
					
						},
						// Non-UART interrupts go here and do nothing.
						_ => {
							println!("Non-UART external interrupt: {}", interrupt);
						}
					}
					// We've claimed it, so now say that we've handled it. This resets the interrupt pending
					// and allows the UART to interrupt again. Otherwise, the UART will get "stuck".
					plic::complete(interrupt);
				}
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
