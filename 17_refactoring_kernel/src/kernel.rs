#![no_std]
#![no_main]
#![feature(fn_align)]

mod common;
mod disk;
mod fs;
mod memory;
mod process;

use crate::common::{
    SCAUSE_ECALL, SYS_EXIT, SYS_GETCHAR, SYS_PUTCHAR, SYS_READFILE, SYS_WRITEFILE,
};
use crate::disk::Device;
use crate::fs::FileSystem;
use crate::process::{PROCESS_TABLE, Process, ProcessState};

unsafe extern "C" {
    static __bss: u8;
    static __bss_end: u8;
    static __stack_top: u8;

}

// from shell.bin.o application
unsafe extern "C" {
    static _binary_target_riscv32i_unknown_none_elf_debug_shell_bin_start: u8;
    static _binary_target_riscv32i_unknown_none_elf_debug_shell_bin_end: u8;
    static _binary_target_riscv32i_unknown_none_elf_debug_shell_bin_size: u8;
}

macro_rules! read_csr {
    ($reg:expr) => {{
        let value: u32;
        unsafe {
            core::arch::asm!(concat!("csrr {}, ", $reg), out(reg) value, options(nomem, nostack));
        }
        value
    }};
}
pub(crate) use read_csr;

macro_rules! write_csr {
    ($reg:expr, $value:expr) => {{
        let value = $value;
        unsafe {
            core::arch::asm!(concat!("csrw ", $reg, ", {}"), in(reg) value, options(nomem, nostack));
        }
    }};
}
pub(crate) use write_csr;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
unsafe extern "C" fn boot() -> ! {
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

static mut FILE_SYSTEM: *mut FileSystem = core::ptr::null_mut();

#[unsafe(no_mangle)]
fn kernel_main() -> ! {
    unsafe {
        let bss_size = &__bss_end as *const u8 as usize - &__bss as *const u8 as usize;
        core::ptr::write_bytes(&__bss as *const u8 as *mut u8, 0, bss_size);
    }

    common::println!("\n\nHello {}\n", "World!");

    write_csr!("stvec", kernel_entry);

    let device = Device::new();

    unsafe {
        let mut filesystem = FileSystem::new(device);
        FILE_SYSTEM = &mut filesystem as *mut FileSystem;

        PROCESS_TABLE.idol = Process::new(core::ptr::null(), 0);
        (*PROCESS_TABLE.idol).set_pid(0);
        PROCESS_TABLE.current = PROCESS_TABLE.idol;

        let binary_shell_bin_start =
            &_binary_target_riscv32i_unknown_none_elf_debug_shell_bin_start as *const u8;
        let binary_shell_bin_size =
            &_binary_target_riscv32i_unknown_none_elf_debug_shell_bin_size as *const u8 as usize;
        Process::new(binary_shell_bin_start, binary_shell_bin_size);
    }

    Process::yield_proc();
    panic!("switched to idle process");
}

// https://ryochack.hatenablog.com/entry/2018/03/23/184943
#[derive(Debug)]
#[repr(C, packed)]
struct TrapFrame {
    ra: i32,
    gp: i32,
    tp: i32,
    t0: i32,
    t1: i32,
    t2: i32,
    t3: i32,
    t4: i32,
    t5: i32,
    t6: i32,
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    a6: i32,
    a7: i32,
    s0: i32,
    s1: i32,
    s2: i32,
    s3: i32,
    s4: i32,
    s5: i32,
    s6: i32,
    s7: i32,
    s8: i32,
    s9: i32,
    s10: i32,
    s11: i32,
    sp: i32,
}

#[repr(align(4))]
fn kernel_entry() {
    unsafe {
        /*
        SCRATCHレジスタ
        - カーネルが自由に利用してよいレジスタとしてMSCRATCH、SSCRATCHと呼ばれるレジスタが用意されている。
        - 割り込み・例外エントリ処理など、汎用レジスタの値を壊すことが許されない処理において、汎用レジスタの値の一時退避先として利用することができる。
        */
        core::arch::asm!(
            // 実行中プロセスのカーネルスタックをsscratchから取り出す
            // tmp = sp; sp = sscratch; sscratch = tmp;
            "csrrw sp, sscratch, sp",
            // 全ての汎用レジスタ（ra, gp, tp, t0〜t6, a0〜a7, s0〜s11）をスタックに保存
            "addi sp, sp, -4 * 31",
            "sw ra,  4 * 0(sp)",
            "sw gp,  4 * 1(sp)",
            "sw tp,  4 * 2(sp)",
            "sw t0,  4 * 3(sp)",
            "sw t1,  4 * 4(sp)",
            "sw t2,  4 * 5(sp)",
            "sw t3,  4 * 6(sp)",
            "sw t4,  4 * 7(sp)",
            "sw t5,  4 * 8(sp)",
            "sw t6,  4 * 9(sp)",
            "sw a0,  4 * 10(sp)",
            "sw a1,  4 * 11(sp)",
            "sw a2,  4 * 12(sp)",
            "sw a3,  4 * 13(sp)",
            "sw a4,  4 * 14(sp)",
            "sw a5,  4 * 15(sp)",
            "sw a6,  4 * 16(sp)",
            "sw a7,  4 * 17(sp)",
            "sw s0,  4 * 18(sp)",
            "sw s1,  4 * 19(sp)",
            "sw s2,  4 * 20(sp)",
            "sw s3,  4 * 21(sp)",
            "sw s4,  4 * 22(sp)",
            "sw s5,  4 * 23(sp)",
            "sw s6,  4 * 24(sp)",
            "sw s7,  4 * 25(sp)",
            "sw s8,  4 * 26(sp)",
            "sw s9,  4 * 27(sp)",
            "sw s10, 4 * 28(sp)",
            "sw s11, 4 * 29(sp)",
            // 例外発生時のspを取り出して保存
            "csrr a0, sscratch",
            "sw a0, 4 * 30(sp)",
            // 「例外発生時のスタックポインタを信頼しない」ために、カーネルスタックを設定し直す
            // そもそも、なぜ信頼すべきではないのか考えてみましょう。
            // 例外ハンドラでは、次の3つのパターンを考慮する必要があります。
            // 1. カーネルモードで例外が発生した
            //   - スタックポインタを設定し直さなくても基本的に問題ありません
            // 2. 例外処理中にカーネルモードで例外が発生した (ネストされた例外)
            //   - 退避領域を上書きしてしまいますが、本実装ではネストされた例外からの復帰を想定せずカーネルパニックして停止するため問題ありません
            // 3. ユーザーモードで例外が発生した
            //   - このとき、spは「ユーザー (アプリケーション) のスタック領域」を指しています。
            //   - spをそのまま利用する (信頼する) 実装の場合では、不正な値をセットして例外を発生させると、カーネルをクラッシュさせる脆弱性に繋がります
            "addi a0, sp, 4 * 31",
            "csrw sscratch, a0",
            // 新しいスタックポインタを引数に設定して、
            // トラップハンドラ呼び出し
            "mv a0, sp",
            "call {handle_trap}",
            // 保存した全てのレジスタをスタックから復元
            "lw ra,  4 * 0(sp)",
            "lw gp,  4 * 1(sp)",
            "lw tp,  4 * 2(sp)",
            "lw t0,  4 * 3(sp)",
            "lw t1,  4 * 4(sp)",
            "lw t2,  4 * 5(sp)",
            "lw t3,  4 * 6(sp)",
            "lw t4,  4 * 7(sp)",
            "lw t5,  4 * 8(sp)",
            "lw t6,  4 * 9(sp)",
            "lw a0,  4 * 10(sp)",
            "lw a1,  4 * 11(sp)",
            "lw a2,  4 * 12(sp)",
            "lw a3,  4 * 13(sp)",
            "lw a4,  4 * 14(sp)",
            "lw a5,  4 * 15(sp)",
            "lw a6,  4 * 16(sp)",
            "lw a7,  4 * 17(sp)",
            "lw s0,  4 * 18(sp)",
            "lw s1,  4 * 19(sp)",
            "lw s2,  4 * 20(sp)",
            "lw s3,  4 * 21(sp)",
            "lw s4,  4 * 22(sp)",
            "lw s5,  4 * 23(sp)",
            "lw s6,  4 * 24(sp)",
            "lw s7,  4 * 25(sp)",
            "lw s8,  4 * 26(sp)",
            "lw s9,  4 * 27(sp)",
            "lw s10, 4 * 28(sp)",
            "lw s11, 4 * 29(sp)",
            // 元のスタックポインタも復元
            "lw sp,  4 * 30(sp)",
            // sret 命令を実行して、スーパーバイザーモード（S-mode）から戻ります
            // これにより、トラップが発生した場所に制御が戻ります
            "sret",
            handle_trap = sym handle_trap,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
unsafe fn handle_trap(f: *mut TrapFrame) {
    let scause = read_csr!("scause");
    let stval = read_csr!("stval");
    let mut user_pc = read_csr!("sepc");

    if scause as usize == SCAUSE_ECALL {
        unsafe {
            if f.is_null() {
                panic!("Null pointer dereference");
            }

            handle_syscall(&mut *f);
        }

        user_pc += 4;
    } else {
        panic!(
            "unexpected trap scause={:x}, stval={:x}, sepc={:x}",
            scause, stval, user_pc,
        );
    }

    write_csr!("sepc", user_pc);
}

fn handle_syscall(f: &mut TrapFrame) {
    let a4 = f.a4 as usize;
    match a4 {
        SYS_PUTCHAR => {
            let a0 = f.a0 as u8 as char;
            crate::common::putchar(a0);
        }
        SYS_GETCHAR => loop {
            let c = crate::common::getchar();
            if c >= 0 {
                f.a0 = c as i32;
                break;
            }

            Process::yield_proc();
        },
        SYS_EXIT => {
            unsafe {
                if PROCESS_TABLE.current.is_null() {
                    panic!("invalid process state");
                }

                let mut current = &mut (*PROCESS_TABLE.current) as &mut Process;
                crate::common::println!("process {} exited", current.pid);
                current.set_state(ProcessState::ProcExit);
            }

            Process::yield_proc();
            panic!("unreachable");
        }
        SYS_READFILE | SYS_WRITEFILE => unsafe {
            let filename_ptr = f.a0 as *const u8;
            let filename_len = f.a1 as usize;
            let filename = core::slice::from_raw_parts(filename_ptr, filename_len);

            let buf_ptr = f.a2 as *mut u8;
            let buf_len = f.a3 as usize;

            if FILE_SYSTEM.is_null() {
                panic!("filesystem not found");
            }

            let filesystem = &mut *FILE_SYSTEM;
            if let Some(file) = filesystem.lookup(&filename) {
                if a4 == SYS_WRITEFILE {
                    // NOTE: explicitely copy by byte for resolving memory layout
                    // core::ptr::copy_nonoverlapping(buf_ptr, file.data.as_mut_ptr() as *mut u8, buf_len);
                    crate::memory::memcpy_by_byte(
                        file.data.as_mut_ptr() as *mut u8,
                        buf_ptr,
                        buf_len,
                    );
                    file.size = buf_len;
                    filesystem.flush();
                } else {
                    core::ptr::copy_nonoverlapping(
                        file.data.as_ptr() as *const u8,
                        buf_ptr,
                        buf_len,
                    );
                }

                f.a0 = buf_len as i32;
            } else {
                crate::common::println!(
                    "file not found: {}",
                    core::str::from_utf8(filename).unwrap()
                );

                f.a0 = -1;
            }
        },
        _ => {
            panic!("unexpected syscall a4={}", a4);
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    common::println!("{}", info);
    loop {}
}
