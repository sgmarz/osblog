// test.rs

use crate::{cpu::{build_satp,
                  memcpy,
                  satp_fence_asid,
                  CpuMode,
                  SatpMode,
                  TrapFrame},
            elf,
			fs::{MinixFileSystem, BlockBuffer},
            page::{map, zalloc, EntryBits, Table, PAGE_SIZE},
            process::{Process,
                      ProcessData,
                      ProcessState,
                      NEXT_PID,
					  PROCESS_LIST,
					  PROCESS_LIST_MUTEX,
                      STACK_ADDR,
                      STACK_PAGES}};
use alloc::string::String;

/// Test block will load raw binaries into memory to execute them. This function
/// will load ELF files and try to execute them.
pub fn test_elf() {
	// This won't be necessary after we connect this to the VFS, but for now, we need it.
	const BDEV: usize = 8;
	let mfs = MinixFileSystem::init(BDEV);
	let desc = mfs.open(&String::from("/helloworld.elf")).ok();
	if desc.is_none() {
		println!("Error reading /helloworld.elf");
		return;
	}
	let ino = desc.unwrap();
	// The bytes to read would usually come from the inode, but we are in an
	// interrupt context right now, so we cannot pause. Usually, this would
	// be done by an exec system call.
	let mut buffer = BlockBuffer::new(ino.size);
	// Read the file from the disk. I got the inode by mounting
	// the harddrive as a loop on Linux and stat'ing the inode.

	let bytes_read = MinixFileSystem::read(BDEV, &ino, buffer.get_mut(), ino.size, 0);
	// After compiling our program, I manually looked and saw it was 18,360
	// bytes. So, to make sure we got the right one, I do a manual check
	// here.
	if bytes_read != ino.size {
		println!(
		         "Unable to load program, which should \
		          be {} bytes, got {}",
		         ino.size, bytes_read
		);
		return;
	}
	// Let's get this program running!
	// Everything is "page" based since we're going to map pages to
	// user space. So, we need to know how many program pages we
	// need. Each page is 4096 bytes.
	let program_pages = (bytes_read as usize / PAGE_SIZE) + 1;
	let my_pid = unsafe { NEXT_PID + 1 };
	let elf_hdr;
	unsafe {
		NEXT_PID += 1;
		// Load the ELF
		elf_hdr =
			(buffer.get() as *const elf::Header).as_ref().unwrap();
	}
	// The ELF magic is 0x75, followed by ELF
	if elf_hdr.magic != elf::MAGIC {
		println!("ELF magic didn't match.");
		return;
	}
	// We need to make sure we're built for RISC-V
	if elf_hdr.machine != elf::MACHINE_RISCV {
		println!("ELF loaded is not RISC-V.");
		return;
	}
	// ELF has several types. However, we can only load
	// executables.
	if elf_hdr.obj_type != elf::TYPE_EXEC {
		println!("ELF is not an executable.");
		return;
	}
	let mut my_proc = Process { frame:       zalloc(1) as *mut TrapFrame,
	                            stack:       zalloc(STACK_PAGES),
	                            pid:         my_pid,
	                            root:        zalloc(1) as *mut Table,
	                            state:       ProcessState::Running,
	                            data:        ProcessData::zero(),
	                            sleep_until: 0,
	                            program:     zalloc(program_pages), };

	let program_mem = my_proc.program;
	let table = unsafe { my_proc.root.as_mut().unwrap() };
	// The ELF has several "program headers". This usually mimics the .text,
	// .rodata, .data, and .bss sections, but not necessarily.
	// What we do here is map the program headers into the process' page
	// table.
	unsafe {
		// The program header table starts where the ELF header says it is
		// given by the field phoff (program header offset).
		let ph_tab = buffer.get().add(elf_hdr.phoff)
					 as *const elf::ProgramHeader;
		// There are phnum number of program headers. We need to go through
		// each one and load it into memory, if necessary.
		for i in 0..elf_hdr.phnum as usize {
			let ph = ph_tab.add(i).as_ref().unwrap();
			// If the segment isn't marked as LOAD (loaded into memory),
			// then there is no point to this. Most executables use a LOAD
			// type for their program headers.
			if ph.seg_type != elf::PH_SEG_TYPE_LOAD {
				continue;
			}
			// If there's nothing in this section, don't load it.
			if ph.memsz == 0 {
				continue;
			}
			// Copy the buffer we got from the filesystem into the program
			// memory we're going to map to the user. The memsz field in the
			// program header tells us how many bytes will need to be loaded.
			// The ph.off is the offset to load this into.
			memcpy(
			       program_mem.add(ph.off,),
			       buffer.get().add(ph.off,),
			       ph.memsz,
			);
			// We start off with the user bit set.
			let mut bits = EntryBits::User.val();
			// This sucks, but we check each bit in the flags to see
			// if we need to add it to the PH permissions.
			if ph.flags & elf::PROG_EXECUTE != 0 {
				bits |= EntryBits::Execute.val();
			}
			if ph.flags & elf::PROG_READ != 0 {
				bits |= EntryBits::Read.val();
			}
			if ph.flags & elf::PROG_WRITE != 0 {
				bits |= EntryBits::Write.val();
			}
			// Now we map the program counter. The virtual address
			// is provided in the ELF program header.
			let pages = (ph.memsz + PAGE_SIZE) / PAGE_SIZE;
			for i in 0..pages {
				let vaddr = ph.vaddr + i * PAGE_SIZE;
				// The ELF specifies a paddr, but not when we
				// use the vaddr!
				let paddr = program_mem as usize + ph.off + i * PAGE_SIZE;
				// println!("DEBUG: Map 0x{:08x} to 0x{:08x} {:02x}", vaddr, paddr, bits);
				map(table, vaddr, paddr, bits, 0);
			}
		}
	}
	// This will map all of the program pages. Notice that in linker.lds in
	// userspace we set the entry point address to 0x2000_0000. This is the
	// same address as PROCESS_STARTING_ADDR, and they must match.
	// Map the stack
	let ptr = my_proc.stack as *mut u8;
	for i in 0..STACK_PAGES {
		let vaddr = STACK_ADDR + i * PAGE_SIZE;
		let paddr = ptr as usize + i * PAGE_SIZE;
		// We create the stack. We don't load a stack from the disk.
		// This is why I don't need to make the stack executable.
		map(table, vaddr, paddr, EntryBits::UserReadWrite.val(), 0);
	}
	// Set everything up in the trap frame
	unsafe {
		// The program counter is a virtual memory address and is loaded
		// into mepc when we execute mret.
		(*my_proc.frame).pc = elf_hdr.entry_addr;
		// Stack pointer. The stack starts at the bottom and works its
		// way up, so we have to set the stack pointer to the bottom.
		(*my_proc.frame).regs[2] =
			STACK_ADDR as usize + STACK_PAGES * PAGE_SIZE;
		// USER MODE! This is how we set what'll go into mstatus when we
		// run the process.
		(*my_proc.frame).mode = CpuMode::User as usize;
		(*my_proc.frame).pid = my_proc.pid as usize;
		// The SATP register is used for the MMU, so we need to
		// map our table into that register. The switch_to_user
		// function will load .satp into the actual register
		// when the time comes.
		(*my_proc.frame).satp = build_satp(
		                                   SatpMode::Sv39,
		                                   my_proc.pid as usize,
		                                   my_proc.root as usize,
		);
	}
	// The ASID field of the SATP register is only 16-bits, and we reserved
	// 0 for the kernel, even though we run the kernel in machine mode for
	// now. Since we don't reuse PIDs, this means that we can only spawn
	// 65534 processes.
	satp_fence_asid(my_pid as usize);
	// I took a different tact here than in process.rs. In there I created
	// the process while holding onto the process list. It might
	// matter since this is asynchronous--it is being ran as a kernel process.
	unsafe {
		PROCESS_LIST_MUTEX.sleep_lock();
	}
	if let Some(mut pl) = unsafe { PROCESS_LIST.take() } {
		// As soon as we push this process on the list, it'll be
		// schedule-able.
		println!(
		         "Added user process to the scheduler...get ready \
		          for take-off!"
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
		// will be deallocated through the process' Drop trait.
	}
	unsafe {
		PROCESS_LIST_MUTEX.unlock();
	}
	println!();
}

