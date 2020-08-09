// syscall.rs
// System calls
// Stephen Marz
// 3 Jan 2020

use crate::{block, vfs, fd, process, page, cpu, cpu::Registers};
use alloc::string::String;

/// user_syscall is called from trap.rs to invoke a system call. This
/// is now separate from M and S modes.
/// Since we can't do anything unless we dereference the passed pointer,
/// I went ahead and made the entire function unsafe.
/// If we return 0 from this function, the m_trap function will schedule
/// the next process--consider this a yield. A non-0 is the program counter
/// we want to go back to.
pub unsafe fn user_syscall(mepc: usize, frame_ptr: *mut cpu::TrapFrame) {
	// Libgloss expects the system call number in A7, so let's follow
	// their lead.
	// A7 is X17, so it's register number 17.
	if frame_ptr.is_null() {
		return;
	}

	// Get a Rust mutable reference to frame. This is better than using raw pointers
	// even though we're in an unsafe context.
	let frame = frame_ptr.as_mut().unwrap();
	let syscall_number = frame.regs[cpu::gpr(Registers::A7)];
	let process_ptr = process::get_by_pid(frame.pid as u16);

	if process_ptr.is_null() {
		return;
	}
	let process = process_ptr.as_mut().unwrap();
	// skip the ecall
	frame.pc = mepc + 4;

	match syscall_number {
		1 => {
			// yield
			// We don't do anything, but we don't want to print "unknown system call"
		}
		17 => {
			// char *getcwd(char *buffer, int size)
			let buf = frame.regs[cpu::gpr(Registers::A0)];
			let size = frame.regs[cpu::gpr(Registers::A1)];
			let bytes = process.data.cwd.as_bytes();
			let bytes_to_copy = if size >= bytes.len() { bytes.len() } else { size }; 
			let bytes_copied = page::copy_to_user(process, buf, bytes.as_ptr() as usize, bytes_to_copy);
			frame.regs[cpu::gpr(Registers::A0)] = bytes_copied;
		}
		23 => {
			// int dup(int filedes)
			let filedes = frame.regs[cpu::gpr(Registers::A0)] as u16;
			if let Some(val) = process.data.fdesc.get(&filedes) {
				let new_desc_key = process.data.find_next_fd();
				let new_desc_val = *val;
				process.data.fdesc.insert(new_desc_key, new_desc_val);
				frame.regs[cpu::gpr(Registers::A0)] = new_desc_key as usize;
			}
			else {
				frame.regs[cpu::gpr(Registers::A0)] = -1isize as usize;
			}
		}
		25 => {
			// int fcntl(int filedes, int cmd, int flags)
			let filedes = frame.regs[cpu::gpr(Registers::A0)] as u16;
			let cmd = frame.regs[cpu::gpr(Registers::A1)];
			let flags = frame.regs[cpu::gpr(Registers::A2)];

		}
		48 => {
			// int faccessat(int fd, const char *path, int amode, int flags)
			let fd = frame.regs[cpu::gpr(Registers::A0)];
			let path = frame.regs[cpu::gpr(Registers::A1)] as *const u8;
			let amode = frame.regs[cpu::gpr(Registers::A2)];
			let flags = frame.regs[cpu::gpr(Registers::A3)];
		}
		49 => {
			// int chdir(const char *path)
			let path = frame.regs[cpu::gpr(Registers::A0)] as *const u8;
			// TODO
			frame.regs[cpu::gpr(Registers::A0)] = -1isize as usize;
		}
		56 => {
			// int openat(int fd, const char *path, int oflag)
			let fd = frame.regs[cpu::gpr(Registers::A0)];
			let path = frame.regs[cpu::gpr(Registers::A1)] as *const u8;
			let oflag = frame.regs[cpu::gpr(Registers::A2)];
		}
		57 => {
			// int close(int filedesc)
			let filedesc = frame.regs[cpu::gpr(Registers::A0)] as u16;
			let kr = process.data.fdesc.remove(&filedesc);
			if kr.is_some() {
				frame.regs[cpu::gpr(Registers::A0)] = 0;
			}
			else {
				frame.regs[cpu::gpr(Registers::A0)] = -1isize as usize;
			}
		}
		61 => {
			// int getdents(int fd, *dirp, count)
			let fd = frame.regs[cpu::gpr(Registers::A0)];
			let dirp = frame.regs[cpu::gpr(Registers::A1)] as *const u8;
			let count = frame.regs[cpu::gpr(Registers::A2)];
		}
		62 => {
			// int lseek(int fd, int offset, int whence)
			let fd = frame.regs[cpu::gpr(Registers::A0)];
			let offset = frame.regs[cpu::gpr(Registers::A1)];
			let whence = frame.regs[cpu::gpr(Registers::A2)];
		}
		63 => {
			// int read(int fd, char *buffer, int size)

		}
		93 | 94 => {
			// exit and exit_group
			// The exit system call takes a return number. We don't handle that, yet.
			process::delete_process(frame.pid as u16);
		}
		172 => {
			// int getpid()
			frame.regs[cpu::gpr(Registers::A0)] = frame.pid as usize;
		}
		214 => {
			// void *brk(void *addr)
			frame.regs[cpu::gpr(Registers::A0)] = process.set_brk(frame.regs[cpu::gpr(Registers::A0)]);
		}
		1024 => {
			// int open(const char *path, int mode)
			let path = frame.regs[cpu::gpr(Registers::A0)] as *mut u8;
			let mode = frame.regs[cpu::gpr(Registers::A1)];
			let path_str = String::with_capacity(30);
			
			let op = vfs::open(path_str.as_str());
			let fd;
			if let Ok(entry) = op {
				fd = process.data.find_next_fd();
				let entry = op.ok().unwrap();
				process.data.fdesc.insert(fd, fd::DescriptorType::File(entry));
			}
			else {
				fd = -1isize as u16;
			}
			frame.regs[cpu::gpr(Registers::A0)] = fd as usize;

		}
		1033 => {
			// int access(const char *path, int amode)
			let path = frame.regs[cpu::gpr(Registers::A0)] as *const u8;
			let amode = frame.regs[cpu::gpr(Registers::A1)];
		}
		_ => {
			println!("Unknown user system call {}", syscall_number);
		}
	}
}

