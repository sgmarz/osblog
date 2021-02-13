use core::fmt::{Write, Error};
pub struct Writer;

impl Write for Writer {
	fn write_str(&mut self, out: &str) -> Result<(), Error> {
		for c in out.bytes() {
			syscall_putchar(c);
		}
		Ok(())
	}
}

pub fn syscall_putchar(c: u8) -> usize {
    syscall(2, c as usize, 0, 0, 0, 0, 0, 0)
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
