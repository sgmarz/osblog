// sched.rs
// Simple process scheduler
// Stephen Marz
// 27 Dec 2019

use crate::process::{ProcessState, PROCESS_LIST};

pub fn schedule() -> usize {
	let mut frame_addr: usize = 0x1111;
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			let mut done = false;
			while !done {
				pl.rotate_left(1);
				// let mut mepc: usize = 0;
				// let mut satp: usize = 0;
				// let mut pid: usize = 0;
				if let Some(prc) = pl.front() {
					match prc.get_state() {
						ProcessState::Running => {
							frame_addr =
								prc.get_frame_address();
							done = true;
							// println!("Process is running on frame 0x{:x}", frame_addr);
							// satp = prc.get_table_address();
							// pid = prc.get_pid() as usize;
						},
						ProcessState::Sleeping => {},
						_ => {},
					}
				}
			}
			PROCESS_LIST.replace(pl);
		}
		else {
			println!("could not take process list");
		}
	}
	frame_addr
}
