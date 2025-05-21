use crate::common::{
    PAGE_SIZE, SECTOR_SIZE, VIRTIO_BLK_PADDR, VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT, VIRTIO_DEVICE_BLK,
    VIRTIO_REG_DEVICE_CONFIG, VIRTIO_REG_DEVICE_ID, VIRTIO_REG_DEVICE_STATUS, VIRTIO_REG_MAGIC,
    VIRTIO_REG_QUEUE_ALIGN, VIRTIO_REG_QUEUE_NOTIFY, VIRTIO_REG_QUEUE_NUM,
    VIRTIO_REG_QUEUE_NUM_MAX, VIRTIO_REG_QUEUE_PFN, VIRTIO_REG_QUEUE_READY, VIRTIO_REG_QUEUE_SEL,
    VIRTIO_REG_VERSION, VIRTIO_STATUS_ACK, VIRTIO_STATUS_DRIVER, VIRTIO_STATUS_DRIVER_OK,
    VIRTIO_STATUS_FEAT_OK, VIRTQ_AVAIL_F_NO_INTERRUPT, VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE,
    VIRTQ_ENTRY_NUM,
};

const FIXED_SIZE_BEFORE_PADDING: usize =
    core::mem::size_of::<[VirtqDesc; VIRTQ_ENTRY_NUM]>() + core::mem::size_of::<VirtqAvail>();
const PADDING_SIZE: usize =
    (PAGE_SIZE - (FIXED_SIZE_BEFORE_PADDING % PAGE_SIZE)) / core::mem::size_of::<u8>();

pub struct Device<'a> {
    vq: &'a mut VirtioVirtq,
    req: &'a mut VirtioBlkReq,
}

impl<'a> Device<'a> {
    pub fn new() -> Self {
        unsafe {
            if virtio_reg_read32(VIRTIO_REG_MAGIC) != 0x74726976 {
                panic!("virtio: invalid magic value");
            }
            if virtio_reg_read32(VIRTIO_REG_VERSION) != 1 {
                panic!("virtio: invalid version");
            }
            if virtio_reg_read32(VIRTIO_REG_DEVICE_ID) != VIRTIO_DEVICE_BLK {
                panic!("virtio: invalid device id");
            }

            // 1. Reset the device.
            virtio_reg_write32(VIRTIO_REG_DEVICE_STATUS, 0);

            // 2. Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
            virtio_reg_fetch_and_or32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_ACK);

            // 3. Set the DRIVER status bit.
            virtio_reg_fetch_and_or32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_DRIVER);

