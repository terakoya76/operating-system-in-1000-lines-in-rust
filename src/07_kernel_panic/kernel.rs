#![no_std]
#![no_main]

mod common;

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
fn memset(buf: *mut u8, c: u8, n: usize) {
    let p = buf;
    let mut i = 0;
    while i < n {
        unsafe {
            *p.add(i) = c;
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
fn kernel_main() -> ! {
    unsafe {
        let bss_size = &__bss_end as *const u8 as usize - &__bss as *const u8 as usize;
        memset(&__bss as *const u8 as *mut u8, 0, bss_size);
    }

    panic!("booted!");
    common::println!("unreachable here!");

    loop {}
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
fn panic(info: &core::panic::PanicInfo) -> ! {
    common::println!("{}", info);
    loop {}
}
