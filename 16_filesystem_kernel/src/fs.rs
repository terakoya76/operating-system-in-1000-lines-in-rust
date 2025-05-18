use crate::common::{FILES_MAX, SECTOR_SIZE};

const DISK_MAX_SIZE: usize =
    crate::memory::align_up(core::mem::size_of::<File>() * FILES_MAX, SECTOR_SIZE) as usize;

// tarヘッダ構造体
#[repr(C, packed)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    checksum: [u8; 8],
    type_flag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
    padding: [u8; 12],
    data: [u8; 0], // ヘッダに続くデータ領域を指す配列
}

#[derive(Clone, Copy)]
pub struct File {
    in_use: bool,         // このファイルエントリが使われているか
    pub name: [u8; 100],  // ファイル名
    pub data: [u8; 1024], // ファイルの内容
    pub size: usize,      // ファイルサイズ
}

// グローバル変数の宣言
static mut FILES: [File; FILES_MAX] = [File {
    in_use: false,
    name: [0; 100],
    data: [0; 1024],
    size: 0,
}; FILES_MAX];

static mut DISK: [u8; DISK_MAX_SIZE] = [0; DISK_MAX_SIZE];

// 8進数文字列を整数に変換
fn oct2int(oct: &[u8], len: usize) -> usize {
    let mut dec = 0;
    for i in 0..len {
        if oct[i] < b'0' || oct[i] > b'7' {
            break;
        }

        dec = dec * 8 + (oct[i] - b'0') as usize;
    }
    dec
}

// 整数を8進数文字列に変換
fn int2oct(value: usize, oct: &mut [u8]) {
    oct.fill(b'0');

    // 値が0の場合は、すでに'0'で初期化されているので終了
    if value == 0 {
        return;
    }

    // 値を8進数に変換
    let mut val = value;
    let mut i = oct.len();

    while val > 0 && i > 0 {
        i -= 1;
        oct[i] = (val % 8) as u8 + b'0';
        val /= 8;
    }
}

pub fn fs_init() {
    unsafe {
        // ディスクからデータを読み込む
        for sector in 0..(DISK_MAX_SIZE / SECTOR_SIZE) {
            let offset = sector * SECTOR_SIZE;
            crate::disk::read_write_disk(&mut DISK[offset..], sector, false);
        }

        crate::common::println!("read {} bytes from disk", DISK_MAX_SIZE);

        let mut off = 0;
        for i in 0..FILES_MAX {
            //crate::common::println!("disk: {:?}", &DISK[off..(off+SECTOR_SIZE)]);
            //crate::common::println!("DISK: {:?}", core::ptr::addr_of!(DISK));
            //crate::common::println!("DISK: {:?}", core::ptr::addr_of!(DISK).add(off));

            // TARヘッダーへの参照を取得
            //crate::common::println!("disk: {:?}", &DISK[off] as *const u8);
            let header = &mut DISK[off] as *mut u8 as *mut TarHeader;
            //crate::common::println!("header: {:?}", &(*header).name);
            // ヘッダの名前が空ならループを抜ける
            if (*header).name[0] == 0 {
                break;
            }

            // magicフィールドが "ustar" かチェック
            let magic = &(*header).magic;
            let magic_str = core::str::from_utf8(&magic[..5]).unwrap(); // "ustar"は5文字
            if magic_str != "ustar" {
                panic!("invalid tar header: magic={}", magic_str);
            }

            // ファイル構造体を設定
            let file = &mut FILES[i];
            file.in_use = true;

            let file_size = oct2int(&(*header).size, (*header).size.len());
            file.size = file_size;

            file.name.copy_from_slice(&(*header).name);

            // ファイルのデータフィールドにコピー
            // header の直後のデータ部分を指す
            let data_ptr = (header as *const u8).add(core::mem::size_of::<TarHeader>());
            let data_slice = core::slice::from_raw_parts(data_ptr, file_size);
            file.data[..file_size].copy_from_slice(&data_slice[..file_size]);

            // ファイル情報を表示
            let name =
                core::str::from_utf8(&file.name[..file.name.iter().position(|&c| c == 0).unwrap()])
                    .unwrap();

            crate::common::println!("file: {}, size={}", name, file.size);

            off +=
                crate::memory::align_up(core::mem::size_of::<TarHeader>() + file_size, SECTOR_SIZE);
        }
    }
}

pub fn fs_flush() {
    unsafe {
        // files変数の各ファイルの内容をdisk変数に書き込むために、0で初期化
        DISK = [0; DISK_MAX_SIZE];

        let mut off = 0;
        for i in 0..FILES_MAX {
            let file = &FILES[i];
            if !file.in_use {
                continue;
            }

            // ディスクの適切な位置にTARヘッダーを配置
            // - ヘッダーを0で初期化
            // - 文字列フィールドを設定
            let header = &mut DISK[off] as *mut u8 as *mut TarHeader;
            core::ptr::write_bytes(header as *mut u8, 0, core::mem::size_of::<TarHeader>());
            (*header).name.copy_from_slice(&file.name);
            (&mut (*header).mode)[..6].copy_from_slice(b"000644");
            (&mut (*header).magic)[..5].copy_from_slice(b"ustar");
            (*header).version.copy_from_slice(b"00");
            (*header).type_flag = b'0';

            // ファイルサイズを8進数文字列に変換して上書き
            int2oct(file.size, &mut (*header).size);

            // チェックサムを計算
            let mut checksum = b' ' as usize * (*header).checksum.len();
            for j in 0..core::mem::size_of::<TarHeader>() {
                checksum += DISK[off + j] as usize;
            }

            // チェックサムを8進数で設定
            int2oct(checksum, &mut (*header).checksum);

            // ファイルデータをコピー
            let data_ptr = (header as *mut u8).add(core::mem::size_of::<TarHeader>());
            let data_slice = core::slice::from_raw_parts_mut(data_ptr, file.size);
            data_slice.copy_from_slice(&file.data[0..file.size]);

            off +=
                crate::memory::align_up(core::mem::size_of::<TarHeader>() + file.size, SECTOR_SIZE);
        }

        // disk変数の内容をディスクに書き込む
        for sector in 0..(DISK_MAX_SIZE / SECTOR_SIZE) {
            let offset = sector * SECTOR_SIZE;
            crate::disk::read_write_disk(&mut DISK[offset..], sector, true);
        }

        crate::common::println!("wrote {} bytes to disk", DISK_MAX_SIZE);
    }
}

pub fn fs_lookup(filename: &[u8]) -> Option<&mut File> {
    for i in 0..FILES_MAX {
        unsafe {
            let file = &mut FILES[i];
            let name =
                core::str::from_utf8(&file.name[..file.name.iter().position(|&c| c == 0).unwrap()])
                    .unwrap();
            // crate::common::println!("file.name: {:?}", &name);
            // crate::common::println!("filename: {:?}", core::str::from_utf8(&filename));

            if name == core::str::from_utf8(filename).unwrap() {
                return Some(file);
            }
        }
    }

    None
}
