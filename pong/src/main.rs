#![no_std]
#![feature(asm, panic_info_message, lang_items, start, global_asm)]

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

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
		println!("line {}, file {}: {}", p.line(), p.file(), info.message().unwrap());
	} else {
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

global_asm!(include_str!("start.S"));

const MAX_EVENTS: usize = 25;
const GAME_FRAME_TIMER: usize = 1000;

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
	use drawing::Framebuffer;
	use drawing::Pixel;

	let ufb = syscall::get_fb(6) as *mut Pixel;
	let mut fb = Framebuffer::new(ufb);
	let background_color = drawing::Pixel::new(25, 36, 100);
	let mut event_list = [event::Event::empty(); MAX_EVENTS];
	let event_list_ptr = event_list.as_mut_ptr();

	let player_color = drawing::Pixel::new(255, 0, 0);
	let npc_color = drawing::Pixel::new(0, 255, 0);
	let ball_color = drawing::Pixel::new(255, 255, 255);

	let mut game = pong::Pong::new(player_color, npc_color, ball_color, background_color);
	'gameloop: loop {
		// handle mouse buttons and keyboard inputs
		// println!("Try get keys");
		let num_events = syscall::get_keys(event_list_ptr, MAX_EVENTS);
		for e in 0..num_events {
			let ref ev = event_list[e];
			// println!("Key {}  Value {}", ev.code, ev.value);
			// Value = 1 if key is PRESSED or 0 if RELEASED
			match ev.code {
				event::KEY_Q => break 'gameloop,
				event::KEY_R => game.reset(),
				event::KEY_W | event::KEY_UP => game.move_player(-20),
				event::KEY_S | event::KEY_DOWN => game.move_player(20),
				event::KEY_SPACE => if ev.value == 1 { 
					game.toggle_pause();
					if game.is_paused() {
						println!("GAME PAUSED");
					}
					else {
						println!("GAME UNPAUSED")
					}
				},
				_ => {}
			}
		}
		game.advance_frame();
		game.draw(&mut fb);
		syscall::inv_rect(6, 0, 0, 640, 480);
		syscall::sleep(GAME_FRAME_TIMER);
	}
	println!("Goodbye :)");
	0
}

pub mod drawing;
pub mod event;
pub mod pong;
pub mod syscall;
