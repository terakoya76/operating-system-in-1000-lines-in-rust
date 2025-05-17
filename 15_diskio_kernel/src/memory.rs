use crate::common::{
    PAGE_R, PAGE_SIZE, PAGE_U, PAGE_V, PAGE_W, PAGE_X, USER_BASE, VIRTIO_BLK_PADDR,
};

unsafe extern "C" {
    static __kernel_base: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

pub type Paddr = usize;
pub type Vaddr = usize;

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

#[unsafe(no_mangle)]
// core::ptr::copy_nonoverlapping を使ったほうがよい
pub fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut d = dst;
    let mut s = src;
    let mut count = n;

    while count > 0 {
        unsafe {
            *d = *s;
            d = d.add(1);
            s = s.add(1);
        }
        count -= 1;
    }

    dst
}

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

fn is_aligned(value: usize, align: usize) -> bool {
    value % align == 0
}

pub fn align_up(value: usize, align: usize) -> usize {
    ((value + align - 1) / align) * align
}

pub fn init_page_table(table: &mut [usize], image: *const u8, image_size: usize) {
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

        // 各プロセスのページテーブルに virtio-blk のMMIO領域をマップ
        map_page(table, VIRTIO_BLK_PADDR, VIRTIO_BLK_PADDR, PAGE_R | PAGE_W);

        // image を memory に展開
        let mut off: usize = 0;
        while off < image_size {
            let page = alloc_pages(1);

            // コピーするデータがページサイズより小さい場合を考慮
            let remaining = image_size - off as usize;
            let copy_size = if PAGE_SIZE <= remaining {
                PAGE_SIZE
            } else {
                remaining
            };

            // 確保したページにデータをコピー
            memcpy(page as *mut u8, image.add(off as usize), copy_size);

            // ページテーブルにマッピング
            map_page(
                table,
                USER_BASE + off as Vaddr,
                page,
                PAGE_U | PAGE_R | PAGE_W | PAGE_X,
            );

            off += PAGE_SIZE;
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
