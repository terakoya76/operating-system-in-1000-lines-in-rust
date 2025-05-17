use crate::common::{PAGE_SIZE, PROCS_MAX, SATP_SV32, SSTATUS_SPIE, USER_BASE};
use crate::memory::Vaddr;

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessState {
    Unused,
    Runnable,
    ProcExit,
}

#[derive(Clone, Copy)]
pub struct Process {
    pub pid: i32,            // プロセスID
    pub state: ProcessState, // プロセスの状態
    sp: Vaddr,               // コンテキストスイッチ時のスタックポインタ
    page_table: *mut usize,  // 動的サイズの配列へのポインタ
    stack: [u8; 8192],       // カーネルスタック
}

static mut PROCS: [Process; PROCS_MAX] = [Process {
    pid: 0,
    state: ProcessState::Unused,
    sp: 0,
    page_table: core::ptr::null_mut(),
    stack: [0; 8192],
}; PROCS_MAX];

#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(_prev_sp: *mut usize, _next_sp: *mut usize) {
    core::arch::naked_asm!(
        // 実行中プロセスのスタックへレジスタを保存
        "addi sp, sp, -13 * 4",
        "sw ra,  0  * 4(sp)",
        "sw s0,  1  * 4(sp)",
        "sw s1,  2  * 4(sp)",
        "sw s2,  3  * 4(sp)",
        "sw s3,  4  * 4(sp)",
        "sw s4,  5  * 4(sp)",
        "sw s5,  6  * 4(sp)",
        "sw s6,  7  * 4(sp)",
        "sw s7,  8  * 4(sp)",
        "sw s8,  9  * 4(sp)",
        "sw s9,  10 * 4(sp)",
        "sw s10, 11 * 4(sp)",
        "sw s11, 12 * 4(sp)",
        // スタックポインタの切り替え
        "sw sp, (a0)",
        "lw sp, (a1)",
        // 次のプロセスのスタックからレジスタを復元
        "lw ra,  0  * 4(sp)",
        "lw s0,  1  * 4(sp)",
        "lw s1,  2  * 4(sp)",
        "lw s2,  3  * 4(sp)",
        "lw s3,  4  * 4(sp)",
        "lw s4,  5  * 4(sp)",
        "lw s5,  6  * 4(sp)",
        "lw s6,  7  * 4(sp)",
        "lw s7,  8  * 4(sp)",
        "lw s8,  9  * 4(sp)",
        "lw s9,  10 * 4(sp)",
        "lw s10, 11 * 4(sp)",
        "lw s11, 12 * 4(sp)",
        "addi sp, sp, 13 * 4",
        "ret",
    );
}

pub fn create_process(image: *const u8, image_size: usize) -> &'static mut Process {
    // 空いているプロセス管理構造体(Process Control Block)を探す
    let (proc_index, proc) = unsafe {
        let mut found_index = None;
        let mut found_proc = None;

        for i in 0..PROCS_MAX {
            if PROCS[i].state == ProcessState::Unused {
                found_index = Some(i);
                found_proc = Some(&mut PROCS[i]);
                break;
            }
        }

        // 空きスロットがなければパニック
        match (found_index, found_proc) {
            (Some(idx), Some(p)) => (idx, p),
            _ => panic!("no free process slots"),
        }
    };

    // switch_context() で復帰できるように、スタックに呼び出し先保存レジスタを積む
    unsafe {
        let stack_top = proc.stack.as_ptr().add(proc.stack.len());
        let mut sp = stack_top as *mut usize;

        sp = sp.sub(1);
        *sp = 0; // s11
        sp = sp.sub(1);
        *sp = 0; // s10
        sp = sp.sub(1);
        *sp = 0; // s9
        sp = sp.sub(1);
        *sp = 0; // s8
        sp = sp.sub(1);
        *sp = 0; // s7
        sp = sp.sub(1);
        *sp = 0; // s6
        sp = sp.sub(1);
        *sp = 0; // s5
        sp = sp.sub(1);
        *sp = 0; // s4
        sp = sp.sub(1);
        *sp = 0; // s3
        sp = sp.sub(1);
        *sp = 0; // s2
        sp = sp.sub(1);
        *sp = 0; // s1
        sp = sp.sub(1);
        *sp = 0; // s0
        sp = sp.sub(1);
        *sp = user_entry as usize; // ra

        // page_table setup
        let page_table_addr = crate::memory::alloc_pages(1);
        // crate::common::println!("page_table_addr: {}", page_table_addr);
        let page_table_ptr = page_table_addr as *mut usize;
        // crate::common::println!("page_table_ptr: {:?}", &page_table_ptr);
        let page_table = core::slice::from_raw_parts_mut(page_table_ptr, 1024);
        crate::memory::init_page_table(page_table, image, image_size);

        // プロセス情報を更新
        proc.pid = (proc_index + 1) as i32;
        proc.state = ProcessState::Runnable;
        proc.page_table = page_table_ptr;
        proc.sp = sp as Vaddr;
    }

    proc
}

