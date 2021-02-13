#![no_std]
#![feature(asm,panic_info_message,lang_items,start,global_asm,naked_functions)]
#[lang = "eh_personality"] extern fn eh_personality() {}
#[macro_export]
macro_rules! print
{
	($($args:tt)+) => ({
			use core::fmt::Write;
			let _ = write!(crate::syscall::Writer, $($args)+);
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
			asm!("wfi");
		}
	}
}

#[no_mangle]
#[naked]
extern "C" fn _start() {
	unsafe {
		asm!("la	gp, __global_pointer$", options(noreturn));
	}
}

#[no_mangle]
extern "C" fn rustmain() {
	println!("Hello, world!");
}
#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
	rustmain();
	0
}

pub mod syscall;
pub mod event;
pub mod drawing;