pub unsafe fn machine_syscall(mepc: usize, frame_ptr: *mut cpu::TrapFrame) {
	if frame_ptr.is_null() {
		return;
	}

	// Get a Rust mutable reference to frame. This is better than using raw pointers
	// even though we're in an unsafe context.
	let frame = frame_ptr.as_mut().unwrap();
	let syscall_number = frame.regs[cpu::gpr(Registers::A0)];
	let process_ptr = process::get_by_pid(frame.pid as u16);

	if process_ptr.is_null() {
		return;
	}
	let process = process_ptr.as_mut().unwrap();
	// skip the ecall
	frame.pc = mepc + 4;

	match syscall_number {
		1 => {
			// yield, do nothing
		}
		94 => {
			// exit(int)
			process::delete_process(frame.pid as u16);
		}
		99000 => {
			process::set_waiting(frame.pid as u16);
			let _ = block::block_op(
			                 frame.regs[Registers::A1 as usize],
			                 frame.regs[Registers::A2 as usize] as *mut u8,
			                 frame.regs[Registers::A3 as usize] as u32,
			                 frame.regs[Registers::A4 as usize] as u64,
			                 false,
			                 frame.pid as u16
			);
		}
		_ => {
			println!("Unknown machine syscall {}", syscall_number);
		}
	}
}

pub fn syscall_exit() {
	unsafe {
		asm!("ecall", in("a0") 94);
	}
}

pub fn syscall_yield() {
	unsafe {
		asm!("ecall", in("a0") 1);
	}
}

pub fn syscall_execv(path: *const u8, v: usize) {

}

pub fn syscall_sleep(tm: usize) {

}

pub fn syscall_block_read(bdev: usize, buffer: *mut u8, size: u32, offset: u32) -> u8 {
	let ret;
	unsafe {
		asm!("ecall", 
			in("a0") 99000, 
			in("a1") bdev, 
			in("a2") buffer as usize,
			in("a3") size as usize,
			in("a4") offset as usize,
			lateout("a0") ret,
		);
	}
	ret
}

/*
#define SYS_getcwd 17
#define SYS_dup 23
#define SYS_fcntl 25
#define SYS_faccessat 48
#define SYS_chdir 49
#define SYS_openat 56
#define SYS_close 57
#define SYS_getdents 61
#define SYS_lseek 62
#define SYS_read 63
#define SYS_write 64
#define SYS_writev 66
#define SYS_pread 67
#define SYS_pwrite 68
#define SYS_fstatat 79
#define SYS_fstat 80
#define SYS_exit 93
#define SYS_exit_group 94
#define SYS_kill 129
#define SYS_rt_sigaction 134
#define SYS_times 153
#define SYS_uname 160
#define SYS_gettimeofday 169
#define SYS_getpid 172
#define SYS_getuid 174
#define SYS_geteuid 175
#define SYS_getgid 176
#define SYS_getegid 177
#define SYS_brk 214
#define SYS_munmap 215
#define SYS_mremap 216
#define SYS_mmap 222
#define SYS_open 1024
#define SYS_link 1025
#define SYS_unlink 1026
#define SYS_mkdir 1030
#define SYS_access 1033
#define SYS_stat 1038
#define SYS_lstat 1039
#define SYS_time 1062
#define SYS_getmainvars 2011
*/
