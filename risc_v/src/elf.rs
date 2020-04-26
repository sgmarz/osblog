// elf.rs
// Routines for reading and parsing ELF
// (Executable and Linkable Format) files.
// 26-April-2020
// Stephen Marz


// Every ELF file starts with ELF "magic", which is a sequence of four bytes 0x7f followed by capital ELF, which is 0x45, 0x4c, and 0x46 respectively.
pub const MAGIC: u32 = 0x464c_457f;

/// The ELF header contains information about placement and numbers of the important sections within our file.
#[repr(C)]
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

