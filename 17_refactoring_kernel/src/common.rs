/*
memory
*/
pub const USER_BASE: usize = 0x1000000;
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_TABLE_ENTRY: usize = 1024;
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
pub const PAGE_V: usize = 1 << 0; // 有効化ビット
pub const PAGE_R: usize = 1 << 1; // 読み込み可能
pub const PAGE_W: usize = 1 << 2; // 書き込み可能
pub const PAGE_X: usize = 1 << 3; // 実行可能
pub const PAGE_U: usize = 1 << 4; // ユーザーモードでアクセス可能

/*
disk
*/
pub const SECTOR_SIZE: usize = 512;
pub const VIRTQ_ENTRY_NUM: usize = 16;
pub const VIRTIO_DEVICE_BLK: u32 = 2;
pub const VIRTIO_BLK_PADDR: usize = 0x10001000;
pub const VIRTIO_REG_MAGIC: usize = 0x00;
pub const VIRTIO_REG_VERSION: usize = 0x04;
pub const VIRTIO_REG_DEVICE_ID: usize = 0x08;
pub const VIRTIO_REG_QUEUE_SEL: usize = 0x30;
pub const VIRTIO_REG_QUEUE_NUM_MAX: usize = 0x34;
pub const VIRTIO_REG_QUEUE_NUM: usize = 0x38;
pub const VIRTIO_REG_QUEUE_ALIGN: usize = 0x3c;
pub const VIRTIO_REG_QUEUE_PFN: usize = 0x40;
pub const VIRTIO_REG_QUEUE_READY: usize = 0x44;
pub const VIRTIO_REG_QUEUE_NOTIFY: usize = 0x50;
pub const VIRTIO_REG_DEVICE_STATUS: usize = 0x70;
pub const VIRTIO_REG_DEVICE_CONFIG: usize = 0x100;
pub const VIRTIO_STATUS_ACK: u32 = 1;
pub const VIRTIO_STATUS_DRIVER: u32 = 2;
pub const VIRTIO_STATUS_DRIVER_OK: u32 = 4;
pub const VIRTIO_STATUS_FEAT_OK: u32 = 8;
pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;
pub const VIRTQ_AVAIL_F_NO_INTERRUPT: usize = 1;
pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;

/*
fs
*/
pub const FILES_MAX: usize = 2;

/*
process
*/
pub const PROCS_MAX: usize = 8;

/*
interrupt
*/
/*
sstatusレジスタのSPIEビット
- U-Modeに入った際に割り込みが有効化され、
- 例外と同じようにstvecレジスタに設定しているハンドラが呼ばれるようになる
*/
pub const SSTATUS_SPIE: usize = 1 << 5;
/*
sstatusレジスタのSUM(permit Supervisor User Memory access)ビット
- これがセットされていない場合、
- S-Modeのプログラム (カーネル) はU-Mode (ユーザー) のページにアクセスできない。
*/
pub const SSTATUS_SUM: usize = 1 << 18;
pub const SCAUSE_ECALL: usize = 8;

/*
syscall
*/
pub const SYS_PUTCHAR: usize = 1;
pub const SYS_GETCHAR: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_READFILE: usize = 4;
pub const SYS_WRITEFILE: usize = 5;

use core::fmt::Write;

pub fn _print(args: core::fmt::Arguments) {
    let mut writer = SbiWriter {};
    writer.write_fmt(args).unwrap();
}

struct SbiWriter;

impl Write for SbiWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            putchar(c);
        }
        Ok(())
    }
}

macro_rules! print {
    ($($arg:tt)*) => ($crate::common::_print(format_args!($($arg)*)));
}
pub(crate) use print;

macro_rules! println {
    ($fmt:expr) => ($crate::common::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::common::print!(concat!($fmt, "\n"), $($arg)*));
}
pub(crate) use println;

/*
  5.2. Extension: Console Putchar (EID #0x01)


  ```
    long sbi_console_putchar(int ch)
  ```

  Write data present in ch to debug console.
  Unlike `sbi_console_getchar()`, this SBI call will block if there remain any pending characters to be transmitted or if the receiving terminal is not yet ready to receive the byte.
  However, if the console doesn’t exist at all, then the character is thrown away.
  This SBI call returns 0 upon success or an implementation specific negative error code.
  -- "RISC-V Supervisor Binary Interface Specification" v2.0-rc1 より引用
*/
pub fn putchar(ch: char) {
    /*
       SBIが呼ばれると、次のような流れで文字が表示されます。
       1. OSがecall命令を実行すると、CPUはM-modeのトラップハンドラ (mtvecレジスタ) へジャンプする。トラップハンドラはOpenSBIが起動時に設定している。
       2. レジスタの保存などを済ませたのちに、Cで書かれた トラップハンドラ が呼ばれる。
       3. eid に応じたSBI処理関数が呼ばれる。
       4. 8250 UART のデバイスドライバ がQEMUへ文字を送信する。
       5. QEMUの8250 UARTエミュレーション実装が文字を受け取り、標準出力に文字を送る。
       6. 端末エミュレータがその文字を表示する。
    */
    sbi_call(
        ch as isize,
        0,
        0,
        0,
        0,
        0,
        0,
        1, /* ExtensionID = 1 (Console Putchar) */
    );
}

pub fn getchar() -> isize {
    let ret = sbi_call(0, 0, 0, 0, 0, 0, 0, 2);
    ret.error
}

#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: isize,
    pub value: isize,
}

/*
  Chapter 3. Binary Encoding

  All SBI functions share a single binary encoding, which facilitates the mixing of SBI extensions.
  The SBI specification follows the below calling convention.

  1. An ECALL is used as the control transfer instruction between the supervisor and the SEE.
  2. a7 encodes the SBI extension ID (EID),
  3. a6 encodes the SBI function ID (FID) for a given extension ID encoded in a7 for any SBI extension defined in or after SBI v0.2.
  4. All registers except a0 & a1 must be preserved across an SBI call by the callee.
  5. SBI functions must return a pair of values in a0 and a1, with a0 returning an error code.
     This is analogous to returning the C structure
*/
fn sbi_call(
    arg0: isize,
    arg1: isize,
    arg2: isize,
    arg3: isize,
    arg4: isize,
    arg5: isize,
    fid: isize,
    eid: isize,
) -> SbiRet {
    let mut a0 = arg0;
    let mut a1 = arg1;
    let a2 = arg2;
    let a3 = arg3;
    let a4 = arg4;
    let a5 = arg5;
    let a6 = fid;
    let a7 = eid;

    unsafe {
        core::arch::asm!(
            // CPUの実行モードをカーネル用 (S-Mode) からOpenSBI用 (M-Mode) に切り替えてOpenSBIの処理ハンドラを呼び出します。
            // OpenSBIの処理が終わると、再びカーネル用に切り替わり、ecall命令の次の行から実行が再開されます。
            // ちなみに、ecall命令はアプリケーションからカーネルを呼び出す際 (システムコール) にも使われます。
            // 「ひとつ下のレイヤを呼び出す」という機能を持つのがこの命令です。
            "ecall",
            inout("a0") a0,
            inout("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
            in("a6") a6,
            in("a7") a7,
        );
    }

    SbiRet {
        error: a0,
        value: a1,
    }
}
