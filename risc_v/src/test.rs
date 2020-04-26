// test.rs

use crate::{cpu::{build_satp,
                  memcpy,
                  satp_fence_asid,
                  CpuMode,
                  SatpMode,
                  TrapFrame},
            kmem::{kfree, kmalloc},
            page::{map, zalloc, EntryBits, Table, PAGE_SIZE},
            process::{Process,
                      ProcessData,
                      ProcessState,
                      NEXT_PID,
                      PROCESS_LIST,
                      PROCESS_STARTING_ADDR,
                      STACK_ADDR,
                      STACK_PAGES},
			syscall::syscall_fs_read};
use crate::elf;
use crate::fs::BlockBuffer;

pub fn test_block() {
	// The bytes to read would usually come from the inode, but we are in an
	// interrupt context right now, so we cannot pause. Usually, this would be done
	// by an exec system call.
	let bytes_to_read = 1024 * 50;
	let buffer = kmalloc(bytes_to_read);
	// Read the file from the disk.
	let bytes_read = syscall_fs_read(8, 8, buffer, bytes_to_read as u32, 0);
	// After compiling our program, I manually looked and saw it was 12,288
	// bytes. So, to make sure we got the right one, I do a manual check
	// here.
	if bytes_read != 12288 {
		println!(
		         "Unable to load program at inode 8, which should be \
		          12,288 bytes, got {}",
		         bytes_read
		);
	}
	else {
		// Let's get this program running!
		// Everything is "page" based since we're going to map pages to
		// user space. So, we need to know how many program pages we
		// need. Each page is 4096 bytes.
		let program_pages = (bytes_read / PAGE_SIZE) + 1;
		let my_pid = unsafe { NEXT_PID + 1 };
		unsafe {
			NEXT_PID += 1;
		}
		let mut my_proc =
			Process { frame:       zalloc(1) as *mut TrapFrame,
			          stack:       zalloc(STACK_PAGES),
			          pid:         my_pid,
			          root:        zalloc(1) as *mut Table,
			          state:       ProcessState::Running,
			          data:        ProcessData::zero(),
			          sleep_until: 0,
			          program:     zalloc(program_pages), };
		// Map the program in the MMU.
		let ptr = my_proc.program;
		unsafe {
			memcpy(ptr, buffer, bytes_read);
		}
		let table = unsafe { my_proc.root.as_mut().unwrap() };
		// This will map all of the program pages. Notice that in linker.lds in userspace
		// we set the entry point address to 0x2000_0000. This is the same address as
		// PROCESS_STARTING_ADDR, and they must match.
		for i in 0..program_pages {
			let vaddr = PROCESS_STARTING_ADDR + i * PAGE_SIZE;
			let paddr = ptr as usize + i * PAGE_SIZE;
			// We don't have an ELF loader yet, so we're loading raw binaries into memory. Since
			// it is a flat binary, all .data, .rodata, and .bss sections get wrapped into
			// the .text section. Normally, we don't want the .text section to be writeable,
			// however because of this "flattening", we don't have a choice.
			// Notice that USER shows up here. Since we're running in user mode, this bit MUST
			// BE SET! Otherwise, we'll get a page fault from the beginning.
			map(
			    table,
			    vaddr,
			    paddr,
			    EntryBits::UserReadWriteExecute.val(),
			    0,
			);
		}
		// Map the stack
		let ptr = my_proc.stack as *mut u8;
		for i in 0..STACK_PAGES {
			let vaddr = STACK_ADDR + i * PAGE_SIZE;
			let paddr = ptr as usize + i * PAGE_SIZE;
			// We create the stack. We don't load a stack from the disk. This is why I don't
			// need to make the stack executable.
			map(
			    table,
			    vaddr,
			    paddr,
			    EntryBits::UserReadWrite.val(),
			    0,
			);
		}
		// Set everything up in the trap frame
		unsafe {
			// The program counter is a virtual memory address and is loaded into mepc
			// when we execute mret.
			(*my_proc.frame).pc = PROCESS_STARTING_ADDR;
			// Stack pointer. The stack starts at the bottom and works its way up, so we have to
			// set the stack pointer to the bottom.
			(*my_proc.frame).regs[2] =
				STACK_ADDR as usize + STACK_PAGES * PAGE_SIZE;
			// USER MODE! This is how we set what'll go into mstatus when we run the process.
			(*my_proc.frame).mode = CpuMode::User as usize;
			(*my_proc.frame).pid = my_proc.pid as usize;
			// The SATP register is used for the MMU, so we need to
			// map our table into that register. The switch_to_user
			// function will load .satp into the actual register
			// when the time comes.
			(*my_proc.frame).satp =
				build_satp(
				           SatpMode::Sv39,
				           my_proc.pid as usize,
				           my_proc.root as usize,
				);
		}
		// We don't reuse PIDs, so this really shouldn't matter.
		satp_fence_asid(my_pid as usize);
		// I took a different tact here than in process.rs. In there I created the process
		// while holding onto the process list. It doesn't really matter since this is synchronous,
		// but it might get dicey 
		if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
			println!(
			         "Added user process to the scheduler...get \
			          ready for take-off!"
			);
			// As soon as we push this process on the list, it'll be schedule-able.
			pl.push_back(my_proc);
			unsafe {
				PROCESS_LIST.replace(pl);
			}
		}
		else {
			println!("Unable to spawn process.");
			// Since my_proc couldn't enter the process list, it
			// will be dropped and all of the associated allocations
			// will be deallocated.
		}
	}
	println!();
	kfree(buffer);
}

