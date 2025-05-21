use crate::common::{
    PAGE_R, PAGE_SIZE, PAGE_TABLE_ENTRY, PAGE_U, PAGE_V, PAGE_W, PAGE_X, USER_BASE,
    VIRTIO_BLK_PADDR,
};

unsafe extern "C" {
    static __kernel_base: u8;
    static __free_ram: u8;
    static __free_ram_end: u8;
}

pub type Paddr = usize;
pub type Vaddr = usize;

pub fn memcpy_by_byte(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
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

// staticを使って前回の割り当て位置を記憶
static mut NEXT_PADDR: Paddr = 0;

pub fn alloc_pages(n: usize) -> Paddr {
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
        core::ptr::write_bytes(paddr as *mut u8, 0, (n as usize) * PAGE_SIZE);

        paddr
    }
}

pub const fn is_aligned(value: usize, align: usize) -> bool {
    value % align == 0
}

pub const fn align_up(value: usize, align: usize) -> usize {
    ((value + align - 1) / align) * align
}

#[derive(Debug, Clone, Copy)]
pub struct PageTable {
    pub addr: Paddr,
}

impl PageTable {
    pub fn new(image: *const u8, image_size: usize) -> Self {
        unsafe {
            let kernel_base_addr = &__kernel_base as *const u8 as Paddr;
            let free_ram_end_addr = &__free_ram_end as *const u8 as Paddr;

            let page_table_page = alloc_pages(1);
            let mut page_table = PageTable {
                addr: page_table_page,
            };

            // ページテーブルへのマッピングループ
            let mut paddr = kernel_base_addr;
            while paddr < free_ram_end_addr {
                // ID マッピング（物理アドレス = 仮想アドレス）
                page_table.map_page(paddr, paddr, PAGE_R | PAGE_W | PAGE_X);
                paddr += PAGE_SIZE as Paddr;
            }

            // 各プロセスのページテーブルに virtio-blk のMMIO領域をマップ
            page_table.map_page(VIRTIO_BLK_PADDR, VIRTIO_BLK_PADDR, PAGE_R | PAGE_W);

            // image を memory に展開
            let mut offset: usize = 0;
            while offset < image_size {
                let page = alloc_pages(1);

                // コピーするデータがページサイズより小さい場合を考慮
                let remaining = image_size - offset;
                let copy_size = if remaining >= PAGE_SIZE {
                    PAGE_SIZE
                } else {
                    remaining
                };

                // 確保したページにデータをコピー
                core::ptr::copy_nonoverlapping(image.add(offset), page as *mut u8, copy_size);

                // ページテーブルにマッピング
                page_table.map_page(
                    USER_BASE + offset as Vaddr,
                    page,
                    PAGE_U | PAGE_R | PAGE_W | PAGE_X,
                );

                offset += PAGE_SIZE;
            }

            page_table
        }
    }

    fn map_page(&mut self, vaddr: Vaddr, paddr: Paddr, flags: usize) {
        if !is_aligned(vaddr, PAGE_SIZE) {
            panic!("unaligned vaddr {:#x}", vaddr);
        }

        if !is_aligned(paddr, PAGE_SIZE) {
            panic!("unaligned paddr {:#x}", paddr);
        }

        let table1 = self.as_mut_slice();

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
            let table0 = core::slice::from_raw_parts_mut(table0_ptr, PAGE_TABLE_ENTRY);
            let ppn0 = paddr / PAGE_SIZE;
            table0[vpn0] = (ppn0 << 10) | flags | PAGE_V;
        }
    }

    fn as_mut_slice(&mut self) -> &mut [usize] {
        let base_ptr = self.addr as *mut usize;

        unsafe { core::slice::from_raw_parts_mut(base_ptr, PAGE_TABLE_ENTRY) }
    }
}
