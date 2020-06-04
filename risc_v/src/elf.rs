// elf.rs
// Routines for reading and parsing ELF
// (Executable and Linkable Format) files.
// 26-April-2020
// Stephen Marz

use crate::{buffer::Buffer,
            cpu::{build_satp, memcpy, satp_fence_asid, CpuMode, Registers, SatpMode, TrapFrame},
            page::{map, zalloc, EntryBits, Table, PAGE_SIZE},
            process::{Process, ProcessData, ProcessState, NEXT_PID, STACK_ADDR, STACK_PAGES}};
use alloc::collections::VecDeque;
// Every ELF file starts with ELF "magic", which is a sequence of four bytes 0x7f followed by capital ELF, which is 0x45, 0x4c, and 0x46 respectively.
pub const MAGIC: u32 = 0x464c_457f;

/// The ELF header contains information about placement and numbers of the important sections within our file.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Header {
	pub magic:             u32,
	pub bitsize:           u8,
	pub endian:            u8,
	pub ident_abi_version: u8,
	pub target_platform:   u8,
	pub abi_version:       u8,
	pub padding:           [u8; 7],
	pub obj_type:          u16,
	pub machine:           u16, // 0xf3 for RISC-V
	pub version:           u32,
	pub entry_addr:        usize,
	pub phoff:             usize,
	pub shoff:             usize,
	pub flags:             u32,
	pub ehsize:            u16,
	pub phentsize:         u16,
	pub phnum:             u16,
	pub shentsize:         u16,
	pub shnum:             u16,
	pub shstrndx:          u16
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProgramHeader {
	pub seg_type: u32,
	pub flags:    u32,
	pub off:      usize,
	pub vaddr:    usize,
	pub paddr:    usize,
	pub filesz:   usize,
	pub memsz:    usize,
	pub align:    usize
}

pub const TYPE_EXEC: u16 = 2;

pub const PROG_READ: u32 = 4;
pub const PROG_WRITE: u32 = 2;
pub const PROG_EXECUTE: u32 = 1;

pub const MACHINE_RISCV: u16 = 0xf3;
pub const PH_SEG_TYPE_NULL: u32 = 0;
pub const PH_SEG_TYPE_LOAD: u32 = 1;
pub const PH_SEG_TYPE_DYNAMIC: u32 = 2;
pub const PH_SEG_TYPE_INTERP: u32 = 3;
pub const PH_SEG_TYPE_NOTE: u32 = 4;

pub struct Program {
	pub header: ProgramHeader,
	pub data:   Buffer
}

pub enum LoadErrors {
	Magic,
	Machine,
	TypeExec,
	FileRead
}

pub struct File {
	pub header:   Header,
	pub programs: VecDeque<Program>
}

impl File {
	pub fn load(buffer: &Buffer) -> Result<Self, LoadErrors> {
		let elf_hdr;
		unsafe {
			// Load the ELF
			elf_hdr = (buffer.get() as *const Header).as_ref().unwrap();
		}
		// The ELF magic is 0x75, followed by ELF
		if elf_hdr.magic != MAGIC {
			return Err(LoadErrors::Magic);
		}
		// We need to make sure we're built for RISC-V
		if elf_hdr.machine != MACHINE_RISCV {
			return Err(LoadErrors::Machine);
		}
		// ELF has several types. However, we can only load
		// executables.
		if elf_hdr.obj_type != TYPE_EXEC {
			return Err(LoadErrors::TypeExec);
		}
		let ph_tab = unsafe { buffer.get().add(elf_hdr.phoff) } as *const ProgramHeader;
		// There are phnum number of program headers. We need to go through
		// each one and load it into memory, if necessary.
		let mut ret = Self { header:   *elf_hdr,
		                     programs: VecDeque::new() };
		for i in 0..elf_hdr.phnum as usize {
			unsafe {
				let ph = ph_tab.add(i).as_ref().unwrap();
				// If the segment isn't marked as LOAD (loaded into memory),
				// then there is no point to this. Most executables use a LOAD
				// type for their program headers.
				if ph.seg_type != PH_SEG_TYPE_LOAD {
					continue;
				}
				// If there's nothing in this section, don't load it.
				if ph.memsz == 0 {
					continue;
				}
				let mut ph_buffer = Buffer::new(ph.memsz);

				memcpy(ph_buffer.get_mut(), buffer.get().add(ph.off), ph.memsz);
				ret.programs.push_back(Program { header: *ph,
				                                 data:   ph_buffer });
			}
		}
		Ok(ret)
	}

