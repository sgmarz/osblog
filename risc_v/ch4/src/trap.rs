// trap.rs
// Trap routines
// Stephen Marz
// 10 October 2019

extern "C" {
	static KERNEL_TABLE: usize;
}

#[no_mangle]
extern "C"
fn s_trap(epc: usize, tval: usize, cause: isize) -> usize {
	println!("STRAP (cause: 0x{:x} @ 0x{:x})", cause, epc);
	unsafe {
		// Switch to kernel's page table.
		//          table / 4096         Sv39
		let satp = KERNEL_TABLE >> 12 | 8 << 60;
		asm!("csrw satp, $0" :: "r"(satp));
	}
	if cause < 0 {
		epc
	}
	else {
		epc  + 4
	}
}

#[no_mangle]
extern "C"
fn m_trap(epc: usize, tval: usize, cause: isize, hart: usize, stat: usize) -> usize {
	println!("MTRAP ({}) (cause: 0x{:x} @ 0x{:x}) [0x{:x}]", hart, cause, epc, stat);
	unsafe {
		if cause < 0 {
			// Asynchronous
			match cause & 0xff {
				4 | 5 | 7 => {
					let satp: usize = KERNEL_TABLE >> 12 | 8 << 60;
					println!("Kernel table = 0x{:x}", KERNEL_TABLE);
					// asm!("csrw satp, $0" :: "r"(satp) :: "volatile");
					// asm!("sfence.vma" :::: "volatile");
					// asm!("csrw mie, zero" :::: "volatile");
				},
				_ => { println!("Async cause\n"); }
			}

		}
		else {
			match cause {
				2 => {
					panic!("Illegal instruction");
				},
				12 => {
					panic!("Instruction page fault.");
				},
				13 => {
					panic!("Load page fault.");
				},
				_ => { println!("Sync cause\n"); }
			}
		}
	}
	epc
}