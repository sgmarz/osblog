// syscall.rs
// System calls
// Stephen Marz
// 3 Jan 2020

use crate::{block::block_op,
			cpu::{dump_registers, TrapFrame, Registers},
			fs,
			elf,
			buffer::Buffer,
			kmem::{kfree, kmalloc},
            page::{virt_to_phys, Table},
            process::{PROCESS_LIST, delete_process, get_by_pid, set_sleeping, set_waiting, add_kernel_process_args}};
use alloc::string::String;
use core::mem::size_of;
/// do_syscall is called from trap.rs to invoke a system call. No discernment is
/// made here whether this is a U-mode, S-mode, or M-mode system call.
/// Since we can't do anything unless we dereference the passed pointer,
/// I went ahead and made the entire function unsafe.
/// If we return 0 from this function, the m_trap function will schedule
/// the next process--consider this a yield. A non-0 is the program counter
/// we want to go back to.
pub unsafe fn do_syscall(mepc: usize, frame: *mut TrapFrame) -> usize {
	// Libgloss expects the system call number in A7, so let's follow
	// their lead.
	// A7 is X17, so it's register number 17.
	let syscall_number = (*frame).regs[Registers::A7 as usize];
	match syscall_number {
		0 | 93 => {
			// Exit
			delete_process((*frame).pid as u16);
			0
		},
		2 => {
			// Easy putchar
			print!("{}", (*frame).regs[Registers::A0 as usize] as u8 as char);
			mepc + 4
		},
		8 => {
			dump_registers(frame);
			mepc + 4
		},
		10 => {
			// Sleep
			set_sleeping((*frame).pid as u16, (*frame).regs[Registers::A0 as usize]);
			0
		},
		11 => {
			// execv
			//A0 = path
			//A1 = argv
			let p = get_by_pid((*frame).pid as u16);
			let table = ((*p).get_table_address()
						 as *mut Table)
						.as_ref()
						.unwrap();
			let mut path_addr = (*frame).regs[Registers::A0 as usize];
			// If the MMU is turned on, translate.
			if (*frame).satp >> 60 != 0 {
				path_addr = virt_to_phys(table, path_addr).unwrap();
			}
			let path_bytes = path_addr as *const u8;
			let mut path = String::new();
			let mut iterator: usize = 0;
			// I really have to figure out how to change an array of bytes
			// to a string.
			loop {
				let ch = *path_bytes.add(iterator);
				if ch == 0 {
					break;
				}
				iterator += 1;
				path.push(ch as char);
			}
			if let Ok(inode) = fs::MinixFileSystem::open(8, &path) {
				let inode_heap = kmalloc(size_of::<fs::Inode>()) as *mut fs::Inode;
				*inode_heap = inode;
				add_kernel_process_args(exec_func, inode_heap as usize);
				delete_process((*frame).pid as u16);
				return 0;
			}
			else {
				println!("Could not open path '{}'.", path);
				(*frame).regs[Registers::A0 as usize] = -1isize as usize;
				return mepc + 4;
			}
		},
		63 => {
			// Read system call
			// This is an asynchronous call. This will get the
			// process going. We won't hear the answer until
			// we an interrupt back.
			// TODO: The buffer is a virtual memory address that
			// needs to be translated to a physical memory location.
			// This needs to be put into a process and ran.
			// The buffer (regs[12]) needs to be translated when ran
			// from a user process using virt_to_phys. If this turns
			// out to be a page fault, we need to NOT proceed with
			// the read!
			let mut physical_buffer = (*frame).regs[Registers::A2 as usize];
			// If the MMU is turned on, we have to translate the
			// address. Eventually, I will put this code into a
			// convenient function, but for now, it will show how
			// translation will be done.
			if (*frame).satp != 0 {
				let p = get_by_pid((*frame).pid as u16);
				let table = ((*p).get_table_address()
				             as *mut Table)
				            .as_ref()
				            .unwrap();
				let paddr =
					virt_to_phys(table, (*frame).regs[12]);
				if paddr.is_none() {
					(*frame).regs[Registers::A0 as usize] = -1isize as usize;
					return mepc + 4;
				}
				physical_buffer = paddr.unwrap();
			}
			// TODO: Not only do we need to check the buffer, but it
			// is possible that the buffer spans multiple pages. We
			// need to check all pages that this might span. We
			// can't just do paddr and paddr + size, since there
			// could be a missing page somewhere in between.
			let _ = fs::process_read(
			                              (*frame).pid as u16,
			                              (*frame).regs[Registers::A0 as usize]
			                              as usize,
			                              (*frame).regs[Registers::A1 as usize] as u32,
			                              physical_buffer
			                              as *mut u8,
			                              (*frame).regs[Registers::A3 as usize] as u32,
			                              (*frame).regs[Registers::A4 as usize] as u32,
			);
			// If we return 0, the trap handler will schedule
			// another process.
			0
		},
		172 => {
			// A0 = pid
			(*frame).regs[Registers::A0 as usize] = (*frame).pid;
			mepc + 4
		},
		180 => {
			set_waiting((*frame).pid as u16);
			let _ = block_op(
			                 (*frame).regs[Registers::A0 as usize],
			                 (*frame).regs[Registers::A1 as usize] as *mut u8,
			                 (*frame).regs[Registers::A2 as usize] as u32,
			                 (*frame).regs[Registers::A3 as usize] as u64,
			                 false,
			                 (*frame).pid as u16,
			);
			0
		},
		_ => {
			println!("Unknown syscall number {}", syscall_number);
			mepc + 4
		},
	}
}

