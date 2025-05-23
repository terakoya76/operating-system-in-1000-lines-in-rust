use crate::common::{SYS_EXIT, SYS_GETCHAR, SYS_PUTCHAR};
use crate::process::{CURRENT_PROC, ProcessState};

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

// https://ryochack.hatenablog.com/entry/2018/03/23/184943
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct TrapFrame {
    ra: u32,
    gp: u32,
    tp: u32,
    t0: u32,
    t1: u32,
    t2: u32,
    t3: u32,
    t4: u32,
    t5: u32,
    t6: u32,
    a0: u32,
    a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    a7: u32,
    s0: u32,
    s1: u32,
    s2: u32,
    s3: u32,
    s4: u32,
    s5: u32,
    s6: u32,
    s7: u32,
    s8: u32,
    s9: u32,
    s10: u32,
    s11: u32,
    sp: u32,
}

#[unsafe(no_mangle)]
#[repr(align(4))]
pub fn kernel_entry() {
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

const SCAUSE_ECALL: usize = 8;

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
    let a3 = f.a3 as usize;
    match a3 {
        SYS_PUTCHAR => {
            let a0 = f.a0 as u8 as char;
            crate::common::putchar(a0);
        }
        SYS_GETCHAR => loop {
            let c = crate::common::getchar();
            if c >= 0 {
                f.a0 = c as u32;
                break;
            }

            crate::process::yield_proc();
        },
        SYS_EXIT => {
            unsafe {
                if CURRENT_PROC.is_null() {
                    panic!("invalid process state");
                }

                crate::common::println!("process {} exited", (*CURRENT_PROC).pid);
                (*CURRENT_PROC).state = ProcessState::ProcExit;
            }

            crate::process::yield_proc();
            panic!("unreachable");
        }
        _ => {
            panic!("unexpected syscall a3={}", a3);
        }
    }
}
