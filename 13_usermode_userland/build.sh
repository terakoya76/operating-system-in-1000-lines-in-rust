#!/usr/bin/env bash

set -euxo pipefail

# build
rustup run nightly cargo build

output_dir="target/riscv32i-unknown-none-elf/debug"
output_elf="$output_dir/shell_elf"
output_binary="$output_dir/shell_bin"
output_object="$output_dir/shell_bin.o"

llvm-objcopy \
  --set-section-flags \
  .bss=alloc,contents \
  -O binary \
  "$output_elf" \
  "$output_binary"

llvm-objcopy \
  -I binary \
  -O elf32-littleriscv \
  "$output_binary" \
  "$output_object"
