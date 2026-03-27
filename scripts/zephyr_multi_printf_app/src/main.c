/*
 * SPDX-License-Identifier: Apache-2.0
 * rsgdb E2E: multiple printf lines — GDB breaks on first, steps over the next.
 */
#include <stdio.h>

int main(void)
{
	printf("RSGDB_E2E line 1\n");
	printf("RSGDB_E2E line 2\n");
	printf("RSGDB_E2E line 3\n");
	return 0;
}
