#include <printf.h>

const int SIZE = 100000;
double myarray[SIZE];
int another_array[5] = {1, 2, 3, 4, 5};

int main()
{
	printf("I'm a C++ program, and I'm running in user space. How about a big, Hello World\n");
	printf("My array is at 0x%p\n", myarray);
	printf("I'm going to start crunching some numbers, so gimme a minute.\n");
	for (int i = 0;i < SIZE;i++) {
		myarray[i] = another_array[i % 5];
	}
	for (int i = 0;i < 100000000;i++) {
		myarray[i % SIZE] += 12.34;
	}
	printf("Ok, I'm done crunching. Wanna see myarray[0]? It's %lf\n", myarray[0]);
	return 0;
}
