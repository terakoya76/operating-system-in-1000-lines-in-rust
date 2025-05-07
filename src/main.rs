#![no_std]
#![no_main]
#![feature(fn_align)]

mod common;
mod memory;
mod trap;

extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;
}

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn boot() -> ! {
    // asm macro
    // - https://doc.rust-lang.org/nightly/rust-by-example/unsafe/asm.html
    core::arch::asm!(
        "mv sp, {stack_top}\n
         j {kernel_main}\n",
        // asm template
        // https://doc.rust-lang.org/reference/inline-assembly.html#r-asm.ts-args.order
        stack_top = in(reg) &__stack_top,
        // asm sym
        // https://doc.rust-lang.org/reference/inline-assembly.html#r-asm.operand-type.supported-operands.sym
        kernel_main = sym kernel_main,
        options(noreturn)
    );
}

#[no_mangle]
fn kernel_main() -> ! {
    unsafe {
        let bss_size = &__bss_end as *const u8 as usize - &__bss as *const u8 as usize;
        memory::memset(&__bss as *const u8 as *mut u8, 0, bss_size);
    }

    common::println!("\n\nHello {}\n", "World!");

    let paddr0 = memory::alloc_pages(2);
    let paddr1 = memory::alloc_pages(1);
    common::println!("alloc_pages test: paddr0={:x}", paddr0);
    common::println!("alloc_pages test: paddr1={:x}", paddr1);

    trap::write_csr!("stvec", trap::kernel_entry);
    unsafe {
        core::arch::asm!("unimp", options(nomem, nostack)); // 無効な命令
    }

    common::println!("unreachable here!");

    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    common::println!("{}", info);
    loop {}
}
