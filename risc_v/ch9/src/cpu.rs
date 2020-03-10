// cpu.rs
// CPU and CPU-related routines
// Also contains the kernel's trap frame
// Stephen Marz
// 14 October 2019

// The frequency of QEMU is 10 MHz
pub const FREQ: u64 = 10_000_000;
// Let's do this 250 times per second for switching
pub const CONTEXT_SWITCH_TIME: u64 = FREQ / 250;

/// In 64-bit mode, we're given three different modes for the MMU:
/// 0 - The MMU is off -- no protection and no translation PA = VA
/// 8 - This is Sv39 mode -- 39-bit virtual addresses
/// 9 - This is Sv48 mode -- 48-bit virtual addresses
#[repr(usize)]
pub enum SatpMode {
	Off = 0,
	Sv39 = 8,
	Sv48 = 9,
}

/// The trap frame is set into a structure
/// and packed into each hart's mscratch register.
/// This allows for quick reference and full
/// context switch handling.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
	pub regs:       [usize; 32], // 0 - 255
	pub fregs:      [usize; 32], // 256 - 511
	pub satp:       usize,       // 512 - 519
	pub pc:         usize,       // 520
	pub hartid:     usize,       // 528
}

/// Rust requires that we initialize our structures
/// because of the move semantics. What'll happen below
/// is Rust will construct a new TrapFrame and move it
/// out of the zero() function below. Rust contains two
/// different "selfs" where self can refer to the object
/// in memory or Self (capital S) which refers to the
/// data type of the structure. In the case below, this
/// is TrapFrame.
impl TrapFrame {
	pub const fn zero() -> Self {
		TrapFrame { regs:       [0; 32],
		            fregs:      [0; 32],
		            satp:       0,
		            pc:		    0,
		            hartid:     0, }
	}
}

/// The global kernel trap frame stores 8 separate
/// frames -- one per CPU hart. We will switch these
/// in and out and store a dormant trap frame with
/// the process itself.
pub static mut KERNEL_TRAP_FRAME: [TrapFrame; 8] =
	[TrapFrame::zero(); 8];

/// The SATP register contains three fields: mode, address space id, and
/// the first level table address (level 2 for Sv39). This function
/// helps make the 64-bit register contents based on those three
/// fields.
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
pub fn mie_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr $0, mie" :"=r"(rval));
		rval
	}
}

pub fn mie_write(val: usize) {
	unsafe {
		asm!("csrw mie, $0" :: "r"(val));
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

pub fn mepc_write(val: usize) {
	unsafe {
		asm!("csrw mepc, $0" :: "r"(val));
	}
}

pub fn mepc_read() -> usize {
	unsafe {
		let rval;
		asm!("csrr $0, mepc" :"=r"(rval));
		rval
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

/// Take a hammer to the page tables and synchronize
/// all of them. This essentially flushes the entire
/// TLB.
pub fn satp_fence(vaddr: usize, asid: usize) {
	unsafe {
		asm!("sfence.vma $0, $1" :: "r"(vaddr), "r"(asid));
	}
}

/// Synchronize based on the address space identifier
/// This allows us to fence a particular process rather
/// than the entire TLB.
/// The RISC-V documentation calls this a TLB flush +.
/// Since there are other memory routines involved, they
/// didn't call it a TLB flush, but it is much like
/// Intel/AMD's invtlb [] instruction.
pub fn satp_fence_asid(asid: usize) {
	unsafe {
		asm!("sfence.vma zero, $0" :: "r"(asid));
	}
}
