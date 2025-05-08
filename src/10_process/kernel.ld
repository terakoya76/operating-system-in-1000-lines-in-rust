ENTRY(boot) /* カーネルのエントリポイントはboot関数 */

/* SECTIONS内で各セクションの配置を定義 */
SECTIONS {
    /*
      ベースアドレスは0x80200000
      `.`は現在のアドレスを表す変数のようなもの
       *(.text) 等でデータが配置されるたびに自動的に加算されていく
    */
    . = 0x80200000;

    /*
      必ず.text.bootセクションを先頭にする
      .text、.rodata、.data、.bssの順に各セクションを配置
    */
    /* .text: コード領域 */
    .text :{
        KEEP(*(.text.boot));
        /*
          *(.text .text.*) のような記述は、
          全ファイル中 (`*`) の`.text`と`.text.`で始まる名前のセクションをそこに配置するという意味になる
        */
        *(.text .text.*);
    }

    /* .rodata: 定数データ領域。読み取り専用 */
    .rodata : ALIGN(4) {
        *(.rodata .rodata.*);
    }

    /* .data: 読み書き可能データ領域 */
    .data : ALIGN(4) {
        *(.data .data.*);
    }

    /*
      .bss: 読み書き可能データ領域
      .dataと違い、初期値がゼロの変数を配置
    */
    .bss : ALIGN(4) {
        __bss = .; /* __bssというシンボルに現在のアドレスを割り当てる */
        *(.bss .bss.* .sbss .sbss.*);
        __bss_end = .;
    }

    . = ALIGN(4);
    . += 128 * 1024; /* 現在のアドレスから128KB進める */
    __stack_top = .;

    . = ALIGN(4096);
    __free_ram = .;
    . += 64 * 1024 * 1024; /* 64MB */
    __free_ram_end = .;
}
