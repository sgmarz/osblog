// Steve Operating System
// Stephen Marz
// 21 Sep 2019
#![no_main]
#![no_std]
#![feature(panic_info_message,
           asm,
		   llvm_asm,
		   global_asm,
           allocator_api,
           alloc_error_handler,
           alloc_prelude,
		   const_raw_ptr_to_usize_cast,
		   lang_items)]

#[lang = "eh_personality"] extern fn eh_personality() {}

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
			llvm_asm!("wfi"::::"volatile");
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
// extern "C" {
// static TEXT_START: usize;
// static TEXT_END: usize;
// static DATA_START: usize;
// static DATA_END: usize;
// static RODATA_START: usize;
// static RODATA_END: usize;
// static BSS_START: usize;
// static BSS_END: usize;
// static KERNEL_STACK_START: usize;
// static KERNEL_STACK_END: usize;
// static HEAP_START: usize;
// static HEAP_SIZE: usize;
// }
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
	fn switch_to_user(frame: usize) -> !;
}

/// Switch to user is an assembly function that loads
/// a frame. Since it will jump to another program counter,
/// it will never return back here. We don't care if we leak
/// the stack, since we will recapture the stack during m_trap.
fn rust_switch_to_user(frame: usize) -> ! {
	unsafe {
		switch_to_user(frame);
	}
}
// ///////////////////////////////////
// / ENTRY POINT
// ///////////////////////////////////
#[no_mangle]
extern "C" fn kinit() {
	uart::Uart::new(0x1000_0000).init();
	page::init();
	kmem::init();
	process::init();
	// We lower the threshold wall so our interrupts can jump over it.
	// Any priority > 0 will be able to be "heard"
	plic::set_threshold(0);
	// VIRTIO = [1..8]
	// UART0 = 10
	// PCIE = [32..35]
	// Enable PLIC interrupts.
	for i in 1..=10 {
		plic::enable(i);
		plic::set_priority(i, 1);
	}
	// Set up virtio. This requires a working heap and page-grained allocator.
	virtio::probe();
	// Test the block driver!
	process::add_kernel_process(test::test_elf);
	// We schedule the next context switch using a multiplier of 1
	// Block testing code removed.
	trap::schedule_next_context_switch(1);
	rust_switch_to_user(sched::schedule());
	// switch_to_user will not return, so we should never get here
}
#[no_mangle]
extern "C" fn kinit_hart(_hartid: usize) {
	// We aren't going to do anything here until we get SMP going.
	// All non-0 harts initialize here.
}

// ///////////////////////////////////
// / RUST MODULES
// ///////////////////////////////////

pub mod assembly;
pub mod block;
pub mod cpu;
pub mod elf;
pub mod fs;
pub mod kmem;
pub mod minixfs;
pub mod page;
pub mod plic;
pub mod process;
pub mod rng;
pub mod sched;
pub mod syscall;
pub mod trap;
pub mod uart;
pub mod virtio;
pub mod test;


