#include <printf.h>
#include <syscall.h>

int main()
{
	int myarray[1000];
	printf("I'm a C++ program, and I'm running in user space. How about a big, Hello World\n");
	printf("My array is at 0x%p\n", myarray);
	printf("I'm going to start crunching some numbers, so gimme a minute.\n");
	for (int i = 0;i < 1000;i++) {
		myarray[i] = 0;
	}
	for (int i = 0;i < 100000000;i++) {
		myarray[i % 1000] += 1;
	}
	printf("Ok, I'm done crunching. Wanna see myarray[0]? It's %d\n", myarray[0]);
	return 0;
}
