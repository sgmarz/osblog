// Steve Operating System
// Stephen Marz
// 21 Sep 2019
#![no_std]
#![feature(panic_info_message,
           asm,
           allocator_api,
           alloc_error_handler,
           alloc_prelude,
           const_raw_ptr_to_usize_cast)]

// #[macro_use]
extern crate alloc;
// This is experimental and requires alloc_prelude as a feature
// use alloc::prelude::v1::*;

// ///////////////////////////////////
// / RUST MACROS
// ///////////////////////////////////
#[macro_export]
macro_rules! print
{
	($($args:tt)+) => ({
			use core::fmt::Write;
			let _ = write!(crate::uart::Uart::new(0x1000_0000), $($args)+);
			});
}
#[macro_export]
macro_rules! println
{
	() => ({
		   print!("\r\n")
		   });
	($fmt:expr) => ({
			print!(concat!($fmt, "\r\n"))
			});
	($fmt:expr, $($args:tt)+) => ({
			print!(concat!($fmt, "\r\n"), $($args)+)
			});
}

// ///////////////////////////////////
// / LANGUAGE STRUCTURES / FUNCTIONS
// ///////////////////////////////////
#[no_mangle]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	print!("Aborting: ");
	if let Some(p) = info.location() {
		println!(
		         "line {}, file {}: {}",
		         p.line(),
		         p.file(),
		         info.message().unwrap()
		);
	}
	else {
		println!("no information available.");
	}
	abort();
}
#[no_mangle]
extern "C" fn abort() -> ! {
	loop {
		unsafe {
			asm!("wfi"::::"volatile");
		}
	}
}

// ///////////////////////////////////
// / CONSTANTS
// ///////////////////////////////////
// const STR_Y: &str = "\x1b[38;2;79;221;13m✓\x1b[m";
// const STR_N: &str = "\x1b[38;2;221;41;13m✘\x1b[m";

// The following symbols come from asm/mem.S. We can use
// the symbols directly, but the address of the symbols
// themselves are their values, which can cause issues.
// Instead, I created doubleword values in mem.S in the .rodata and .data
// sections.
/*
extern "C" {
	static TEXT_START: usize;
	static TEXT_END: usize;
	static DATA_START: usize;
	static DATA_END: usize;
	static RODATA_START: usize;
	static RODATA_END: usize;
	static BSS_START: usize;
	static BSS_END: usize;
	static KERNEL_STACK_START: usize;
	static KERNEL_STACK_END: usize;
	static HEAP_START: usize;
	static HEAP_SIZE: usize;
}
*/
/// Identity map range
/// Takes a contiguous allocation of memory and maps it using PAGE_SIZE
/// This assumes that start <= end
pub fn id_map_range(root: &mut page::Table,
                    start: usize,
                    end: usize,
                    bits: i64)
{
	let mut memaddr = start & !(page::PAGE_SIZE - 1);
	let num_kb_pages =
		(page::align_val(end, 12) - memaddr) / page::PAGE_SIZE;

	// I named this num_kb_pages for future expansion when
	// I decide to allow for GiB (2^30) and 2MiB (2^21) page
	// sizes. However, the overlapping memory regions are causing
	// nightmares.
	for _ in 0..num_kb_pages {
		page::map(root, memaddr, memaddr, bits, 0);
		memaddr += 1 << 12;
	}
}
extern "C" {
	fn switch_to_user(frame: usize, mepc: usize, satp: usize) -> !;
}
// ///////////////////////////////////
// / ENTRY POINT
// ///////////////////////////////////
#[no_mangle]
extern "C" fn kinit() {
	uart::Uart::new(0x1000_0000).init();
	page::init();
	kmem::init();
	let ret = process::init();
	println!("Init process created at address 0x{:08x}", ret);
	// We lower the threshold wall so our interrupts can jump over it.
	plic::set_threshold(0);
	// VIRTIO = [1..8]
	// UART0 = 10
	// PCIE = [32..35]
	// Enable the UART interrupt.
	plic::enable(10);
	plic::set_priority(10, 1);
	println!("UART interrupts have been enabled and are awaiting your command.");
	println!("Getting ready for first process.");
	println!("Issuing the first context-switch timer.");
	unsafe {
		let mtimecmp = 0x0200_4000 as *mut u64;
		let mtime = 0x0200_bff8 as *const u64;
		// The frequency given by QEMU is 10_000_000 Hz, so this sets
		// the next interrupt to fire one second from now.
		// mtimecmp.write_volatile(mtime.read_volatile() + 1_000_000);
	}
	sched::schedule();
	// When we return, we put the return value into mepc and start there. This
	// should be init's starting point.
	// ret
}
#[no_mangle]
extern "C" fn kinit_hart(hartid: usize) {
	// All non-0 harts initialize here.
	unsafe {
		// We have to store the kernel's table. The tables will be moved
		// back and forth between the kernel's table and user
		// applicatons' tables.
		cpu::mscratch_write(
		                    (&mut cpu::KERNEL_TRAP_FRAME[hartid]
		                     as *mut cpu::TrapFrame)
		                    as usize,
		);
		// Copy the same mscratch over to the supervisor version of the
		// same register.
		cpu::sscratch_write(cpu::mscratch_read());
		cpu::KERNEL_TRAP_FRAME[hartid].hartid = hartid;
		// We can't do the following until zalloc() is locked, but we
		// don't have locks, yet :( cpu::KERNEL_TRAP_FRAME[hartid].satp
		// = cpu::KERNEL_TRAP_FRAME[0].satp;
		// cpu::KERNEL_TRAP_FRAME[hartid].trap_stack = page::zalloc(1);
	}
}

// ///////////////////////////////////
// / RUST MODULES
// ///////////////////////////////////

pub mod cpu;
pub mod kmem;
pub mod page;
pub mod plic;
pub mod process;
pub mod sched;
pub mod syscall;
pub mod trap;
pub mod uart;
