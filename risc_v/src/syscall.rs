// syscall.rs
// System calls
// Stephen Marz
// 3 Jan 2020

use crate::{block::block_op,
            buffer::Buffer,
            cpu::{dump_registers, Registers, TrapFrame, gp},
            elf,
            fs,
            gpu,
            input::{Event, ABS_EVENTS, KEY_EVENTS},
            page::{map, virt_to_phys, EntryBits, Table, PAGE_SIZE, zalloc},
			process::{add_kernel_process_args, delete_process, get_by_pid, set_sleeping, set_waiting, PROCESS_LIST, PROCESS_LIST_MUTEX, Descriptor}};
use crate::console::{IN_LOCK, IN_BUFFER, push_queue};
use alloc::{boxed::Box, string::String};

/// do_syscall is called from trap.rs to invoke a system call. No discernment is
/// made here whether this is a U-mode, S-mode, or M-mode system call.
/// Since we can't do anything unless we dereference the passed pointer,
/// I went ahead and made the entire function unsafe.
/// If we return 0 from this function, the m_trap function will schedule
/// the next process--consider this a yield. A non-0 is the program counter
/// we want to go back to.
pub unsafe fn do_syscall(mepc: usize, frame: *mut TrapFrame) {
	// Libgloss expects the system call number in A7, so let's follow
	// their lead.
	// A7 is X17, so it's register number 17.
	let syscall_number = (*frame).regs[gp(Registers::A7)];
	// skip the ecall
	(*frame).pc = mepc + 4;
	match syscall_number {
		93 | 94 => {
			// exit and exit_group
			delete_process((*frame).pid as u16);
		}
		1 => {
			//yield
			// We don't do anything, but we don't want to print "unknown system call"
		}
		2 => {
			// Easy putchar
			print!("{}", (*frame).regs[Registers::A0 as usize] as u8 as char);
		}
		8 => {
			dump_registers(frame);
		}
		10 => {
			// Sleep
			set_sleeping((*frame).pid as u16, (*frame).regs[Registers::A0 as usize]);
		}
		11 => {
			// execv
			// A0 = path
			// A1 = argv
			let mut path_addr = (*frame).regs[Registers::A0 as usize];
			// If the MMU is turned on, translate.
			if (*frame).satp >> 60 != 0 {
				let p = get_by_pid((*frame).pid as u16);
				let table = ((*p).mmu_table).as_ref().unwrap();
				path_addr = virt_to_phys(table, path_addr).unwrap();
			}
			// Our path address here is now a physical address. If it came in virtual,
			// it is now physical.
			let path_bytes = path_addr as *const u8;
			let mut path = String::new();
			let mut iterator: usize = 0;
			// I really have to figure out how to change an array of bytes
			// to a string. For now, this is very C-style and mimics strcpy.
			loop {
				let ch = *path_bytes.add(iterator);
				if ch == 0 {
					break;
				}
				iterator += 1;
				path.push(ch as char);
			}
			// See if we can find the path.
			if let Ok(inode) = fs::MinixFileSystem::open(8, &path) {
				let inode_heap = Box::new(inode);
				// The Box above moves the Inode to a new memory location on the heap.
				// This needs to be on the heap since we are about to hand over control
				// to a kernel process.
				// THERE is an issue here. If we fail somewhere inside the kernel process,
				// we shouldn't delete our process here. However, since this is asynchronous
				// our process will still get deleted and the error won't be reported.
				// We have to make sure we relinquish Box control here by using into_raw.
				// Otherwise, the Box will free the memory associated with this inode.
				add_kernel_process_args(exec_func, Box::into_raw(inode_heap) as usize);
				// This deletes us, which is what we want.
				delete_process((*frame).pid as u16);
			}
			else {
				// If we get here, the path couldn't be found, or for some reason
				// open failed. So, we return -1 and move on.
				println!("Could not open path '{}'.", path);
				(*frame).regs[Registers::A0 as usize] = -1isize as usize;
			}
		}
		17 => { //getcwd
			let mut buf = (*frame).regs[gp(Registers::A0)] as *mut u8;
			let size = (*frame).regs[gp(Registers::A1)];
			let process = get_by_pid((*frame).pid as u16).as_ref().unwrap();
			let mut iter = 0usize;
			if (*frame).satp >> 60 != 0 {
				let table = ((*process).mmu_table).as_mut().unwrap();
				let paddr = virt_to_phys(table, buf as usize);
				if let Some(bufaddr) = paddr {
					buf = bufaddr as *mut u8;
				}
				else {
					(*frame).regs[gp(Registers::A0)] = -1isize as usize;
					return;
				}
			}
			for i in process.data.cwd.as_bytes() {
				if iter == 0 || iter >= size {
					break;
				}
				buf.add(iter).write(*i);
				iter += 1;
			}
		}
		48 => {
		// #define SYS_faccessat 48
			(*frame).regs[gp(Registers::A0)] = -1isize as usize;
		}
		57 => {
			// #define SYS_close 57
			let fd = (*frame).regs[gp(Registers::A0)] as u16;
			let process = get_by_pid((*frame).pid as u16).as_mut().unwrap();
			if process.data.fdesc.contains_key(&fd) {
				process.data.fdesc.remove(&fd);
				(*frame).regs[gp(Registers::A0)] = 0;
			}
			else {
				(*frame).regs[gp(Registers::A0)] = -1isize as usize;
			}
			// Flush?
		}
		63 => { // sys_read
			let fd = (*frame).regs[gp(Registers::A0)] as u16;
			let mut buf = (*frame).regs[gp(Registers::A1)] as *mut u8;
			let size = (*frame).regs[gp(Registers::A2)];
			let process = get_by_pid((*frame).pid as u16).as_mut().unwrap();
			let mut ret = 0usize;
			// If we return 0, the trap handler will schedule
			// another process.
			if fd == 0 { // stdin
				IN_LOCK.spin_lock();
				if let Some(mut inb) = IN_BUFFER.take() {
					let num_elements = if inb.len() >= size { size } else { inb.len() };
					let mut buf_ptr = buf as *mut u8;
					if num_elements == 0 {
						push_queue((*frame).pid as u16);
						set_waiting((*frame).pid as u16);
					}
					else {
						for i in inb.drain(0..num_elements) {
							if (*frame).satp >> 60 != 0 {
								let table = ((*process).mmu_table).as_mut().unwrap();
								let buf_addr = virt_to_phys(table, buf as usize);
								if buf_addr.is_none() {
									break;
								}
								buf_ptr = buf_addr.unwrap() as *mut u8;
								buf_ptr.write(i);
								ret += 1;
								println!("R: {}", ret);
							}
							buf = buf.add(1);
							buf_ptr = buf_ptr.add(1);
						}
					}
					IN_BUFFER.replace(inb);
				}
				IN_LOCK.unlock();
			}
			(*frame).regs[gp(Registers::A0)] = ret;
		}
		64 => { // sys_write
			let fd = (*frame).regs[gp(Registers::A0)] as u16;
			let buf = (*frame).regs[gp(Registers::A1)] as *const u8;
			let size = (*frame).regs[gp(Registers::A2)];
			let process = get_by_pid((*frame).pid as u16).as_ref().unwrap();
			if fd == 1 || fd == 2 {
				// stdout / stderr
				// println!("WRITE {}, 0x{:08x}, {}", fd, bu/f as usize, size);
				let mut iter = 0;
				for i in 0..size {
					iter += 1;
					if (*frame).satp >> 60 != 0 {
						let table = ((*process).mmu_table).as_mut().unwrap();
						// We don't need to do the following until we reach a page boundary,
						// however that code isn't written, yet.
						let paddr = virt_to_phys(table, buf.add(i) as usize);
						if let Some(bufaddr) = paddr {
							print!("{}", *(bufaddr as *const u8) as char);
						}
						else {
							break;
						}
					}
				}
				(*frame).regs[gp(Registers::A0)] = iter as usize;
			}
			else {
				let descriptor = process.data.fdesc.get(&fd);
				if descriptor.is_none() {
					(*frame).regs[gp(Registers::A0)] = 0;
					return;
				}
				else {
					let descriptor = descriptor.unwrap();
					match descriptor {
						Descriptor::Framebuffer => {

						}
						Descriptor::File(inode) => {

						
						}
						_ => {
							// unsupported
							(*frame).regs[gp(Registers::A0)] = 0;
						}
					}
				}
			}
		}
		66 => {
			(*frame).regs[gp(Registers::A0)] = -1isize as usize;
		}
		// #define SYS_fstat 80
		80 => {
			// int fstat(int filedes, struct stat *buf)
			(*frame).regs[gp(Registers::A0)] = 0;
		}
		172 => {
			// A0 = pid
			(*frame).regs[Registers::A0 as usize] = (*frame).pid;
		}
		180 => {
			set_waiting((*frame).pid as u16);
			let _ = block_op(
			                 (*frame).regs[Registers::A0 as usize],
			                 (*frame).regs[Registers::A1 as usize] as *mut u8,
			                 (*frame).regs[Registers::A2 as usize] as u32,
			                 (*frame).regs[Registers::A3 as usize] as u64,
			                 false,
			                 (*frame).pid as u16
			);
		}
		214 => { // brk
			// #define SYS_brk 214
			// void *brk(void *addr);
			let addr = (*frame).regs[gp(Registers::A0)];
			let process = get_by_pid((*frame).pid as u16).as_mut().unwrap();
			// println!("Break move from 0x{:08x} to 0x{:08x}", process.brk, addr);
			if addr > process.brk {
				if (*frame).satp >> 60 != 0 {
					let table = ((*process).mmu_table).as_mut().unwrap();
					let diff = (addr + PAGE_SIZE - process.brk) / PAGE_SIZE;
					for i in 0..diff {
						let new_addr = zalloc(1) as usize;
						process.data.pages.push_back(new_addr);
						map(table, process.brk + (i << 12), new_addr, EntryBits::UserReadWrite.val(), 0);
					}
				}
				process.brk = addr;
			}
			(*frame).regs[gp(Registers::A0)] = process.brk;
		}
		// System calls 1000 and above are "special" system calls for our OS. I'll
		// try to mimic the normal system calls below 1000 so that this OS is compatible
		// with libraries.
		1000 => {
			// get framebuffer
			// syscall_get_framebuffer(device)
			let dev = (*frame).regs[Registers::A0 as usize];
			(*frame).regs[Registers::A0 as usize] = 0;
			if dev > 0 && dev <= 8 {
				if let Some(p) = gpu::GPU_DEVICES[dev - 1].take() {
					let ptr = p.get_framebuffer() as usize;
					if (*frame).satp >> 60 != 0 {
						let process = get_by_pid((*frame).pid as u16);
						let table = ((*process).mmu_table).as_mut().unwrap();
						let num_pages = (p.get_width() * p.get_height() * 4) as usize / PAGE_SIZE;
						for i in 0..num_pages {
							let vaddr = 0x3000_0000 + (i << 12);
							let paddr = ptr + (i << 12);
							map(table, vaddr, paddr, EntryBits::UserReadWrite as usize, 0);
						}
						gpu::GPU_DEVICES[dev - 1].replace(p);
					}
					(*frame).regs[Registers::A0 as usize] = 0x3000_0000;
				}
			}
		}
		1001 => {
			// transfer rectangle and invalidate
			let dev = (*frame).regs[Registers::A0 as usize];
			let x = (*frame).regs[Registers::A1 as usize] as u32;
			let y = (*frame).regs[Registers::A2 as usize] as u32;
			let width = (*frame).regs[Registers::A3 as usize] as u32;
			let height = (*frame).regs[Registers::A4 as usize] as u32;
			gpu::transfer(dev, x, y, width, height);
		}
		1002 => {
			// wait for keyboard events
			let mut ev = KEY_EVENTS.take().unwrap();
			let max_events = (*frame).regs[Registers::A1 as usize];
			let vaddr = (*frame).regs[Registers::A0 as usize] as *const Event;
			if (*frame).satp >> 60 != 0 {
				let process = get_by_pid((*frame).pid as u16);
				let table = (*process).mmu_table.as_mut().unwrap();
				(*frame).regs[Registers::A0 as usize] = 0;
				let num_events = if max_events <= ev.len() {
					max_events
				}
				else {
					ev.len()
				};
				for i in 0..num_events {
					let paddr = virt_to_phys(table, vaddr.add(i) as usize);
					if paddr.is_none() {
						break;
					}
					let paddr = paddr.unwrap() as *mut Event;
					*paddr = ev.pop_front().unwrap();
					(*frame).regs[Registers::A0 as usize] += 1;
				}
			}
			KEY_EVENTS.replace(ev);
		}
		1004 => {
			// wait for abs events
			let mut ev = ABS_EVENTS.take().unwrap();
			let max_events = (*frame).regs[Registers::A1 as usize];
			let vaddr = (*frame).regs[Registers::A0 as usize] as *const Event;
			if (*frame).satp >> 60 != 0 {
				let process = get_by_pid((*frame).pid as u16);
				let table = ((*process).mmu_table as *mut Table).as_mut().unwrap();
				(*frame).regs[Registers::A0 as usize] = 0;
				for i in 0..if max_events <= ev.len() {
					max_events
				}
				else {
					ev.len()
				} {
					let paddr = virt_to_phys(table, vaddr.add(i) as usize);
					if paddr.is_none() {
						break;
					}
					let paddr = paddr.unwrap() as *mut Event;
					*paddr = ev.pop_front().unwrap();
					(*frame).regs[Registers::A0 as usize] += 1;
				}
			}
			ABS_EVENTS.replace(ev);
		}
		1024 => {
			// #define SYS_open 1024
			let mut path = (*frame).regs[gp(Registers::A0)];
			let _perm = (*frame).regs[gp(Registers::A1)];
			let process = get_by_pid((*frame).pid as u16).as_mut().unwrap();
			if (*frame).satp >> 60 != 0 {
				let table = process.mmu_table.as_mut().unwrap();
				let paddr = virt_to_phys(table, path);
				if paddr.is_none() {
					(*frame).regs[gp(Registers::A0)] = -1isize as usize;
					return;
				}
				path = paddr.unwrap();
			}
			let path_ptr = path as *const u8;
			let mut str_path = String::new();
			for i in 0..256 {
				let c = path_ptr.add(i).read();
				if c == 0 {
					break;
				}
				str_path.push(c as char);
			}
			// Allocate a blank file descriptor
			let mut max_fd = 2;
			for k in process.data.fdesc.keys() {
				if *k > max_fd {
					max_fd = *k;
				}
			}
			max_fd += 1;
			match str_path.as_str() {
				"/dev/fb" => {
					// framebuffer
					process.data.fdesc.insert(max_fd, Descriptor::Framebuffer);
				}
				"/dev/butev" => {
					process.data.fdesc.insert(max_fd, Descriptor::ButtonEvents);
				}
				"/dev/absev" => {
					process.data.fdesc.insert(max_fd, Descriptor::AbsoluteEvents);
				}
				_ => {
					let res = fs::MinixFileSystem::open(8, &str_path);
					if res.is_err() {
						(*frame).regs[gp(Registers::A0)] = -1isize as usize;
						return;
					}
					else {
						let inode = res.ok().unwrap();
						process.data.fdesc.insert(max_fd, Descriptor::File(inode));
					}
				}
			}
			(*frame).regs[gp(Registers::A0)] = max_fd as usize;
		}
		1062 => {
			// gettime
			(*frame).regs[Registers::A0 as usize] = crate::cpu::get_mtime();
		}
		_ => {
			println!("Unknown syscall number {}", syscall_number);
		}
	}
}

