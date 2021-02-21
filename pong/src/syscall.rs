use core::fmt::{Write, Error};
use crate::event::Event;

pub struct Writer;

impl Write for Writer {
	fn write_str(&mut self, out: &str) -> Result<(), Error> {
		for c in out.bytes() {
			putchar(c);
		}
		Ok(())
	}
}

pub fn putchar(c: u8) -> usize {
    syscall(2, c as usize, 0, 0, 0, 0, 0, 0)
}

pub fn sleep(tm: usize) {
    let _ = syscall(10, tm, 0, 0, 0, 0, 0, 0);
}

pub fn get_fb(which_fb: usize) -> usize {
    syscall(1000, which_fb, 0, 0, 0, 0, 0, 0)
}

pub fn inv_rect(d: usize, x: usize, y: usize, w: usize, h: usize) {
    let _ = syscall(1001, d, x, y, w, h, 0, 0);
} 

pub fn get_keys(x: *mut Event, y: usize) -> usize {	
    syscall(1002, x as usize, y, 0, 0, 0, 0, 0)
}

pub fn get_abs(x: *mut Event, y: usize) -> usize {	
    syscall(1004, x as usize, y, 0, 0, 0, 0, 0)
}

pub fn get_time() -> usize {
     syscall(1062, 0, 0, 0, 0, 0, 0, 0)
}

pub fn syscall(sysno: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> usize {
    let ret;
    unsafe {
    asm!("ecall",
        in ("a7") sysno,
        in ("a0") a0,
        in ("a1") a1,
        in ("a2") a2,
        in ("a3") a3,
        in ("a4") a4,
        in ("a5") a5,
        in ("a6") a6,
        lateout("a0") ret);
    }
    ret
}
