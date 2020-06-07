#include <cstdio>
#include <unistd.h>
int main()
{
	printf("Started shell.\n");
	char data[100];
	while (1) {
		printf("Enter value: ");
		int r = read(0, data, 100);
		if (r > 0) {
			printf("Got %s\n", data);
		}
	}
	return 0;
}
