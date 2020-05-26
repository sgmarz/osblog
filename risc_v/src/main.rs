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
	process::add_kernel_process(test::test);
	// Get the GPU going
	gpu::init(6);
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
pub mod buffer;
pub mod cpu;
pub mod elf;
pub mod fs;
pub mod gpu;
pub mod input;
pub mod kmem;
pub mod lock;
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


