#![no_std]
#![no_main]
#![feature(fn_align)]

mod common;

unsafe extern "C" {
    static __stack_top: u8;
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.start")]
pub unsafe extern "C" fn start() -> ! {
    unsafe {
        // スタックポインタを設定し、main を呼び出す
        core::arch::asm!(
            "mv sp, {stack_top}",
            "call {main}",
            "call {exit}",
            stack_top = in(reg) &__stack_top,
            main = sym main,
            exit = sym exit,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
fn main() -> ! {
    // common::println!("\nHello World from {}!", "shell");

    loop {
        'prompt: loop {
            common::print!("> ");
            let mut cmdline = [0u8; 128];
            for i in 0.. {
                let c = common::user_getchar() as u8;
                common::user_putchar(c.try_into().unwrap());

                if i == cmdline.len() - 1 {
                    common::println!("command line too long");
                    continue 'prompt;
                }

                if c == b'\r' {
                    common::println!("");
                    cmdline[i] = 0;
                    break;
                }

                cmdline[i] = c;
            }

            // cmdline配列から最初のnullバイトまでの部分を切り出し、それをUTF-8文字列として解釈する
            let term_pos = cmdline
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(cmdline.len());
            let command = match core::str::from_utf8(&cmdline[..term_pos]) {
                Ok(s) => s,
                Err(_) => {
                    common::println!("Invalid UTF-8 sequence");
                    continue 'prompt;
                }
            };

            match command {
                "hello" => {
                    common::println!("Hello world from shell!");
                },
                "exit" => {
                    // common::println!("exit from shell!");
                    exit();
                },
                "readfile" => {
                    // common::println!("read from shell!");
                    let filename = b"hello.txt";
                    let mut buf: [u8; 128] = [0; 128];
                    let buf_len = buf.len();
                    let len = common::user_readfile(filename, filename.len(), &mut buf, buf_len);
                    let read = core::str::from_utf8(
                        &buf[..buf.iter().position(|&c| c == 0).unwrap()])
                        .unwrap();
                    crate::common::println!("readfile: {:?}", read);
                },
                "writefile" => {
                    let filename = b"hello.txt";
                    let buf = b"Hello from shell!\n";
                    common::user_writefile(filename, filename.len(), buf, buf.len());
                },
                _ => {
                    common::println!("unknown command: {}", command);
                },
            }

            break;
        }
    }
}

#[unsafe(no_mangle)]
fn exit() -> ! {
    common::syscall(common::SYS_EXIT, 0, 0, 0, 0);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
