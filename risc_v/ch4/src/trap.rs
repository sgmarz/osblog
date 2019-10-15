// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019


use crate::cpu::KernelTrapFrame;

#[no_mangle]
extern "C"
fn s_trap(epc: usize, tval: usize, cause: isize) -> usize {
	println!("STRAP (cause: 0x{:x} @ 0x{:x})", cause, epc);
	if cause < 0 {
		epc
	}
	else {
		epc  + 4
	}
}

#[no_mangle]
extern "C"
fn m_trap(epc: usize, tval: usize, cause: usize, hart: usize, stat: usize, frame: &mut KernelTrapFrame) -> usize {
	// println!("MTRAP ({}) (cause: 0x{:x} @ 0x{:x}) [0x{:x}]", hart, cause, epc, stat);
	// println!("Stack = {:p}", &frame.trap_stack);
	// Only machine timers should come here. Everything else should be brought to supervisor
	// mode (s_trap).
	if cause == 0x8000_0000_0000_0007 {
		unsafe {
			// let satp: usize = KERNEL_TABLE >> 12 | 8 << 60;
			// println!("cause: {}", cause & 0xff);
			// println!("Kernel table = 0x{:x}", KERNEL_TABLE);
			// asm!("csrw satp, $0" :: "r"(satp) :: "volatile");
			let mtimecmp = 0x0200_4000 as *mut u64;
			let mtime = 0x0200_bff8 as *const u64;
			mtimecmp.write_volatile(mtime.read_volatile() + 10_000_000);
			asm!("csrw sip, $0" ::"r"(2));
			// asm!("sfence.vma" :::: "volatile");
			// asm!("csrw mie, zero" :::: "volatile");
		}
		epc
	}
	else {
		panic!("Non-timer machine interrupt: 0x{:x} on hart {}", cause, hart)
	}
}