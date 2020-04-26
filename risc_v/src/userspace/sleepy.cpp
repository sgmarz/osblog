#include <printf.h>
#include <syscall.h>

int main()
{
	printf("I'm going to bed.\nYou can watch me sleep for 100 switches using 'top'\n");
	for (int i = 0;i < 100;i++) {
		syscall_sleep(1000000);
	}
	return 0;
}