	// load
	pub fn load_proc(buffer: &Buffer) -> Result<Process, LoadErrors> {
		let elf_fl = Self::load(&buffer);
		if elf_fl.is_err() {
			return Err(elf_fl.err().unwrap());
		}
		let elf_fl = elf_fl.ok().unwrap();
		let mut sz = 0usize;
		// Get the size, in memory, that we're going to need for the program storage.
		for p in elf_fl.programs.iter() {
			sz += p.header.memsz;
		}
		// We add two pages since we could possibly split the front and back pages, hence
		// necessitating the need for two extra pages. This can get wasteful, but for now
		// if we don't do this, we could end up mapping into the MMU table!
		let program_pages = (sz + PAGE_SIZE * 2) / PAGE_SIZE;
		// I did this to demonstrate the expressive nature of Rust. Kinda cool, no?
		let my_pid = unsafe {
			let p = NEXT_PID + 1;
			NEXT_PID += 1;
			p
		};
		let mut my_proc = Process { frame:       zalloc(1) as *mut TrapFrame,
		                            stack:       zalloc(STACK_PAGES),
		                            pid:         my_pid,
		                            mmu_table:        zalloc(1) as *mut Table,
		                            state:       ProcessState::Running,
		                            data:        ProcessData::new(),
		                            sleep_until: 0,
									program:     zalloc(program_pages),
									brk:         0,
								 };

		let program_mem = my_proc.program;
		let table = unsafe { my_proc.mmu_table.as_mut().unwrap() };
		// The ELF has several "program headers". This usually mimics the .text,
		// .rodata, .data, and .bss sections, but not necessarily.
		// What we do here is map the program headers into the process' page
		// table.
		for p in elf_fl.programs.iter() {
			// The program header table starts where the ELF header says it is
			// given by the field phoff (program header offset).
			// Copy the buffer we got from the filesystem into the program
			// memory we're going to map to the user. The memsz field in the
			// program header tells us how many bytes will need to be loaded.
			// The ph.off is the offset to load this into.
			unsafe {
				memcpy(program_mem.add(p.header.off), p.data.get(), p.header.memsz);
			}
			// We start off with the user bit set.
			let mut bits = EntryBits::User.val();
			// This sucks, but we check each bit in the flags to see
			// if we need to add it to the PH permissions.
			if p.header.flags & PROG_EXECUTE != 0 {
				bits |= EntryBits::Execute.val();
			}
			if p.header.flags & PROG_READ != 0 {
				bits |= EntryBits::Read.val();
			}
			if p.header.flags & PROG_WRITE != 0 {
				bits |= EntryBits::Write.val();
			}
			// Now we map the program counter. The virtual address
			// is provided in the ELF program header.
			let pages = (p.header.memsz + PAGE_SIZE) / PAGE_SIZE;
			for i in 0..pages {
				let vaddr = p.header.vaddr + i * PAGE_SIZE;
				// The ELF specifies a paddr, but not when we
				// use the vaddr!
				let paddr = program_mem as usize + p.header.off + i * PAGE_SIZE;
				// There is no checking here! This is very dangerous, and I have already
				// been bitten by it. I mapped too far and mapped userspace into the MMU
				// table, which is AWFUL!
				map(table, vaddr, paddr, bits, 0);
				if vaddr > my_proc.brk {
					my_proc.brk = vaddr;
				}
				// println!("DEBUG: Map 0x{:08x} to 0x{:08x} {:02x}", vaddr, paddr, bits);
			}
			my_proc.brk += 0x1000;
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
			(*my_proc.frame).pc = elf_fl.header.entry_addr;
			// Stack pointer. The stack starts at the bottom and works its
			// way up, so we have to set the stack pointer to the bottom.
			(*my_proc.frame).regs[Registers::Sp as usize] = STACK_ADDR as usize + STACK_PAGES * PAGE_SIZE - 0x1000;
			// USER MODE! This is how we set what'll go into mstatus when we
			// run the process.
			(*my_proc.frame).mode = CpuMode::User as usize;
			(*my_proc.frame).pid = my_proc.pid as usize;
			// The SATP register is used for the MMU, so we need to
			// map our table into that register. The switch_to_user
			// function will load .satp into the actual register
			// when the time comes.
			(*my_proc.frame).satp = build_satp(SatpMode::Sv39, my_proc.pid as usize, my_proc.mmu_table as usize);
		}
		// The ASID field of the SATP register is only 16-bits, and we reserved
		// 0 for the kernel, even though we run the kernel in machine mode for
		// now. Since we don't reuse PIDs, this means that we can only spawn
		// 65534 processes.
		satp_fence_asid(my_pid as usize);
		Ok(my_proc)
	}
}
