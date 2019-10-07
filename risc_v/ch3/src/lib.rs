// Steve Operating System
// Stephen Marz
// 21 Sep 2019
#![no_std]
#![feature(panic_info_message,asm,allocator_api,alloc_error_handler,alloc_prelude)]

#[macro_use]
extern crate alloc;

use alloc::prelude::v1::*;

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
extern "C"
fn abort() -> ! {
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

// ///////////////////////////////////
// / ENTRY POINT
// ///////////////////////////////////
#[no_mangle]
extern "C"
fn kmain() {
	// Main should initialize all sub-systems and get
	// ready to start scheduling. The last thing this
	// should do is start the timer.

	// Let's try using our newly minted UART by initializing it first.
	// The UART is sitting at MMIO address 0x1000_0000, so for testing
	// now, lets connect to it and see if we can initialize it and write
	// to it.
	let mut my_uart = uart::Uart::new(0x1000_0000);

	my_uart.init();

	// Now test println! macro!
	println!("This is my operating system!");
	println!("I'm so awesome. If you start typing something, I'll show you what you typed!");
	mem::init();
	mem::print_page_allocations();
	let root_ptr = mem::zalloc(1) as *mut mem::Table;
	let mut root = unsafe { root_ptr.as_mut().unwrap() };
	mem::map(&mut root, 0x7f2_2000_01f2, 0x8000_0ddd, mem::EntryBits::Read.val());
	let m = mem::walk(&root, 0x7f2_2000_0234).unwrap_or(0);
	mem::print_page_allocations();
	mem::unmap(&mut root);
	mem::print_page_allocations();
	mem::dealloc(root_ptr as *mut u8);
	mem::print_page_allocations();
	println!("Memory = 0x{:x}", m);
	// Create a new scope so that we can test the global allocator and deallocator
	{
		// We have the global allocator, so let's see if that works!
		let k: Box<u32> = Box::new(100);
		println!("Boxed value = {}", *k);
		// The following comes from the Rust documentation:
		// some bytes, in a vector
		let sparkle_heart = vec![240, 159, 146, 150];
		// We know these bytes are valid, so we'll use `unwrap()`.
		let sparkle_heart = String::from_utf8(sparkle_heart).unwrap();
		println!("String = {}", sparkle_heart);
		mem::print_page_allocations();
	}
	// The box inside of the scope above should be dropped when k goes
	// out of scope. The Drop trait for Box should call dealloc from the
	// global allocator.
	mem::print_page_allocations();

	// Now see if we can read stuff:
	// Usually we can use #[test] modules in Rust, but it would convolute the
	// task at hand. So, we'll just add testing snippets.
	loop {
		if let Some(c) = my_uart.get() {
			match c {
				8 => {
					// This is a backspace, so we essentially have
					// to write a space and backup again:
					print!("{}{}{}", 8 as char, ' ', 8 as char);
				},
				  10 | 13 => {
					  // Newline or carriage-return
					  println!();
				  },
				  0x1b => {
					  // Those familiar with ANSI escape sequences
					  // knows that this is one of them. The next
					  // thing we should get is the left bracket [
					  // These are multi-byte sequences, so we can take
					  // a chance and get from UART ourselves.
					  // Later, we'll button this up.
					  if let Some(next_byte) = my_uart.get() {
						  if next_byte == 91 {
							  // This is a right bracket! We're on our way!
							  if let Some(b) = my_uart.get() {
								  match b as char {
									  'A' => {
										  println!("That's the up arrow!");
									  },
									  'B' => {
										  println!("That's the down arrow!");
									  },
									  'C' => {
										  println!("That's the right arrow!");
									  },
									  'D' => {
										  println!("That's the left arrow!");
									  },
									  _ => {
										  println!("That's something else.....");
									  }
								  }
							  }
						  }
					  }
				  },
				  _ => {
					  print!("{}", c as char);
				  }
			}
		}
	}
}

// ///////////////////////////////////
// / RUST MODULES
// ///////////////////////////////////

pub mod uart;
pub mod mem;
