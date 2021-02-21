// cpu.rs
// CPU and CPU-related routines
// Also contains the kernel's trap frame
// Stephen Marz
// 14 October 2019

// The frequency of QEMU is 10 MHz
pub const FREQ: u64 = 10_000_000;
// Let's do this 250 times per second for switching
pub const CONTEXT_SWITCH_TIME: u64 = FREQ / 500;

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

#[repr(usize)]
pub enum CpuMode {
	User = 0,
	Supervisor = 1,
	Machine = 3,
}

#[repr(usize)]
pub enum Registers {
	Zero = 0,
	Ra,
	Sp,
	Gp,
	Tp,
	T0,
	T1,
	T2,
	S0,
	S1,
	A0, /* 10 */
	A1,
	A2,
	A3,
	A4,
	A5,
	A6,
	A7,
	S2,
	S3,
	S4, /* 20 */
	S5,
	S6,
	S7,
	S8,
	S9,
	S10,
	S11,
	T3,
	T4,
	T5, /* 30 */
	T6
}

// Floating point registers
#[repr(usize)]
pub enum FRegisters {
	Ft0,
	Ft1,
	Ft2,
	Ft3,
	Ft4,
	Ft5,
	Ft6,
	Ft7,
	Fs0,
	Fs1,
	Fa0, /* 10 */
	Fa1,
	Fa2,
	Fa3,
	Fa4,
	Fa5,
	Fa6,
	Fa7,
	Fs2,
	Fs3,
	Fs4, /* 20 */
	Fs5,
	Fs6,
	Fs7,
	Fs8,
	Fs9,
	Fs10,
	Fs11,
	Ft8,
	Ft9,
	Ft10, /* 30 */
	Ft11
}

/// The trap frame is set into a structure
/// and packed into each hart's mscratch register.
/// This allows for quick reference and full
/// context switch handling.
/// To make offsets easier, everything will be a usize (8 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
	pub regs:   [usize; 32], // 0 - 255
	pub fregs:  [usize; 32], // 256 - 511
	pub satp:   usize,       // 512 - 519
	pub pc:     usize,       // 520
	pub hartid: usize,       // 528
	pub qm:     usize,       // 536
	pub pid:    usize,       // 544
	pub mode:   usize,       // 552
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
	pub const fn new() -> Self {
		TrapFrame { regs:   [0; 32],
		            fregs:  [0; 32],
		            satp:   0,
		            pc:     0,
		            hartid: 0,
		            qm:     1,
		            pid:    0,
		            mode:   0, }
	}
}

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
		llvm_asm!("csrr $0, mhartid" :"=r"(rval));
		rval
	}
}
pub fn mie_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr $0, mie" :"=r"(rval));
		rval
	}
}

pub fn mie_write(val: usize) {
	unsafe {
		llvm_asm!("csrw mie, $0" :: "r"(val));
	}
}

pub fn mstatus_write(val: usize) {
	unsafe {
		llvm_asm!("csrw	mstatus, $0" ::"r"(val));
	}
}

pub fn mstatus_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr	$0, mstatus":"=r"(rval));
		rval
	}
}

pub fn stvec_write(val: usize) {
	unsafe {
		llvm_asm!("csrw	stvec, $0" ::"r"(val));
	}
}

pub fn stvec_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr	$0, stvec" :"=r"(rval));
		rval
	}
}

pub fn mscratch_write(val: usize) {
	unsafe {
		llvm_asm!("csrw	mscratch, $0" ::"r"(val));
	}
}

pub fn mscratch_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr	$0, mscratch" : "=r"(rval));
		rval
	}
}

pub fn mscratch_swap(to: usize) -> usize {
	unsafe {
		let from;
		llvm_asm!("csrrw	$0, mscratch, $1" : "=r"(from) : "r"(to));
		from
	}
}

pub fn sscratch_write(val: usize) {
	unsafe {
		llvm_asm!("csrw	sscratch, $0" ::"r"(val));
	}
}

pub fn sscratch_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr	$0, sscratch" : "=r"(rval));
		rval
	}
}

pub fn sscratch_swap(to: usize) -> usize {
	unsafe {
		let from;
		llvm_asm!("csrrw	$0, sscratch, $1" : "=r"(from) : "r"(to));
		from
	}
}

pub fn mepc_write(val: usize) {
	unsafe {
		llvm_asm!("csrw mepc, $0" :: "r"(val));
	}
}

pub fn mepc_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr $0, mepc" :"=r"(rval));
		rval
	}
}

pub fn sepc_write(val: usize) {
	unsafe {
		llvm_asm!("csrw sepc, $0" :: "r"(val));
	}
}

pub fn sepc_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr $0, sepc" :"=r"(rval));
		rval
	}
}

pub fn satp_write(val: usize) {
	unsafe {
		llvm_asm!("csrw satp, $0" :: "r"(val));
	}
}

pub fn satp_read() -> usize {
	unsafe {
		let rval;
		llvm_asm!("csrr $0, satp" :"=r"(rval));
		rval
	}
}

/// Take a hammer to the page tables and synchronize
/// all of them. This essentially flushes the entire
/// TLB.
pub fn satp_fence(vaddr: usize, asid: usize) {
	unsafe {
		llvm_asm!("sfence.vma $0, $1" :: "r"(vaddr), "r"(asid));
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
		llvm_asm!("sfence.vma zero, $0" :: "r"(asid));
	}
}

const MMIO_MTIME: *const u64 = 0x0200_BFF8 as *const u64;

pub fn get_mtime() -> usize {
	unsafe { (*MMIO_MTIME) as usize }
}

/// Copy one data from one memory location to another.
pub unsafe fn memcpy(dest: *mut u8, src: *const u8, bytes: usize) {
	for i in 0..bytes {
		dest.add(i).write(src.add(i).read());
	}
}

/// Dumps the registers of a given trap frame. This is NOT the
/// current CPU registers!
pub fn dump_registers(frame: *const TrapFrame) {
	print!("   ");
	for i in 1..32 {
		if i % 4 == 0 {
			println!();
			print!("   ");
		}
		print!("x{:2}:{:08x}   ", i, unsafe { (*frame).regs[i] });
	}
	println!();
}
