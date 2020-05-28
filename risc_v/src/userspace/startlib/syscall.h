#pragma once

extern "C"
{
	unsigned long make_syscall(unsigned long sysno,
				   unsigned long a1=0,
				   unsigned long a2=0,
				   unsigned long a3=0,
				   unsigned long a4=0,
				   unsigned long a5=0,
				   unsigned long a6=0);
}
#define syscall_exit()		make_syscall(93)
#define syscall_get_char()	make_syscall(1)
#define syscall_put_char(x)	make_syscall(2, (unsigned long)x)
#define syscall_yield()		make_syscall(9)
#define syscall_sleep(x)	make_syscall(10, (unsigned long)x)
#define syscall_get_fb(x)	make_syscall(1000, (unsigned long)x)
#define syscall_inv_rect(d, x, y, w, h) make_syscall(1001, (unsigned long) d, (unsigned long)x, (unsigned long)y, (unsigned long)w, (unsigned long)h)
#define syscall_get_key(x, y)	make_syscall(1002, (unsigned long)x, (unsigned long)y)
#define syscall_get_abs(x, y)	make_syscall(1004, (unsigned long)x, (unsigned long)y)
#define syscall_get_time()  make_syscall(1062)