extern "C" {
	fn make_syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize;
}

fn do_make_syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> usize {
	unsafe { make_syscall(sysno, arg0, arg1, arg2, arg3, arg4, arg5) }
}

pub fn syscall_yield() {
	let _ = do_make_syscall(1, 0, 0, 0, 0, 0, 0);
}

pub fn syscall_exit() {
	let _ = do_make_syscall(93, 0, 0, 0, 0, 0, 0);
}

pub fn syscall_execv(path: *const u8, argv: usize) -> usize {
	do_make_syscall(11, path as usize, argv, 0, 0, 0, 0)
}

pub fn syscall_fs_read(dev: usize, inode: u32, buffer: *mut u8, size: u32, offset: u32) -> usize {
	do_make_syscall(63, dev, inode as usize, buffer as usize, size as usize, offset as usize, 0)
}

pub fn syscall_block_read(dev: usize, buffer: *mut u8, size: u32, offset: u32) -> u8 {
	do_make_syscall(180, dev, buffer as usize, size as usize, offset as usize, 0, 0) as u8
}

pub fn syscall_sleep(duration: usize) {
	let _ = do_make_syscall(10, duration, 0, 0, 0, 0, 0);
}

pub fn syscall_get_pid() -> u16 {
	do_make_syscall(172, 0, 0, 0, 0, 0, 0) as u16
}