#[unsafe(no_mangle)]
fn user_entry() -> ! {
    unsafe {
        core::arch::asm!(
            "csrw sepc, {sepc}",
            "csrw sstatus, {sstatus}",
            "sret",
            sepc = in(reg) USER_BASE,
            sstatus = in(reg) SSTATUS_SPIE,
            options(noreturn)
        );
    }
}

// 現在実行中のプロセスとアイドルプロセスのグローバル変数
pub static mut CURRENT_PROC: *mut Process = core::ptr::null_mut();
pub static mut IDLE_PROC: *mut Process = core::ptr::null_mut();

pub fn yield_proc() {
    unsafe {
        // 実行可能なプロセスを探す
        let mut next = IDLE_PROC;

        // 現在のプロセスが初期化されているか確認
        if !CURRENT_PROC.is_null() {
            let current = &*CURRENT_PROC;

            // 現在のプロセスからの相対位置でプロセスを探す
            for i in 0..PROCS_MAX {
                let idx = (current.pid as usize + i) % PROCS_MAX;
                let proc = &mut PROCS[idx];

                if proc.state == ProcessState::Runnable && proc.pid > 0 {
                    next = proc as *mut Process;
                    break;
                }
            }

            // 現在実行中のプロセス以外に、実行可能なプロセスがない場合は戻る
            if next == CURRENT_PROC {
                // crate::common::println!("no processes are available");
                return;
            }

            let prev = CURRENT_PROC;
            CURRENT_PROC = next;

            if !prev.is_null() && !next.is_null() {
                let prev_ref = &mut *prev;
                let next_ref = &mut *next;

                // crate::common::println!("context switch, prev pid:{}, next pid:{}", prev_ref.pid, next_ref.pid);

                core::arch::asm!(
                    // ページテーブルの物理ページ番号を計算
                    "sfence.vma",
                    "csrw satp, {satp}",
                    "sfence.vma",
                    // スタックポインタは下位アドレスの方向に伸びる(スタック領域の末尾から使われていく)ため、
                    // `next.stack[next.stack.len()-1]`バイト目のアドレスをカーネルスタックの初期値として設定します。
                    "csrw sscratch, {sscratch}",
                    satp = in(reg) (SATP_SV32 | (next_ref.page_table as usize / PAGE_SIZE)) as usize,
                    sscratch = in(reg) &next_ref.stack[next_ref.stack.len()-1] as *const u8 as usize,
                    options(nomem, nostack)
                );

                switch_context(
                    &mut prev_ref.sp as *mut usize,
                    &mut next_ref.sp as *mut usize,
                );
            }
        } else {
            // 現在のプロセスが初期化されていない場合
            if !next.is_null() {
                CURRENT_PROC = next;
            }
        }
    }
}

fn delay() {
    for _ in 0..30000000 {
        unsafe {
            core::arch::asm!("nop", options(nomem, nostack));
        }
    }
}

pub static mut PROC_A: *mut Process = core::ptr::null_mut();
pub static mut PROC_B: *mut Process = core::ptr::null_mut();

pub fn proc_a_entry() {
    crate::common::println!("\nstarting process A");

    loop {
        crate::common::print!("A");

        /*
         Manual Context Switch
        */
        // unsafe {
        //     // nullでないことを確認
        //     if !PROC_A.is_null() && !PROC_B.is_null() {
        //         // rawポインタから参照を作成
        //         let proc_a = &mut *PROC_A;
        //         let proc_b = &mut *PROC_B;

        //         // コンテキストスイッチを実行
        //         switch_context(&mut proc_a.sp as *mut usize,
        //                        &mut proc_b.sp as *mut usize);
        //     } else {
        //         panic!("Process pointers not initialized");
        //     }
        // }

        /*
         Scheduler Context Switch
        */
        yield_proc();

        delay();
    }
}

pub fn proc_b_entry() {
    crate::common::println!("\nstarting process B");

    loop {
        crate::common::print!("B");

        /*
         Manual Context Switch
        */
        // unsafe {
        //     // nullでないことを確認
        //     if !PROC_A.is_null() && !PROC_B.is_null() {
        //         // rawポインタから参照を作成
        //         let proc_a = &mut *PROC_A;
        //         let proc_b = &mut *PROC_B;

        //         // コンテキストスイッチを実行
        //         switch_context(&mut proc_b.sp as *mut usize,
        //                        &mut proc_a.sp as *mut usize);
        //     } else {
        //         panic!("Process pointers not initialized");
        //     }
        // }

        /*
         Scheduler Context Switch
        */
        yield_proc();

        delay();
    }
}
