// test.rs

use crate::{kmem::{kfree, kmalloc},
            process::{Process,
                      NEXT_PID,
                      PROCESS_LIST,
                      PROCESS_STARTING_ADDR,
                      STACK_ADDR,
                      STACK_PAGES, ProcessState, ProcessData},
			syscall::syscall_fs_read};
use crate::page::{zalloc, Table, map,  EntryBits};
use crate::cpu::{memcpy, TrapFrame, CpuMode, satp_fence_asid, SatpMode, build_satp};

pub fn test_block() {
	// Let's test the block driver!
	let bytes_to_read = 1024 * 50;
	let buffer = kmalloc(bytes_to_read);
	let bytes_read = syscall_fs_read(8, 8, buffer, bytes_to_read as u32, 0);
	if bytes_read != 12288 {
		println!(
		         "Unable to load program at inode 8, which should be \
		          12,288 bytes, got {}",
		         bytes_read
		);
	}
	else {
		// Let's get this program running!
		let program_pages = (bytes_read / 4096) + 1;
		let my_pid = unsafe { NEXT_PID + 1 };
		unsafe {
			NEXT_PID += 1;
		}
		satp_fence_asid(my_pid as usize);
		let mut my_proc=
			Process { frame:       zalloc(1) as *mut TrapFrame,
			          stack:       zalloc(STACK_PAGES),
			          pid:         my_pid,
			          root:        zalloc(1) as *mut Table,
			          state:       ProcessState::Running,
			          data:        ProcessData::zero(),
					  sleep_until: 0,
					  program:	   zalloc(program_pages)
					 };
		// Map the program in the MMU.
		let ptr = my_proc.program;
		unsafe {
			memcpy(ptr, buffer, bytes_read);
		}
		let table = unsafe { my_proc.root.as_mut().unwrap() };
		for i in 0..program_pages {
			let vaddr = PROCESS_STARTING_ADDR + (i << 12);
			let paddr = ptr as usize + (i << 12);
			map(table, vaddr, paddr, EntryBits::UserReadWriteExecute.val(), 0);
		}
		// Map the stack
		let ptr = my_proc.stack as *mut u8;
		for i in 0..STACK_PAGES {
			let vaddr = STACK_ADDR + (i << 12);
			let paddr = ptr as usize + (i << 12);
			map(table, vaddr, paddr, EntryBits::UserReadWrite.val(), 0);
		}
		// Set everything up in the trap frame
		unsafe {
			(*my_proc.frame).pc = PROCESS_STARTING_ADDR;
			// Stack pointer
			(*my_proc.frame).regs[2] =
				STACK_ADDR as usize + STACK_PAGES * 4096;
			(*my_proc.frame).mode = CpuMode::User as usize;
			(*my_proc.frame).pid = my_proc.pid as usize;
		}
		unsafe {
			(*my_proc.frame).satp =
				build_satp(
				           SatpMode::Sv39,
				           my_proc.pid as usize,
				           my_proc.root as usize,
				);
		}
		if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
			println!("Added user process to the scheduler...get ready for take-off!");
			pl.push_back(my_proc);
			unsafe {
				PROCESS_LIST.replace(pl);
			}
		}
		else {
			println!("Unable to spawn process.");
			// Since my_proc couldn't enter the process list, it will
			// be dropped and all of the associated allocations will
			// be deallocated.

		}
	}
	println!();
	kfree(buffer);
}
