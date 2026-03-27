# SPDX-License-Identifier: Apache-2.0
# Minimal Zephyr app for rsgdb E2E (native_sim). Built with:
#   west build -b native_sim/native/64 -s /path/to/zephyr_multi_printf_app

cmake_minimum_required(VERSION 3.20.0)
find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(multi_printf_rsgdb)

target_sources(app PRIVATE src/main.c)
