// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::cpu::TrapFrame;

#[no_mangle]
extern "C" fn s_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     stat: usize,
                     frame: &mut TrapFrame)
                     -> usize
{
	// Harts in supervisor mode with delegated traps will come here.
	// Right now, these are exceptions.
	println!("STRAP (cause: {} @ 0x{:x}) [cpu: {}]", cause, epc, hart);
	// If this is an exception, we will skip the faulting instruction. This is
	// dangerous, but we don't actually handle anything, yet.
	epc + 4
}

#[no_mangle]
extern "C" fn m_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     stat: usize,
                     frame: &mut TrapFrame)
                     -> usize
{
	// Only machine timers should come here. Everything else should be
	// brought to supervisor mode (s_trap). However, the software interrupt
	// and timer interrupts will trap to machine mode. Below (cause = 7) is
	// a timer interrupt.
	if cause == 0x8000_0000_0000_0007 {
		unsafe {
			let addr = 0x0200_4000 + hart * 8;
			let mtimecmp = addr as *mut u64;
			let mtime = 0x0200_bff8 as *const u64;
			mtimecmp.write_volatile(
			                        mtime.read_volatile()
			                        + 10_000_000,
			);
		}
		epc
	}
	else {
		panic!(
		       "Non-timer machine interrupt: 0x{:x} on hart {}",
		       cause, hart
		)
	}
}
