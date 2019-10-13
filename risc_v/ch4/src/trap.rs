// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

extern "C" {
	static KERNEL_TABLE: usize;
}

#[no_mangle]
extern "C"
fn s_trap(epc: usize, tval: usize, cause: usize) -> usize {
	println!("STRAP (cause: 0x{:x} @ 0x{:x})", cause, epc);
	unsafe {
		// Switch to kernel's page table.
		//          table / 4096         Sv39
		let satp = KERNEL_TABLE >> 12 | 8 << 60;
		asm!("csrw satp, $0" :: "r"(satp));
	}
	epc + 4
}

#[no_mangle]
extern "C"
fn m_trap(epc: usize, tval: usize, cause: usize, hart: usize) -> usize {
	println!("MTRAP (cause: 0x{:x} @ 0x{:x})", cause, epc);
	epc + 4
}