/// This is a helper function ran as a process in kernel space
/// to finish loading and executing a process.
fn exec_func(args: usize) {
	unsafe {
		// We got the inode from the syscall. Its Box rid itself of control, so
		// we take control back here. The Box now owns the Inode and will complete
		// freeing the heap memory allocated for it.
		let inode = Box::from_raw(args as *mut fs::Inode);
		let mut buffer = Buffer::new(inode.size as usize);
		// This is why we need to be in a process context. The read() call may sleep as it
		// waits for the block driver to return.
		fs::MinixFileSystem::read(8, &inode, buffer.get_mut(), inode.size, 0);
		// Now we have the data, so the following will load the ELF file and give us a process.
		let proc = elf::File::load_proc(&buffer);
		if proc.is_err() {
			println!("Failed to launch process.");
		}
		else {
			let process = proc.ok().unwrap();
			// If we hold this lock, we can still be preempted, but the scheduler will
			// return control to us. This required us to use try_lock in the scheduler.
			PROCESS_LIST_MUTEX.sleep_lock();
			if let Some(mut proc_list) = PROCESS_LIST.take() {
				proc_list.push_back(process);
				PROCESS_LIST.replace(proc_list);
			}
			PROCESS_LIST_MUTEX.unlock();
		}
	}
}
// These system call numbers come from libgloss so that we can use newlib
// for our system calls.
// Libgloss wants the system call number in A7 and arguments in A0..A6
// #define SYS_dup 23
// #define SYS_fcntl 25
// #define SYS_faccessat 48
// #define SYS_chdir 49
// #define SYS_openat 56
// #define SYS_getdents 61
// #define SYS_lseek 62
// #define SYS_read 63
// #define SYS_pread 67
// #define SYS_pwrite 68
// #define SYS_fstatat 79

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
// #define SYS_munmap 215
// #define SYS_mremap 216
// #define SYS_mmap 222
// #define SYS_link 1025
// #define SYS_unlink 1026
// #define SYS_mkdir 1030
// #define SYS_access 1033
// #define SYS_stat 1038
// #define SYS_lstat 1039
// #define SYS_time 1062
// #define SYS_getmainvars 2011
