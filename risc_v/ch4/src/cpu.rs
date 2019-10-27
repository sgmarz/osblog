// cpu.rs
// CPU and CPU-related routines
// Also contains the kernel's trap frame
// Stephen Marz
// 14 October 2019

use core::ptr::null_mut;

#[repr(usize)]
pub enum SatpMode {
	Off = 0,
	Sv39 = 8,
	Sv48 = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelTrapFrame {
	pub regs:       [usize; 32], // 0 - 255
	pub fregs:      [usize; 32], // 256 - 511
	pub satp:       usize,       // 512 - 519
	pub trap_stack: *mut u8,     // 520
	pub hartid:     usize,       // 528
}

impl KernelTrapFrame {
	pub const fn zero() -> Self {
		KernelTrapFrame { regs:       [0; 32],
		                  fregs:      [0; 32],
		                  satp:       0,
		                  trap_stack: null_mut(),
		                  hartid:     0, }
	}
}

pub static mut KERNEL_TRAP_FRAME: [KernelTrapFrame; 8] =
	[KernelTrapFrame::zero(); 8];

pub const fn build_satp(mode: SatpMode, asid: usize, addr: usize) -> usize {
	(mode as usize) << 60
	| (asid & 0xffff) << 44
	| (addr >> 12) & 0xff_ffff_ffff
}

pub fn mhartid_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr $0, mhartid" :"=r"(rval));
		rval
	}
}

pub fn mstatus_write(val: usize) {
	unsafe {
		asm!("csrw	mstatus, $0" ::"r"(val));
	}
}

pub fn mstatus_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr	$0, mstatus":"=r"(rval));
		rval
	}
}

pub fn stvec_write(val: usize) {
	unsafe {
		asm!("csrw	stvec, $0" ::"r"(val));
	}
}

pub fn stvec_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr	$0, stvec" :"=r"(rval));
		rval
	}
}

pub fn mscratch_write(val: usize) {
	unsafe {
		asm!("csrw	mscratch, $0" ::"r"(val));
	}
}

pub fn mscratch_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr	$0, mscratch" : "=r"(rval));
		rval
	}
}

pub fn mscratch_swap(to: usize) -> usize {
	unsafe {
		let from;
		asm!("csrrw	$0, mscratch, $1" : "=r"(from) : "r"(to));
		from
	}
}

pub fn sscratch_write(val: usize) {
	unsafe {
		asm!("csrw	sscratch, $0" ::"r"(val));
	}
}

pub fn sscratch_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr	$0, sscratch" : "=r"(rval));
		rval
	}
}

pub fn sscratch_swap(to: usize) -> usize {
	unsafe {
		let from;
		asm!("csrrw	$0, sscratch, $1" : "=r"(from) : "r"(to));
		from
	}
}

pub fn sepc_write(val: usize) {
	unsafe {
		asm!("csrw sepc, $0" :: "r"(val));
	}
}

pub fn sepc_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr $0, sepc" :"=r"(rval));
		rval
	}
}

pub fn satp_write(val: usize) {
	unsafe {
		asm!("csrw satp, $0" :: "r"(val));
	}
}

pub fn satp_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr $0, satp" :"=r"(rval));
		rval
	}
}

pub fn satp_fence(vaddr: usize, asid: usize) {
	unsafe {
		asm!("sfence.vma $0, $1" :: "r"(vaddr), "r"(asid));
	}
}

pub fn satp_fence_asid(asid: usize) {
	unsafe {
		asm!("sfence.vma zero, $0" :: "r"(asid));
	}
}
