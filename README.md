# operating-system-in-1000-lines-in-rust

https://operating-system-in-1000-lines.vercel.app/ja/

## Dev
build
```bash
$ rustup run nightly cargo build
```

check assembly
```bash
$ llvm-objdump -d target/riscv32i-unknown-none-elf/debug/kernel_elf
```

run os
```bash
$ rustup run nightly cargo run
```

map a memory address to the LoC
```bash
$ rustup run nightly cargo run
(snip)
panicked at src/main.rs:190:5:
unexpected trap scause=2, stval=c0001073, sepc=80200c5c

# get the LoC from the user pc
$ llvm-addr2line -e target/riscv32i-unknown-none-elf/debug/kernel_elf 80200c5c
operating-system-in-1000-lines-in-rust/src/main.rs:40
```

check symbol(function, variable) address, type and name in the object file
```bash
$ llvm-nm target/riscv32i-unknown-none-elf/debug/kernel_elf | grep __free_ram
80224000 B __free_ram
84224000 B __free_ram_end
```
