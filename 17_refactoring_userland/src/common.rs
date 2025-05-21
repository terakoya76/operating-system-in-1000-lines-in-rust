use core::fmt::{Debug, Write};

pub fn _print(args: core::fmt::Arguments) {
    let mut writer = SyscallWriter {};
    writer.write_fmt(args).unwrap();
}

#[derive(Debug)]
struct SyscallWriter;

impl Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            user_putchar(c);
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

pub const SYS_PUTCHAR: usize = 1;
pub const SYS_GETCHAR: usize = 2;
pub const SYS_EXIT: usize = 3;
pub const SYS_READFILE: usize = 4;
pub const SYS_WRITEFILE: usize = 5;

pub fn user_putchar(ch: char) {
    syscall(SYS_PUTCHAR, ch as usize, 0, 0, 0);
}

pub fn user_getchar() -> usize {
    syscall(SYS_GETCHAR, 0, 0, 0, 0)
}

pub fn user_readfile(
    filename: &[u8],
    filename_len: usize,
    buf: &mut [u8],
    buf_len: usize,
) -> usize {
    let filename_addr = filename.as_ptr() as usize;
    let buf_addr = buf.as_ptr() as usize;
    syscall(SYS_READFILE, filename_addr, filename_len, buf_addr, buf_len)
}

pub fn user_writefile(filename: &[u8], filename_len: usize, buf: &[u8], buf_len: usize) {
    let filename_addr = filename.as_ptr() as usize;
    let buf_addr = buf.as_ptr() as usize;
    syscall(
        SYS_WRITEFILE,
        filename_addr,
        filename_len,
        buf_addr,
        buf_len,
    );
}

pub fn syscall(sysno: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let mut a0 = arg0;
    let a1 = arg1;
    let a2 = arg2;
    let a3 = arg3;
    let a4 = sysno;

    unsafe {
        core::arch::asm!(
            "ecall",
            inout("a0") a0,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
        );
    }

    a0
}
