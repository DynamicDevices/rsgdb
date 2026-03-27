/*
 * Minimal smoke binary for GDB + gdbserver on an aarch64 (or other) Linux target.
 * Build with the Makefile or your Yocto/Poky SDK: source environment-setup-* && make CC=$CC
 */
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

volatile int g_counter = 0;

int main(void)
{
	volatile int local_step = 0;

	printf("board_test_app: pid=%ld rsgdb smoke OK\n", (long)getpid());
	fflush(stdout);

	while (1) {
		g_counter++;
		local_step++;

		if ((g_counter % 5) == 0) {
			printf("tick g_counter=%d local_step=%d\n", g_counter, local_step);
			fflush(stdout);
		}
		sleep(1);
	}
	return 0;
}
