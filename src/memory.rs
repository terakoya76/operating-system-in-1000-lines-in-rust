extern "C" {
    static __kernel_base: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[no_mangle]
pub fn memset(buf: *mut u8, c: u8, n: usize) {
    let p = buf;
    let mut i = 0;
    while i < n {
        unsafe {
            *p.add(i) = c;
            i += 1;
        }
    }
}

pub const PAGE_SIZE: usize = 4096;

pub type Paddr = usize;
pub type Vaddr = usize;

#[no_mangle]
pub fn alloc_pages(n: usize) -> Paddr {
    // staticを使って前回の割り当て位置を記憶
    static mut NEXT_PADDR: Paddr = 0;

    unsafe {
        // 初期化が必要な場合（最初の呼び出し時）
        if NEXT_PADDR == 0 {
            NEXT_PADDR = &__free_ram as *const u8 as Paddr;
        }

        let paddr = NEXT_PADDR;
        NEXT_PADDR += (n as usize) * PAGE_SIZE;

        // メモリ範囲チェック
        if NEXT_PADDR > (&__free_ram_end as *const u8 as Paddr) {
            panic!("out of memory");
        }

        // 割り当てたメモリをゼロクリア
        memset(paddr as *mut u8, 0, (n as usize) * PAGE_SIZE);

        paddr
    }
}

/*
ページテーブルエントリー(RISC-V Sv32)
- PPN[1] (12 ビット)
- PPN[0] (10ビット)
- Flags (10ビット)

仮想アドレス(RISC-V Sv32)
- VPN[1] (10 ビット)
- VPN[0] (10ビット)
- Offset (12ビット)

https://vlsi.jp/UnderstandMMU.html
*/
pub const SATP_SV32: usize = 1 << 31;
const PAGE_V: usize = 1 << 0; // 有効化ビット
const PAGE_R: usize = 1 << 1; // 読み込み可能
const PAGE_W: usize = 1 << 2; // 書き込み可能
const PAGE_X: usize = 1 << 3; // 実行可能
const PAGE_U: usize = 1 << 4; // ユーザーモードでアクセス可能

fn is_aligned(value: usize, align: usize) -> bool {
    value % align == 0
}

pub fn init_page_table(table: &mut [usize]) {
    unsafe {
        let kernel_base_addr = &__kernel_base as *const u8 as Paddr;
        let free_ram_end_addr = &__free_ram_end as *const u8 as Paddr;

        // ページテーブルへのマッピングループ
        let mut paddr = kernel_base_addr;
        while paddr < free_ram_end_addr {
            // ID マッピング（物理アドレス = 仮想アドレス）
            map_page(table, paddr, paddr, PAGE_R | PAGE_W | PAGE_X);
            paddr += PAGE_SIZE as Paddr;
        }
    }
}

pub fn map_page(table1: &mut [usize], vaddr: Vaddr, paddr: Paddr, flags: usize) {
    if !is_aligned(vaddr, PAGE_SIZE) {
        panic!("unaligned vaddr {:#x}", vaddr);
    }

    if !is_aligned(paddr, PAGE_SIZE) {
        panic!("unaligned paddr {:#x}", paddr);
    }

    // 0x3ff = 10bit mask
    let vpn1 = (vaddr >> 22) & 0x3ff;
    if (table1[vpn1] & PAGE_V) == 0 {
        // 2段目のページテーブルが存在しないので作成する
        let pt_paddr = alloc_pages(1);
        let ppn1 = pt_paddr / PAGE_SIZE;
        table1[vpn1] = (ppn1 << 10) | PAGE_V;
    }

    // 2段目のページテーブルにエントリを追加する
    let vpn0 = (vaddr >> 12) & 0x3ff;
    let ppn1 = table1[vpn1] >> 10;
    let table0_ptr = (ppn1 * PAGE_SIZE) as *mut usize;
    unsafe {
        let table0 = core::slice::from_raw_parts_mut(table0_ptr, 1024);
        let ppn0 = paddr / PAGE_SIZE;
        table0[vpn0] = (ppn0 << 10) | flags | PAGE_V;
    }
}
