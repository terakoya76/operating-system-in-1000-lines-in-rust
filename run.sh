#!/bin/bash
set -xue

cargo build

# QEMUのファイルパス
QEMU=qemu-system-riscv32
# QEMUを起動
#   -machine virt     = virtマシンとして起動する
#   -bios default     = デフォルトのBIOS (ここではOpenSBI) を使用する
#   -nographic        = QEMUをウィンドウなしで起動する
#   -serial mon:stdio = QEMUの標準入出力を仮想マシンのシリアルポートに接続する
#   --no-reboot       = 仮想マシンがクラッシュしたら、再起動せずに停止させる (デバッグに便利)
$QEMU \
  -machine virt \
  -bios default \
  -nographic \
  -serial mon:stdio \
  --no-reboot \
  -kernel target/riscv32i-unknown-none-elf/debug/kernel_elf
