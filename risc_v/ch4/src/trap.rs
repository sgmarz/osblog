// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

use crate::cpu::KernelTrapFrame;

#[no_mangle]
extern "C" fn s_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     stat: usize,
                     frame: &mut KernelTrapFrame)
                     -> usize
{
	println!("STRAP (cause: {} @ 0x{:x}) [cpu: {}]", cause, epc, hart);
	epc + 4
}

#[no_mangle]
extern "C" fn m_trap(epc: usize,
                     tval: usize,
                     cause: usize,
                     hart: usize,
                     stat: usize,
                     frame: &mut KernelTrapFrame)
                     -> usize
{
	// println!("MTRAP ({}) (cause: 0x{:x} @ 0x{:x}) [0x{:x}]", hart, cause,
	// epc, stat); println!("Stack = {:p}", &frame.trap_stack);
	// Only machine timers should come here. Everything else should be
	// brought to supervisor mode (s_trap).
	if cause == 0x8000_0000_0000_0007 {
		unsafe {
			let addr = 0x0200_4000 + hart * 8;
			let mtimecmp = addr as *mut u64;
			let mtime = 0x0200_bff8 as *const u64;
			mtimecmp.write_volatile(
			                        mtime.read_volatile()
			                        + 10_000_000,
			);
			asm!("csrw sip, $0" ::"r"(2));
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
