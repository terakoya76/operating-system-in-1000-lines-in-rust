# operating-system-in-1000-lines-in-rust

https://operating-system-in-1000-lines.vercel.app/ja/

## Dev

tools
```bash
# linter
$ rustup component add rustfmt

# binutils
$ cargo install cargo-binutils
$ rustup component add llvm-tools
$ rustup component add llvm-tools-preview
```

build
```bash
$ cargo build

# if you want build binary
$ cargo objcopy --release -- -O binary app.bin
```

disassemble a binary
```bash
$ cargo objdump -- --disassemble --no-show-raw-insn

# or llvm-utils
$ llvm-objdump -d target/riscv32i-unknown-none-elf/debug/kernel_elf
```

run os
```bash
$ cargo run
```

map a memory address to the LoC
```bash
$ cargo run
(snip)
panicked at src/main.rs:190:5:
unexpected trap scause=2, stval=c0001073, sepc=80200c5c

# get the LoC from the user pc
$ llvm-addr2line -e target/riscv32i-unknown-none-elf/debug/kernel_elf 80200c5c
operating-system-in-1000-lines-in-rust/src/main.rs:40
```

check symbol(function, variable) address, type and name in the object file
```bash
$ cargo nm --release -- --print-size --size-sort | grep __free_ram
    Finished `release` profile [optimized] target(s) in 0.00s
80233000 00000000 B __free_ram
84233000 00000000 B __free_ram_end

# or llvm-utils
$ llvm-nm target/riscv32i-unknown-none-elf/debug/kernel_elf | grep __free_ram
80224000 B __free_ram
84224000 B __free_ram_end
```