/// Test block will load raw binaries into memory to execute them. This function
/// will load ELF files and try to execute them.
pub fn test_elf() {
	// The bytes to read would usually come from the inode, but we are in an
	// interrupt context right now, so we cannot pause. Usually, this would be done
	// by an exec system call.
	let files_inode = 25u32;
	let files_size = 14304;
	let bytes_to_read = 1024 * 50;
	let mut buffer = BlockBuffer::new(bytes_to_read);
	// Read the file from the disk. I got the inode by mounting
	// the harddrive as a loop on Linux and stat'ing the inode.
	let bytes_read = syscall_fs_read(8, files_inode, buffer.get_mut(), bytes_to_read as u32, 0);
	// After compiling our program, I manually looked and saw it was 18,360
	// bytes. So, to make sure we got the right one, I do a manual check
	// here.
	if bytes_read != files_size {
		println!(
		         "Unable to load program at inode {}, which should be \
				  {} bytes, got {}",
				  files_inode,
				  files_size,
		         bytes_read
		);
		return;
	}
	// Let's get this program running!
	// Everything is "page" based since we're going to map pages to
	// user space. So, we need to know how many program pages we
	// need. Each page is 4096 bytes.
	let program_pages = (bytes_read / PAGE_SIZE) + 1;
	let my_pid = unsafe { NEXT_PID + 1 };
	let elf_hdr;
	unsafe {
		NEXT_PID += 1;
		// Load the ELF
		elf_hdr = (buffer.get() as *const elf::Header).as_ref().unwrap();
	}
	if elf_hdr.magic != elf::MAGIC {
		println!("ELF magic didn't match.");
		return;
	}
	if elf_hdr.machine != elf::MACHINE_RISCV {
		println!("ELF loaded is not RISC-V.");
		return;
	}
	if elf_hdr.obj_type != elf::TYPE_EXEC {
		println!("ELF is not an executable.");
		return;
	}
	let mut my_proc =
		Process { frame:       zalloc(1) as *mut TrapFrame,
					stack:       zalloc(STACK_PAGES),
					pid:         my_pid,
					root:        zalloc(1) as *mut Table,
					state:       ProcessState::Running,
					data:        ProcessData::zero(),
					sleep_until: 0,
					program:     zalloc(program_pages), };
	// Map the program in the MMU.
	let ptr = my_proc.program;
	let table = unsafe { my_proc.root.as_mut().unwrap() };
	unsafe {
		let ph_tab = buffer.get().add(elf_hdr.phoff) as *const elf::ProgramHeader;
		for i in 0..elf_hdr.phnum as usize {
			let ph = ph_tab.add(i).as_ref().unwrap();
			if ph.seg_type != elf::PH_SEG_TYPE_LOAD {
				continue;
			}
			if ph.memsz == 0 {
				continue;
			}
			memcpy(ptr.add(ph.off), buffer.get().add(ph.off), ph.memsz);
			let pages = ph.memsz / PAGE_SIZE + 1;
			let mut bits = EntryBits::User.val();
			// This sucks, but we check each bit in the flags to see if
			// we need to add it to the PH permissions.
			if ph.flags & elf::PROG_EXECUTE != 0 {
				bits |= EntryBits::Execute.val();
			}
			if ph.flags & elf::PROG_READ != 0 {
				bits |= EntryBits::Read.val();
			}
			if ph.flags & elf::PROG_WRITE != 0 {
				bits |= EntryBits::Write.val();
			}
			for i in 0..pages {
				let vaddr = ph.vaddr + i * PAGE_SIZE;
				let paddr = ptr as usize + i * PAGE_SIZE;
				map(
					table,
					vaddr,
					paddr,
					bits,
					0,
				);
			}
		}
	}
	// This will map all of the program pages. Notice that in linker.lds in userspace
	// we set the entry point address to 0x2000_0000. This is the same address as
	// PROCESS_STARTING_ADDR, and they must match.
	// Map the stack
	let ptr = my_proc.stack as *mut u8;
	for i in 0..STACK_PAGES {
		let vaddr = STACK_ADDR + i * PAGE_SIZE;
		let paddr = ptr as usize + i * PAGE_SIZE;
		// We create the stack. We don't load a stack from the disk. This is why I don't
		// need to make the stack executable.
		map(
			table,
			vaddr,
			paddr,
			EntryBits::UserReadWrite.val(),
			0,
		);
	}
	// Set everything up in the trap frame
	unsafe {
		// The program counter is a virtual memory address and is loaded into mepc
		// when we execute mret.
		(*my_proc.frame).pc = elf_hdr.entry_addr;
		// Stack pointer. The stack starts at the bottom and works its way up, so we have to
		// set the stack pointer to the bottom.
		(*my_proc.frame).regs[2] =
			STACK_ADDR as usize + STACK_PAGES * PAGE_SIZE;
		// USER MODE! This is how we set what'll go into mstatus when we run the process.
		(*my_proc.frame).mode = CpuMode::User as usize;
		(*my_proc.frame).pid = my_proc.pid as usize;
		// The SATP register is used for the MMU, so we need to
		// map our table into that register. The switch_to_user
		// function will load .satp into the actual register
		// when the time comes.
		(*my_proc.frame).satp =
			build_satp(
						SatpMode::Sv39,
						my_proc.pid as usize,
						my_proc.root as usize,
			);
	}
	// The ASID field of the SATP register is only 16-bits, and we reserved
	// 0 for the kernel, even though we run the kernel in machine mode for now.
	// Since we don't reuse PIDs, this means that we can only spawn 65534 processes.
	satp_fence_asid(my_pid as usize);
	// I took a different tact here than in process.rs. In there I created the process
	// while holding onto the process list. It doesn't really matter since this is synchronous,
	// but it might get dicey 
	if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
		// As soon as we push this process on the list, it'll be schedule-able.
		println!(
			"Added user process to the scheduler...get \
				ready for take-off!"
		);
		pl.push_back(my_proc);
		unsafe {
			PROCESS_LIST.replace(pl);
		}
	}
	else {
		println!("Unable to spawn process.");
		// Since my_proc couldn't enter the process list, it
		// will be dropped and all of the associated allocations
		// will be deallocated.
	}
	println!();
}