extern "C" {
	fn make_syscall(sysno: usize,
	                arg0: usize,
	                arg1: usize,
	                arg2: usize,
	                arg3: usize,
	                arg4: usize,
	                arg5: usize)
	                -> usize;
}

fn do_make_syscall(sysno: usize,
                   arg0: usize,
                   arg1: usize,
                   arg2: usize,
                   arg3: usize,
                   arg4: usize,
                   arg5: usize)
                   -> usize
{
	unsafe { make_syscall(sysno, arg0, arg1, arg2, arg3, arg4, arg5) }
}

pub fn syscall_exit() {
	let _ = do_make_syscall(93, 0, 0, 0, 0, 0, 0);
}

pub fn syscall_execv(path: *const u8, argv: usize) -> usize {
	do_make_syscall(11, path as usize, argv, 0, 0, 0, 0)
}

pub fn syscall_fs_read(dev: usize,
                       inode: u32,
                       buffer: *mut u8,
                       size: u32,
                       offset: u32)
                       -> usize
{
	do_make_syscall(
	                63,
	                dev,
	                inode as usize,
	                buffer as usize,
	                size as usize,
	                offset as usize,
	                0,
	)
}

pub fn syscall_block_read(dev: usize,
                          buffer: *mut u8,
                          size: u32,
                          offset: u32)
                          -> u8
{
	do_make_syscall(
	                180,
	                dev,
	                buffer as usize,
	                size as usize,
	                offset as usize,
	                0,
	                0,
	) as u8
}

pub fn syscall_sleep(duration: usize)
{
	let _ = do_make_syscall(10, duration, 0, 0, 0, 0, 0);
}

pub fn syscall_get_pid() -> u16 {
	do_make_syscall(172, 0, 0, 0, 0, 0, 0) as u16
}

fn exec_func(args: usize) {
	unsafe {
		let inode_ptr = args as *const fs::Inode;
		let inode = *inode_ptr;
		let mut buffer = Buffer::new(inode.size as usize);
		fs::MinixFileSystem::read(8, &inode, buffer.get_mut(), inode.size, 0);
		let proc = elf::File::load_proc(&buffer, inode.size as usize);
		if proc.is_err() {
			println!("Failed to launch process.");
		}
		else {
			if let Some(mut proc_list) = PROCESS_LIST.take()  {
				proc_list.push_back(proc.ok().unwrap());
				PROCESS_LIST.replace(proc_list);
			}
		}
		kfree(inode_ptr as *mut u8);
	}
}
// These system call numbers come from libgloss so that we can use newlib
// for our system calls.
// Libgloss wants the system call number in A7 and arguments in A0..A6
// #define SYS_getcwd 17
// #define SYS_dup 23
// #define SYS_fcntl 25
// #define SYS_faccessat 48
// #define SYS_chdir 49
// #define SYS_openat 56
// #define SYS_close 57
// #define SYS_getdents 61
// #define SYS_lseek 62
// #define SYS_read 63
// #define SYS_write 64
// #define SYS_writev 66
// #define SYS_pread 67
// #define SYS_pwrite 68
// #define SYS_fstatat 79
// #define SYS_fstat 80
// #define SYS_exit 93
// #define SYS_exit_group 94
// #define SYS_kill 129
// #define SYS_rt_sigaction 134
// #define SYS_times 153
// #define SYS_uname 160
// #define SYS_gettimeofday 169
// #define SYS_getpid 172
// #define SYS_getuid 174
// #define SYS_geteuid 175
// #define SYS_getgid 176
// #define SYS_getegid 177
// #define SYS_brk 214
// #define SYS_munmap 215
// #define SYS_mremap 216
// #define SYS_mmap 222
// #define SYS_open 1024
// #define SYS_link 1025
// #define SYS_unlink 1026
// #define SYS_mkdir 1030
// #define SYS_access 1033
// #define SYS_stat 1038
// #define SYS_lstat 1039
// #define SYS_time 1062
// #define SYS_getmainvars 2011
