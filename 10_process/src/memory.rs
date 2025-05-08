unsafe extern "C" {
    static __free_ram: u8;
    static __free_ram_end: u8;
}

#[unsafe(no_mangle)]
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

const PAGE_SIZE: usize = 4096;

// 物理アドレスの型
type Paddr = usize;

pub fn alloc_pages(n: u32) -> Paddr {
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
