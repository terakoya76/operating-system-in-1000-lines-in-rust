use crate::common::{FILES_MAX, SECTOR_SIZE};
use crate::disk::Device;

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

impl TarHeader {
    fn is_empty(&self) -> bool {
        self.name[0] == 0
    }

    fn get_name(&self) -> &str {
        core::str::from_utf8(
            &self.name[..self
                .name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(self.name.len())],
        )
        .unwrap()
    }

    fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        if bytes.len() <= self.name.len() {
            self.name[..bytes.len()].copy_from_slice(bytes);
        }
    }

    fn get_mode(&self) -> &str {
        core::str::from_utf8(
            &self.mode[..self
                .mode
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(self.mode.len())],
        )
        .unwrap()
    }

    fn set_mode(&mut self, mode: &str) {
        let bytes = mode.as_bytes();
        if bytes.len() <= self.mode.len() {
            self.mode[..bytes.len()].copy_from_slice(bytes);
        }
    }

    fn get_size(&self) -> usize {
        oct2int(&self.size, self.size.len())
    }

    fn set_size(&mut self, size: usize) {
        // ファイルサイズを8進数文字列に変換して上書き
        int2oct(size, &mut self.size);
    }

    fn get_checksum(&self, disk: &[u8; DISK_MAX_SIZE], offset: usize) -> usize {
        // チェックサムを計算
        let mut checksum = b' ' as usize * self.checksum.len();
        for i in 0..core::mem::size_of::<TarHeader>() {
            checksum += disk[offset + i] as usize;
        }

        checksum
    }

    fn set_checksum(&mut self, checksum: usize) {
        // チェックサムを8進数で設定
        int2oct(checksum, &mut self.checksum);
    }

    fn get_magic(&self) -> &str {
        core::str::from_utf8(
            &self.magic[..self
                .magic
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(self.magic.len())],
        )
        .unwrap()
    }

    fn set_magic(&mut self, magic: &str) {
        let bytes = magic.as_bytes();
        if bytes.len() <= self.magic.len() {
            self.magic[..bytes.len()].copy_from_slice(bytes);
        }
    }

    fn get_version(&self) -> &str {
        core::str::from_utf8(
            &self.version[..self
                .version
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(self.version.len())],
        )
        .unwrap()
    }

    fn set_version(&mut self, version: &str) {
        let bytes = version.as_bytes();
        if bytes.len() <= self.version.len() {
            self.version[..bytes.len()].copy_from_slice(bytes);
        }
    }

    fn set_data(&mut self, file: &File) {
        unsafe {
            let data_ptr =
                (self as *mut TarHeader as *mut u8).add(core::mem::size_of::<TarHeader>());
            let data_slice = core::slice::from_raw_parts_mut(data_ptr, file.size);
            data_slice.copy_from_slice(&file.data[0..file.size]);
        }
    }
}

pub struct FileSystem<'a> {
    files: [File; FILES_MAX],
    disk: [u8; DISK_MAX_SIZE],
    device: Device<'a>,
}

impl<'a> FileSystem<'a> {
    pub fn new(device: Device<'a>) -> Self {
        let mut fs = FileSystem {
            files: core::array::from_fn(|_i| File::new()),
            disk: [0; DISK_MAX_SIZE],
            device: device,
        };

        // ディスクからデータを読み込む
        for sector in 0..(DISK_MAX_SIZE / SECTOR_SIZE) {
            let offset = sector * SECTOR_SIZE;
            fs.device
                .read_write_disk(&mut fs.disk[offset..], sector, false);
        }

        crate::common::println!("read {} bytes from disk", DISK_MAX_SIZE);

        let mut offset = 0;
        for i in 0..fs.files.len() {
            unsafe {
                // TARヘッダーへの参照を取得
                let header = &mut (*(&mut fs.disk[offset] as *mut u8 as *mut TarHeader));

                // ヘッダの名前が空ならループを抜ける
                if header.is_empty() {
                    break;
                }

                // magicフィールドが "ustar" かチェック
                let magic_str = header.get_magic();
                if magic_str != "ustar" {
                    panic!("invalid tar header: magic={}", magic_str);
                }

                // ファイル構造体を設定
                let file = &mut fs.files[i];
                file.setup(&header);

                // ファイル情報を表示
                crate::common::println!("file: {}, size={}", file.get_name(), file.size);

                offset += crate::memory::align_up(
                    core::mem::size_of::<TarHeader>() + file.size,
                    SECTOR_SIZE,
                );
            }
        }

        fs
    }

    pub fn flush(&mut self) {
        unsafe {
            // files変数の各ファイルの内容をdisk変数に書き込むために、0で初期化
            self.init_disk();

            let mut offset = 0;
            for i in 0..self.files.len() {
                let file = &self.files[i];
                if !file.in_use {
                    continue;
                }

                // ディスクの適切な位置にTARヘッダーを配置
                let header = &mut (*(&mut self.disk[offset] as *mut u8 as *mut TarHeader));

                // ヘッダーを0で初期化
                core::ptr::write_bytes(
                    header as *mut TarHeader as *mut u8,
                    0,
                    core::mem::size_of::<TarHeader>(),
                );

                // 文字列フィールドを設定
                header.set_name(file.get_name());
                header.set_mode("000644");
                header.set_magic("ustar");
                header.set_version("00");

                header.type_flag = b'0';
                header.set_size(file.size);
                header.set_checksum(header.get_checksum(&self.disk, offset));

                // ファイルデータをコピー
                header.set_data(file);

                offset += crate::memory::align_up(
                    core::mem::size_of::<TarHeader>() + file.size,
                    SECTOR_SIZE,
                );
            }

            // disk変数の内容をディスクに書き込む
            for sector in 0..(DISK_MAX_SIZE / SECTOR_SIZE) {
                let off = sector * SECTOR_SIZE;
                self.device
                    .read_write_disk(&mut self.disk[off..], sector, true);
            }

            crate::common::println!("wrote {} bytes to disk", DISK_MAX_SIZE);
        }
    }

    fn init_disk(&mut self) {
        self.disk = [0; DISK_MAX_SIZE];
    }

    pub fn lookup(&mut self, filename: &[u8]) -> Option<&mut File> {
        for i in 0..FILES_MAX {
            let file = &self.files[i];
            if file.get_name() == core::str::from_utf8(filename).unwrap() {
                return Some(&mut self.files[i]);
            }
        }

        None
    }
}

#[derive(Debug)]
pub struct File {
    in_use: bool,         // このファイルエントリが使われているか
    pub name: [u8; 100],  // ファイル名
    pub data: [u8; 1024], // ファイルの内容
    pub size: usize,      // ファイルサイズ
}

impl File {
    fn new() -> Self {
        File {
            in_use: false,
            name: [0; 100],
            data: [0; 1024],
            size: 0,
        }
    }

    fn get_name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name.iter().position(|&c| c == 0).unwrap()])
            .unwrap_or("")
    }

    fn setup(&mut self, header: &TarHeader) {
        self.in_use = true;
        self.name.copy_from_slice(&header.name);

        let file_size = oct2int(&header.size, header.size.len());
        self.size = file_size;

        unsafe {
            // ファイルのデータフィールドにコピー
            // header の直後のデータ部分を指す
            let data_ptr =
                (header as *const TarHeader as *const u8).add(core::mem::size_of::<TarHeader>());
            let data_slice = core::slice::from_raw_parts(data_ptr, file_size);
            self.data[..file_size].copy_from_slice(&data_slice[..file_size]);
        }
    }
}

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
