// syscall.rs
// System calls
// Stephen Marz
// 3 Jan 2020

use crate::{block::block_op,
            cpu::TrapFrame,
            fs::FileSystem,
            minixfs,
            process::{delete_process, set_sleeping}};

pub fn do_syscall(mepc: usize, frame: *mut TrapFrame) -> usize {
	let syscall_number;
	unsafe {
		// A7 is X17, so it's register number 17.
		syscall_number = (*frame).regs[17];
		// for i in 0..32 {
		//     print!("regs[{:02}] = 0x{:08x}    ", i, (*frame).regs[i]);
		//     if (i+1) % 4 == 0 {
		//         println!();
		//     }
		// }
	}

	match syscall_number {
		0 | 93 => unsafe {
			// Exit
			// Currently, we cannot kill a process, it runs forever. We will delete
			// the process later and free the resources, but for now, we want to get
			// used to how processes will be scheduled on the CPU.
			delete_process((*frame).pid as u16);
			0
		},
		1 => {
			println!("Test syscall");
			mepc + 4
		},
		2 => unsafe {
			// Sleep
			set_sleeping((*frame).pid as u16, (*frame).regs[10]);
			0
		},
		63 => unsafe {
			// Read system call
			// This is an asynchronous call. This will get the process going. We won't hear the answer until
			// we an interrupt back.
			// TODO: The buffer is a virtual memory address that needs to be translated to a physical memory
			// location.
			// This needs to be put into a process and ran.
			let _ = minixfs::process_read(
			                     (*frame).pid as u16,
			                     (*frame).regs[10] as usize,
			                     (*frame).regs[11] as *mut u8,
                                 (*frame).regs[12] as u32,
                                 (*frame).regs[13] as u32
                                );
			// If we return 0, the trap handler will schedule another process.
			0
		},
		180 => unsafe {
			println!(
			         "Pid: {}, Dev: {}, Buffer: 0x{:x}, Size: {}, Offset: {}",
			         (*frame).pid,
			         (*frame).regs[10],
			         (*frame).regs[11],
			         (*frame).regs[12],
			         (*frame).regs[13]
			);
            let _ = block_op((*frame).regs[10],
                            (*frame).regs[11] as *mut u8,
                            (*frame).regs[12] as u32,
                            (*frame).regs[13] as u64,
                            false,
                            (*frame).pid as u16

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
    fn make_syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize;
}

fn do_make_syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
    unsafe {
        make_syscall(sysno, arg0, arg1, arg2, arg3, arg4, arg5)
    }
}

pub fn syscall_exit() {
    let _ = do_make_syscall(93, 0, 0, 0, 0, 0, 0);
}

pub fn syscall_fs_read(dev: usize, buffer: *mut u8, size: u32, offset: u32) -> usize {
	do_make_syscall(63, dev, buffer as usize, size as usize, offset as usize, 0, 0)
}

pub fn syscall_block_read(dev: usize, buffer: *mut u8, size: u32, offset: u32) -> usize {
	do_make_syscall(180, dev, buffer as usize, size as usize, offset as usize, 0, 0)
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
