use crate::common::{PAGE_SIZE, PROCS_MAX, SATP_SV32, SSTATUS_SPIE, SSTATUS_SUM, USER_BASE};
use crate::memory::{PageTable, Vaddr};

// 現在実行中のプロセスとアイドルプロセスのグローバル変数
pub struct ProcessTable {
    pub current: *mut Process,
    pub idol: *mut Process,
    processes: [Process; PROCS_MAX],
}

pub static mut PROCESS_TABLE: ProcessTable = ProcessTable {
    current: core::ptr::null_mut(),
    idol: core::ptr::null_mut(),
    processes: [Process {
        pid: 0,
        state: ProcessState::Unused,
        sp: 0,
        page_table: PageTable { addr: 0 },
        stack: [0; 8192],
    }; PROCS_MAX],
};

#[derive(Clone, Copy)]
pub struct Process {
    pub pid: i32,            // プロセスID
    pub state: ProcessState, // プロセスの状態
    sp: Vaddr,               // コンテキストスイッチ時のスタックポインタ
    page_table: PageTable,   // 動的サイズの配列へのポインタ
    stack: [u8; 8192],       // カーネルスタック
}

impl Process {
    pub fn new(image: *const u8, image_size: usize) -> *mut Self {
        // 空いているプロセス管理構造体(Process Control Block)を探す
        let (proc_index, proc) = unsafe {
            let mut found_index = None;
            let mut found_proc = None;

            for i in 0..PROCS_MAX {
                if PROCESS_TABLE.processes[i].state == ProcessState::Unused {
                    found_index = Some(i);
                    found_proc = Some(&mut PROCESS_TABLE.processes[i]);
                    break;
                }
            }

            // 空きスロットがなければパニック
            match (found_index, found_proc) {
                (Some(idx), Some(p)) => (idx, p),
                _ => panic!("no free process slots"),
            }
        };

        // Process::switch_context() で復帰できるように、スタックに呼び出し先保存レジスタを積む
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

            let page_table = PageTable::new(image, image_size);
            // プロセス情報を更新
            proc.pid = (proc_index + 1) as i32;
            proc.state = ProcessState::Runnable;
            proc.page_table = page_table;
            proc.sp = sp as Vaddr;
        }

        proc as *mut Process
    }

    pub fn set_pid(&mut self, pid: i32) {
        self.pid = pid;
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    fn switch_context(_prev_sp: *mut usize, _next_sp: *mut usize) {
        unsafe {
            core::arch::asm!(
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
    }

    pub fn yield_proc() {
        unsafe {
            // 実行可能なプロセスを探す
            let mut next = PROCESS_TABLE.idol;

            // 現在のプロセスが初期化されているか確認
            if !PROCESS_TABLE.current.is_null() {
                let current = &*PROCESS_TABLE.current;

                // 現在のプロセスからの相対位置でプロセスを探す
                for i in 0..PROCS_MAX {
                    let idx = (current.pid as usize + i) % PROCS_MAX;
                    let proc = &mut PROCESS_TABLE.processes[idx];

                    if proc.state == ProcessState::Runnable && proc.pid > 0 {
                        next = proc as *mut Process;
                        break;
                    }
                }

                // 現在実行中のプロセス以外に、実行可能なプロセスがない場合は戻る
                if next == PROCESS_TABLE.current {
                    return;
                }

                let prev = PROCESS_TABLE.current;
                PROCESS_TABLE.current = next;

                if !prev.is_null() && !next.is_null() {
                    let prev_ref = &mut *prev;
                    let next_ref = &mut *next;

                    core::arch::asm!(
                        // ページテーブルの物理ページ番号を計算
                        "sfence.vma",
                        "csrw satp, {satp}",
                        "sfence.vma",
                        // スタックポインタは下位アドレスの方向に伸びる(スタック領域の末尾から使われていく)ため、
                        // `next.stack[next.stack.len()-1]`バイト目のアドレスをカーネルスタックの初期値として設定します。
                        "csrw sscratch, {sscratch}",
                        satp = in(reg) (SATP_SV32 | (next_ref.page_table.addr as usize / PAGE_SIZE)) as usize,
                        sscratch = in(reg) &next_ref.stack[next_ref.stack.len()-1] as *const u8 as usize,
                        options(nomem, nostack)
                    );

                    Process::switch_context(
                        &mut prev_ref.sp as *mut usize,
                        &mut next_ref.sp as *mut usize,
                    );
                }
            } else {
                // 現在のプロセスが初期化されていない場合
                if !next.is_null() {
                    PROCESS_TABLE.current = next;
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ProcessState {
    Unused,
    Runnable,
    ProcExit,
}

fn user_entry() -> ! {
    unsafe {
        core::arch::asm!(
            "csrw sepc, {sepc}",
            "csrw sstatus, {sstatus}",
            "sret",
            sepc = in(reg) USER_BASE,
            sstatus = in(reg) (SSTATUS_SPIE | SSTATUS_SUM),
            options(noreturn)
        );
    }
}
