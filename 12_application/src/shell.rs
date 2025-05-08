#![no_std]
#![no_main]

unsafe extern "C" {
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn start() -> ! {
    unsafe {
        // スタックポインタを設定し、main を呼び出す
        core::arch::asm!(
            "mv sp, {stack_top}",
            "call {main}",
            "call {exit}",
            stack_top = in(reg) &__stack_top,
            main = sym main,
            exit = sym exit,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    loop {}
}

#[unsafe(no_mangle)]
fn putchar(_ch: u8) {
    /* 後で実装する */
}

#[unsafe(no_mangle)]
#[allow(unconditional_recursion)]
fn exit() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
