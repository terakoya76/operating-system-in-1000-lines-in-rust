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
fn putchar(ch: char) {
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
            // https://doc.rust-jp.rs/rust-by-example-ja/unsafe/asm.html#%E3%83%A1%E3%83%A2%E3%83%AA%E3%82%A2%E3%83%89%E3%83%AC%E3%82%B9%E3%82%AA%E3%83%9A%E3%83%A9%E3%83%B3%E3%83%89
            // nostack:
            // - アセンブリコードがスタックにデータをプッシュしないことを意味します。
            // - これにより、コンパイラはx86-64のスタックレッドゾーンなどの最適化を利用し、スタックポインタの調整を避けることができます。
            options(nostack)
        );
    }

    SbiRet {
        error: a0,
        value: a1,
    }
}

macro_rules! read_csr {
    ($reg:expr) => {{
        let value: u32;
        unsafe {
            core::arch::asm!(concat!("csrr {}, ", $reg), out(reg) value);
        }
        value
    }};
}
pub(crate) use read_csr;

macro_rules! write_csr {
    ($reg:expr, $value:expr) => {{
        let value = $value;
        unsafe {
            core::arch::asm!(concat!("csrw ", $reg, ", {}"), in(reg) value);
        }
    }};
}
pub(crate) use write_csr;
