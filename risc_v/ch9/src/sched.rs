// sched.rs
// Simple process scheduler
// Stephen Marz
// 27 Dec 2019

use crate::{cpu::{build_satp, SatpMode},
            process::{ProcessState, PROCESS_LIST}};

pub fn schedule() -> (usize, usize, usize) {
	unsafe {
		if let Some(mut pl) = PROCESS_LIST.take() {
			pl.rotate_left(1);
			let mut frame_addr: usize = 0;
			let mut mepc: usize = 0;
			let mut satp: usize = 0;
			let mut pid: usize = 0;
			if let Some(prc) = pl.front() {
				match prc.get_state() {
					ProcessState::Running => {
						frame_addr =
							prc.get_frame_address();
						mepc = prc.get_program_counter();
						satp = prc.get_table_address();
						pid = prc.get_pid() as usize;
					},
					ProcessState::Sleeping => {},
					_ => {},
				}
			}
			// println!("Scheduling {}", pid);
			PROCESS_LIST.replace(pl);
			if frame_addr != 0 {
				// MODE 8 is 39-bit virtual address MMU
				// I'm using the PID as the address space
				// identifier to hopefully help with (not?)
				// flushing the TLB whenever we switch
				// processes.
				if satp != 0 {
					return (frame_addr,
					        mepc,
					        build_satp(
					                   SatpMode::Sv39,
					                   pid,
					                   satp,
					));
				}
				else {
					return (frame_addr, mepc, 0);
				}
			}
		}
	}
	(0, 0, 0)
}