            // 5. Set the FEATURES_OK status bit.
            virtio_reg_fetch_and_or32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_FEAT_OK);

            // 7. Perform device-specific setup, including discovery of virtqueues for the device
            let vq = VirtioVirtq::new(0);

            // 8. Set the DRIVER_OK status bit.
            virtio_reg_write32(VIRTIO_REG_DEVICE_STATUS, VIRTIO_STATUS_DRIVER_OK);

            // ディスクの容量を取得
            let blk_capacity = virtio_get_blk_capacity();
            crate::common::println!("virtio-blk: capacity is {} bytes", blk_capacity);

            let req = VirtioBlkReq::new();

            Device {
                vq: &mut *vq as &mut VirtioVirtq,
                req: &mut *req as &mut VirtioBlkReq,
            }
        }
    }

    // virtio-blkデバイスの読み書き
    pub fn read_write_disk(&mut self, buf: &mut [u8], sector: usize, is_write: bool) {
        // 指定されたセクターがデバイスの容量内に収まっているかを確認
        let blk_capacity = virtio_get_blk_capacity();
        if sector >= (blk_capacity / SECTOR_SIZE) {
            crate::common::println!(
                "virtio: tried to read/write sector={}, but capacity is {}",
                sector,
                blk_capacity / SECTOR_SIZE
            );
            return;
        }

        // virtio-blkの仕様に従って、リクエストを構築する
        let blk_req_paddr = self.req as *const VirtioBlkReq as usize;

        self.req.sector = sector as u64;
        self.req.type_ = if is_write {
            VIRTIO_BLK_T_OUT
        } else {
            VIRTIO_BLK_T_IN
        };

        if is_write {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    self.req.data.as_mut_ptr(),
                    SECTOR_SIZE,
                );
            }
        }

        // virtqueueのディスクリプタを構築する (3つのディスクリプタを使う)
        // 1番目のディスクリプタ: ヘッダー (type(u32), reserved(u32), sector(u64))
        self.vq.descs[0].addr = blk_req_paddr as u64;
        self.vq.descs[0].len =
            (core::mem::size_of::<u32>() * 2 + core::mem::size_of::<u64>()) as u32;
        self.vq.descs[0].flags = VIRTQ_DESC_F_NEXT;
        self.vq.descs[0].next = 1;

        // 2番目のディスクリプタ: データ
        /*
        デバイスからの読み込み操作 (ゲストOSがデータを読む場合)
        - ゲストOSはデバイスからデータを取得したい
        - このとき、デバイスはバッファに書き込む必要があります
        - したがって、バッファには VIRTQ_DESC_F_WRITE フラグが必要です

        デバイスへの書き込み操作 (ゲストOSがデータを書く場合)
        - ゲストOSはデバイスにデータを送信したい
        - このとき、デバイスはバッファから読み取るだけです
        - したがって、バッファには VIRTQ_DESC_F_WRITE フラグは不要です
         */
        self.vq.descs[1].addr = (blk_req_paddr + core::mem::offset_of!(VirtioBlkReq, data)) as u64;
        self.vq.descs[1].len = SECTOR_SIZE as u32;
        self.vq.descs[1].flags = VIRTQ_DESC_F_NEXT | if is_write { 0 } else { VIRTQ_DESC_F_WRITE };
        self.vq.descs[1].next = 2;

        // 3番目のディスクリプタ: ステータス
        self.vq.descs[2].addr =
            (blk_req_paddr + core::mem::offset_of!(VirtioBlkReq, status)) as u64;
        self.vq.descs[2].len = core::mem::size_of::<u8>() as u32;
        self.vq.descs[2].flags = VIRTQ_DESC_F_WRITE;

        // デバイスに新しいリクエストがあることを通知する
        self.vq.kick(0);

        // デバイス側の処理が終わるまで待つ (ビジーウェイト)
        while self.vq.is_busy() {}

        // virtio-blk: 0でない値が返ってきたらエラー
        if self.req.status != 0 {
            crate::common::println!(
                "virtio: warn: failed to read/write sector={} status={}",
                sector,
                self.req.status
            );
            return;
        }

        // 読み込み処理の場合は、バッファにデータをコピーする
        if !is_write {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.req.data.as_ptr(),
                    buf.as_mut_ptr(),
                    SECTOR_SIZE,
                );
            }
        }
    }
}

#[repr(C, packed)]
struct VirtioVirtq {
    descs: [VirtqDesc; VIRTQ_ENTRY_NUM],
    avail: VirtqAvail,

    // for PAGE_SIZE alignment
    _padding: [u8; PADDING_SIZE],

    used: VirtqUsed,
    queue_index: i32,
    used_index: *const u16,
    last_used_index: u16,
}

impl VirtioVirtq {
    fn new(index: usize) -> *mut Self {
        unsafe {
            let pages_of_vq =
                crate::memory::align_up(core::mem::size_of::<Self>(), PAGE_SIZE) / PAGE_SIZE;
            let vq_paddr = crate::memory::alloc_pages(pages_of_vq);
            let vq_ptr = vq_paddr as *mut Self;

            (*vq_ptr).queue_index = index as i32;

            let used_ptr = (*vq_ptr).get_used();
            (*vq_ptr).used_index = (*used_ptr).get_index_ptr();

            // 1. Select the queue writing its index (first queue is 0) to QueueSel.
            virtio_reg_write32(VIRTIO_REG_QUEUE_SEL, index as u32);
            // 5. Notify the device about the queue size by writing the size to QueueNum.
            virtio_reg_write32(VIRTIO_REG_QUEUE_NUM, VIRTQ_ENTRY_NUM as u32);
            // 6. Notify the device about the used alignment by writing its value in bytes to QueueAlign.
            virtio_reg_write32(VIRTIO_REG_QUEUE_ALIGN, 0);
            // 7. Write the physical number of the first page of the queue to the QueuePFN register.
            virtio_reg_write32(VIRTIO_REG_QUEUE_PFN, vq_paddr as u32);

            vq_ptr
        }
    }

