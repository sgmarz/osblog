#pragma once

extern "C"
{
	unsigned long make_syscall(unsigned long sysno,
				   unsigned long a1=0,
				   unsigned long a2=0,
				   unsigned long a3=0);
}
#define syscall_exit()		make_syscall(93)
#define syscall_get_char()	make_syscall(1)
#define syscall_put_char(x)	make_syscall(2, (unsigned long)x)
#define syscall_yield()		make_syscall(9)
#define syscall_sleep(x)	make_syscall(10, (unsigned long)x)
