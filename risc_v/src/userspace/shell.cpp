#include <printf.h>
#include <syscall.h>

int main()
{
	unsigned long a;
	asm volatile("mv %0, sp\n" : "=r"(a));	
	printf("Stack is at %p\n", a);
	return 0;
}