    unsafe fn get_descs(&self) -> *mut [VirtqDesc; VIRTQ_ENTRY_NUM] {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, descs);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut [VirtqDesc; VIRTQ_ENTRY_NUM];
            ptr
        }
    }

    unsafe fn get_avail(&self) -> *mut VirtqAvail {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, avail);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut VirtqAvail;
            ptr
        }
    }

    unsafe fn get_used(&self) -> *mut VirtqUsed {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, used);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut VirtqUsed;
            ptr
        }
    }

    unsafe fn get_queue_index(&self) -> i32 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, queue_index);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut i32;
            *ptr
        }
    }

    unsafe fn get_used_index(&self) -> *const u16 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, used_index);
        unsafe {
            let ptr = base_ptr.add(offset) as *const *const u16;
            *ptr
        }
    }

    unsafe fn get_last_used_index(&self) -> u16 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(Self, last_used_index);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut u16;
            *ptr
        }
    }

    // デバイスに新しいリクエストがあることを通知する
    // desc_indexは新しいリクエストの先頭ディスクリプタのインデックス
    fn kick(&mut self, desc_index: i32) {
        let avail_idx = self.avail.index as usize % VIRTQ_ENTRY_NUM;
        self.avail.ring[avail_idx] = desc_index as u16;
        self.avail.index += 1;

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        virtio_reg_write32(VIRTIO_REG_QUEUE_NOTIFY, self.queue_index as u32);
        self.last_used_index += 1;
    }

    // デバイスが処理中のリクエストがあるかどうかを返す
    fn is_busy(&self) -> bool {
        unsafe {
            if !self.used_index.is_null() {
                self.last_used_index != core::ptr::read_volatile(self.used_index)
            } else {
                true
            }
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[derive(Debug)]
#[repr(C, packed)]
struct VirtqAvail {
    flags: u16,
    index: u16,
    ring: [u16; VIRTQ_ENTRY_NUM],
}

impl VirtqAvail {
    unsafe fn get_index(&self) -> u16 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(VirtqAvail, index);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut u16;
            *ptr
        }
    }

    unsafe fn get_ring(&self) -> [u16; VIRTQ_ENTRY_NUM] {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(VirtqAvail, ring);
        unsafe {
            let ptr = base_ptr.add(offset) as *mut [u16; VIRTQ_ENTRY_NUM];
            *ptr
        }
    }
}

#[repr(C, packed)]
struct VirtqUsed {
    flags: u16,
    index: u16,
    ring: [VirtqUsedElem; VIRTQ_ENTRY_NUM],
}

impl VirtqUsed {
    unsafe fn get_index(&self) -> u16 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(VirtqUsed, index);
        unsafe {
            let ptr = base_ptr.add(offset) as *const u16;
            *ptr
        }
    }

    unsafe fn get_index_ptr(&self) -> *const u16 {
        let base_ptr = self as *const Self as *const u8;
        let offset = core::mem::offset_of!(VirtqUsed, index);
        unsafe {
            let ptr = base_ptr.add(offset) as *const u16;
            ptr
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[derive(Debug)]
#[repr(C, packed)]
struct VirtioBlkReq {
    type_: u32,
    reserved: u32,
    sector: u64,
    data: [u8; 512],
    status: u8,
}

impl VirtioBlkReq {
    fn new() -> *mut Self {
        // デバイスへの処理要求を格納する領域を確保
        let pages_of_blk_req =
            crate::memory::align_up(core::mem::size_of::<VirtioBlkReq>(), PAGE_SIZE) / PAGE_SIZE;
        let blk_req_paddr = crate::memory::alloc_pages(pages_of_blk_req);
        let blk_req_ptr = blk_req_paddr as *mut Self;

        blk_req_ptr
    }
}

fn virtio_reg_read32(offset: usize) -> u32 {
    unsafe { core::ptr::read_volatile((VIRTIO_BLK_PADDR + offset) as *const u32) }
}

fn virtio_reg_read64(offset: usize) -> u64 {
    unsafe { core::ptr::read_volatile((VIRTIO_BLK_PADDR + offset) as *const u64) }
}

fn virtio_reg_write32(offset: usize, value: u32) {
    unsafe {
        core::ptr::write_volatile((VIRTIO_BLK_PADDR + offset) as *mut u32, value);
    }
}

fn virtio_reg_fetch_and_or32(offset: usize, value: u32) {
    virtio_reg_write32(offset, virtio_reg_read32(offset) | value);
}

fn virtio_get_blk_capacity() -> usize {
    let blk_capacity = virtio_reg_read64(VIRTIO_REG_DEVICE_CONFIG + 0) as usize * SECTOR_SIZE;
    blk_capacity
}
