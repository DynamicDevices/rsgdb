# Cross-compile for the target board (aarch64 example).
# Yocto/Poky:  source <sdk>/environment-setup-aarch64-poky-linux  &&  make CC="$CC"
#
# Default is aarch64 cross; a shell-exported CC overrides this — use `make CC=aarch64-linux-gnu-gcc`
# or `env -u CC make` if your environment forces host cc.
CC := aarch64-linux-gnu-gcc
CFLAGS ?= -O0 -g -Wall -Wextra -std=c11
LDFLAGS ?=
TARGET := board_test_app

.PHONY: all clean

all: $(TARGET)

$(TARGET): main.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f $(TARGET)
