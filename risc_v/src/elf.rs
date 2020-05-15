// elf.rs
// Routines for reading and parsing ELF
// (Executable and Linkable Format) files.
// 26-April-2020
// Stephen Marz

use crate::{buffer::Buffer, cpu::memcpy};
use alloc::collections::VecDeque;
// Every ELF file starts with ELF "magic", which is a sequence of four bytes 0x7f followed by capital ELF, which is 0x45, 0x4c, and 0x46 respectively.
pub const MAGIC: u32 = 0x464c_457f;

/// The ELF header contains information about placement and numbers of the important sections within our file.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Header {
    pub magic: u32,
    pub bitsize: u8,
    pub endian: u8,
    pub ident_abi_version: u8,
    pub target_platform: u8,
    pub abi_version: u8,
    pub padding: [u8; 7],
    pub obj_type: u16,
    pub machine: u16, // 0xf3 for RISC-V
    pub version: u32,
    pub entry_addr: usize,
    pub phoff: usize,
    pub shoff: usize,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProgramHeader {
    pub seg_type: u32,
    pub flags: u32,
    pub off: usize,
    pub vaddr: usize,
    pub paddr: usize,
    pub filesz: usize,
    pub memsz: usize,
    pub align: usize,
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
    pub data: Buffer
}

pub struct File {
    pub header: Header,
    pub programs: VecDeque<Program>
}

impl File {
    pub fn load(buffer: &Buffer) -> Option<Self> {
        let elf_hdr;
        unsafe {
            // Load the ELF
            elf_hdr =
                (buffer.get() as *const Header).as_ref().unwrap();
        }
        // The ELF magic is 0x75, followed by ELF
        if elf_hdr.magic != MAGIC {
            println!("ELF magic didn't match.");
            return None;
        }
        // We need to make sure we're built for RISC-V
        if elf_hdr.machine != MACHINE_RISCV {
            println!("ELF loaded is not RISC-V.");
            return None;
        }
        // ELF has several types. However, we can only load
        // executables.
        if elf_hdr.obj_type != TYPE_EXEC {
            println!("ELF is not an executable.");
            return None;
        }
		let ph_tab = unsafe {buffer.get().add(elf_hdr.phoff) }
					 as *const ProgramHeader;
		// There are phnum number of program headers. We need to go through
		// each one and load it into memory, if necessary.
        let mut ret = Self {
            header: *elf_hdr,
            programs: VecDeque::new()
        };
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
                let mut ph_buffer = Buffer::new(ph.memsz as u32);
                
                memcpy(ph_buffer.get_mut(), buffer.get().add(ph.off), ph.memsz);    
                ret.programs.push_back(Program {
                    header: *ph,
                    data: ph_buffer
                });
            }
        }
        Some(ret)
    }
}
