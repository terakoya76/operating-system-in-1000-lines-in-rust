#![no_std]
#![no_main]

mod common;

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
fn kernel_main() -> ! {
    common::println!("\n\nHello {}\n", "World!");
    common::println!("1 + 2 = {}, {:x}\n", 1 + 2, 0x1234abcd);

    loop {
        unsafe {
            core::arch::asm!(
                "wfi",
                // https://doc.rust-jp.rs/rust-by-example-ja/unsafe/asm.html#%E3%83%A1%E3%83%A2%E3%83%AA%E3%82%A2%E3%83%89%E3%83%AC%E3%82%B9%E3%82%AA%E3%83%9A%E3%83%A9%E3%83%B3%E3%83%89
                // nomem:
                // - アセンブリコードがメモリの読み書きをしないことを意味します。
                // - デフォルトでは、インラインアセンブリはアクセス可能なメモリアドレス(例えばオペランドとして渡されたポインタや、グローバルなど)の読み書きを行うとコンパイラは仮定しています。
                options(nostack, nomem)
            );
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn boot() -> ! {
    unsafe {
        // asm macro
        // - https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html
        core::arch::asm!(
            "mv sp, {stack_top}",
            "j {kernel_main}",
            // asm template
            // https://doc.rust-lang.org/reference/inline-assembly.html#r-asm.ts-args.order
            stack_top = in(reg) &__stack_top,
            // asm sym
            // https://doc.rust-lang.org/reference/inline-assembly.html#r-asm.operand-type.supported-operands.sym
            kernel_main = sym kernel_main,
            options(noreturn)
        );
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
