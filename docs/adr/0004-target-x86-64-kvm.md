---
title: 基準ターゲットのx86_64（QEMU/KVM）への変更とベアメタル起動方針
status: proposed
date: 2026-10-31
kind: adr
depends-on:
  - "0001"
  - "0002"
---

# ADR-000004: 基準ターゲットのx86_64（QEMU/KVM）への変更とベアメタル起動方針

- **更新日**: 2026-11-15（ロードマップレビューを受けて決定2〜5を改訂し、決定6を追加。前回の更新は 2026-11-01）

## コンテキスト

[ADR-000001（憲法）](0001-constitution.md) 第3条4項および第7条2項は、本プロジェクトの基準ターゲットを `riscv64`（QEMU `virt` マシン）と定めていた。

しかし、憲法が掲げる第3の動機（「WASMの『安全な実行』『モジュラー性』『高速な立ち上がり』を、宣伝文句としてではなく、自分の実装と計測で検証可能な仮説として扱う」）および Phase 3（複数WASMモジュール間のIPCと、サンドボックス境界コストの計測）の実験設計を具体化する過程で、以下の構造的課題が明確になった。

1. **エミュレーション（QEMU TCG）による計測の歪み**: 開発ホスト（`x86_64` Linux）上で `qemu-system-riscv64` を実行する場合、QEMU の動的バイナリ変換（TCG）およびソフトウェア MMU（SoftMMU）が介在する。そのため、ハードウェア本来のパイプライン、キャッシュ階層、TLB フラッシュ、および関数呼び出し境界が支払う実クロックサイクル数を直接測定できない。
2. **先行研究および一次資料との対照性**: 単一アドレス空間（SAS）や言語・検証ベース隔離を扱う先行研究（[Theseus (OSDI 2020)](https://www.usenix.org/system/files/osdi20-boos.pdf)、[RedLeaf (OSDI 2020)](https://www.usenix.org/system/files/osdi20-narayanan.pdf)、[Li et al. (PLOS '21)](https://dl.acm.org/doi/10.1145/3477113.3487275)、[Mewz (arXiv:2411.01129)](https://arxiv.org/abs/2411.01129)、[Munal OS](https://github.com/Askannz/munal-os)）の実装および境界コストの計測値は、`x86_64` 上のタイムスタンプカウンタ（`rdtsc` / `rdtscp`）を基準としている。同一 ISA 上で実装・計測することで、先行研究の知見や数値と直接対照できる。
3. **Phase 0（`modal-one-core`）の完全なポータビリティ**: [ADR-000002](0002-crate-layout-and-execution-representation.md) および [ADR-000003](0003-verification-and-ci.md) により、コアインタプリタは `#![no_std]` かつ `#![forbid(unsafe_code)]` の純粋な Rust で実装され、JIT を持たない。したがって、ターゲット ISA の変更は `modal-one-core` の設計・実装に一切影響しない。

一方で、`x86_64` には 16-bit Real Mode からの歴史的なブート階層や、64-bit Long Mode におけるページング必須制約が存在する。これらによって憲法第2条の不変条件（「作者が全行を理解している状態を保つ」）や第5条（「実行パスは自分の手で書く」）が損なわれないよう、Phase 1 のベアメタル構成と Phase 3 の計測プロトコルを一次仕様に基づいて定義する必要がある。

## 決定事項

### 1. 憲法第3条4項・第7条2項の部分改正（憲法第8条に基づく手続き）

[ADR-000001](0001-constitution.md) 第8条に従い、正本である `0001-constitution.md` は直接編集せず、本 ADR によって以下の条項を部分改正（supersede）する。なお、DocDag の `supersedes` エッジは対象文書全体を `superseded` 状態へ遷移させるため、グラフ上は `depends-on` で接続し、本文にて改正内容を記録する。

- **改正対象1 — 第3条4項（実機ハードウェアのサポート）**:
  - **変更前**: 基準環境はQEMU virt（riscv64）とし、物理ボード・デバイスドライバの対応は行わない。ただし本項はISAには適用されない。RISC-V ISA自体は本プロジェクトの中心的な対象である（第7条）。
  - **変更後**: 基準環境は QEMU + KVM（`x86_64`）とし、物理ボード・固有デバイスドライバの網羅的な対応は行わない。ただし本項は ISA には適用されない。`x86_64` ISA 自体は本プロジェクトの中心的な対象である（第7条）。
- **改正対象2 — 第7条2項（ターゲット）**:
  - **変更前**: 基準ターゲットはriscv64（QEMU virtマシン）とする。
  - **変更後**: 基準ターゲットは `x86_64`（QEMU + KVM、OVMF UEFI ファームウェア環境）とする。
- **ロードマップ見取り図（Phase 1）の読み替え**:
  - 「QEMU/KVM `x86_64` 上のベアメタルへ移植（ブート・シリアル出力・アロケータのみのナノカーネル）」と読み替える。

### 2. Phase 1 のブート方式 — ゼロ依存 UEFI 直起動と段階的掌握

16-bit Real Mode 等のレガシー互換処理に時間を奪われず、かつ憲法第5条（「実行パスでは外部の実装クレートを使用しない」）を守るため、Phase 1（`kernel` クレート）は以下の構成を採る。

1. **ターゲット仕様とファームウェア**:
   - Rust 標準の Tier 2 ターゲット `x86_64-unknown-uefi` と QEMU + OVMF（UEFI ファームウェア）を用いる。`rust-lld` によりカスタムリンカスクリプトなしで PE32+（`.efi`）実行ファイルが生成され、エントリポイントは `#[unsafe(no_mangle)] pub extern "efiapi" fn efi_main(...)`（Microsoft x64 呼び出し規約）となる。このターゲットは内部的に Windows 系の LLVM ターゲット（`x86_64-unknown-windows`）を使うので、`extern "C"` も同じ MS x64 規約になる。それでも UEFI の関数ポインタ型には `extern "efiapi"` を使う。ターゲットごとの規約差を Rust 側に吸収させ、意図を型に残すためである。
   - `x86_64-unknown-uefi` はターゲット仕様で `"disable-redzone": true`（Red Zone 無効）が既定となっている（`rustc 1.100.0` の target-spec-json で 2026-11-15 に確認）。MS x64 ABI にはそもそも red zone が無い。そのため、単一特権レベル（Ring 0）のまま CPU 例外・割り込みが発生して現在の `RSP` に例外フレームがプッシュされても、スタック下方のローカル変数が破壊されない。
   - [UEFI Specification 2.10 §2.3.4 (x64 Platforms)](https://uefi.org/specs/UEFI/2.10/02_Overview.html#x64-platforms) により、`efi_main` 到達時点で CPU は 64-bit Long Mode（Ring 0）・方向フラグ `DF = 0`・初期スタック確保済み（128 KiB 以上、16 バイト整列）・タイマー割り込み有効（`RFLAGS.IF = 1`）の状態となる。恒等マッピング（$\text{VA} = \text{PA}$）されているのは、**UEFI メモリマップに記載された領域**である。それ以外の領域のマッピングは未定義であり、「全物理メモリが恒等マップされている」とは仮定しない。
2. **外部クレートなしの UEFI 離脱と物理メモリ選別**:
   - `uefi` や `bootloader`、`x86_64` 等の外部クレートは使用しない。`kernel` 内に最小限の `EFI_SYSTEM_TABLE` / `EFI_BOOT_SERVICES` / `EFI_MEMORY_DESCRIPTOR`（`#[repr(C)]` 構造体）を手書き定義する（先行事例: [Munal OS](https://github.com/Askannz/munal-os)）。
   - [UEFI Specification 2.10 §7.4](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#efi-boot-services-exitbootservices) に従い、`ExitBootServices()` には直前の `GetMemoryMap()` が返した `MapKey` を渡す。`MapKey` が古いと `EFI_INVALID_PARAMETER` が返る。仕様は、最初の呼び出しでファームウェアがブートサービスを部分的に停止してよいとし、それ以降はメモリ確保サービス以外を呼ぶべきでないと定める。再確保するとメモリマップがまた変わるので、実装は次のようにする。メモリマップ用バッファは、事前に「必要サイズ + 記述子 8 個分」程度の余裕をもって確保しておく。再試行では `GetMemoryMap()` と `ExitBootServices()` だけを呼ぶ。メモリマップ走査時は構造体サイズではなくファームウェアが返した `DescriptorSize` バイト単位でポインタを進める。
   - `ExitBootServices()` 成功直後に `cli` 命令で割り込みを無効化（`RFLAGS.IF = 0`）し、Step 4 で自前 IDT を据えるまでファームウェア由来のタイマー割り込みを遮断する。
   - `ExitBootServices()` 直後は、実行中の初期スタック（`RSP`）および OVMF のページテーブル（`CR3`、EDK2 では Read-Only かつ `CR0.WP = 1` で保護されうる）が `EfiBootServicesData` 領域内に残っている。離脱後は `EfiLoaderCode`／`EfiLoaderData` と `EfiBootServicesCode`／`EfiBootServicesData` も再利用できるが、使用中のイメージ・スタック・ページテーブルがそこに含まれる。自己上書きを防ぐため、初期ヒープには **`EfiConventionalMemory`（タイプ 7）かつ物理アドレス `0x10_0000`（1 MiB）以上** の空き領域のみを使用する。
3. **段階的なハードウェア掌握（Phase 1 の縦切り）**:
   - **Step 1**: COM1 シリアルポート（16550A UART、I/O ポート `0x3F8`）の最小初期化・ポーリング文字出力（LSR `0x3FD` の `THRE` ビット確認）と、QEMU 自動終了デバイス（`isa-debug-exit`、終了コード変換式 `(code << 1) | 1`）を数行のインラインアセンブリ（`out` / `in` 命令）で手書きする。`isa-debug-exit` の I/O ポートは、QEMU を `-device isa-debug-exit,iobase=0xf4,iosize=0x04` で起動したときに `0xF4` になる。デバイスの既定値は `iobase=0x501`・`iosize=0x02` である（QEMU `hw/misc/debugexit.c`）。本 ADR と関連文書が `0xF4` と書くときは、この起動オプションを前提にする。
   - **Step 2**: `ExitBootServices()` と `cli` でファームウェアを停止し、選別した `EfiConventionalMemory` 領域上に `#[global_allocator]`（まずは Bump Allocator）を実装する。
   - **Step 3**: `modal-one-core` をリンクし、Phase 0 と同一の最小 Wasm モジュール（`answer.wasm`）をベアメタル上で実行して `42` をシリアル出力する。
   - **Step 4**: 動く状態を保ったまま、次の順でファームウェア由来の構造から独立する。IDT を設置する前に起きた例外はトリプルフォールトになるので、IDT と TSS の設置を、自前ページテーブルへの切替より先に行う。
     - (a) 自前の最小 GDT（Null / 64-bit Kernel Code `0x08` / Kernel Data `0x10` / 64-bit TSS）をロードし、`lretq`（far return）で `CS` を、続けてデータセレクタを明示的にリロードする。64-bit の TSS 記述子は 16 バイトを占める。TSS は `ltr` でロードする。
     - (b) 最小 IDT（CPU 例外ハンドラ）を登録する。`#DF`（ベクタ 8）と `#PF`（ベクタ 14）には、TSS の IST で専用スタックを割り当てる（決定3）。
     - (c) 2MiB HugePage（PDE `PS = 1`）を用いた静的ページテーブルを構築して `CR3` にロードする。PML4 1 ページ + PDPT 1 ページ + PD 4 ページ = 計 6 ページ（24 KiB）で恒等マップできるのは**先頭 4 GiB だけ**である。4 GiB を超える RAM、OVMF が高位に配置し得る 64-bit PCI MMIO、4 GiB 超にロードされたイメージ自身は覆われない。そのため、`GetMemoryMap()` で得た最大物理アドレスまで PD を追加するか、CPU が `pdpe1gb` を公開している場合に限り 1 GiB ページを使う。
     - (d) 自前のカーネルスタックへ切り替え、その下端にガードページ（非存在ページ）を置く（決定3）。4 KiB 単位で非存在にするには、その 2 MiB 領域を PT 1 枚で 4 KiB ページに分割する。0 番地のヌル参照を検出するために、先頭 2 MiB を同じ方法で分割して 0 番地のページを非存在にしてもよい。
4. **実機起動は Phase 1 の完了条件に含めない**:
   - x86_64 実機で起動すると、QEMU + OVMF の前提が次の点で崩れる。(1) COM1（`0x3F8`）のシリアルポートが無い機種が多い（GOP フレームバッファ等の別経路が要る）。(2) `isa-debug-exit` が存在しない。(3) メモリマップは機種ごとに大きく異なり、4 GiB 超へのロードが普通に起こる。(4) Secure Boot が有効だと未署名の EFI アプリは起動しない。(5) 割り込みコントローラの初期状態や SMI によるタイミングの乱れが QEMU より大きい。
   - これらへの対応は Phase 1 の完了条件に含めない。試す場合は Phase 1 の後の独立した小段階として扱う。憲法第3条4項（決定1で改正）の「物理ボード・固有デバイスドライバの網羅的な対応は行わない」は変更しない。

### 3. 単一アドレス空間（SAS）・単一特権レベル（SPL）とスタック安全性の整合

`x86_64` の 64-bit Long Mode は、CPU 仕様上ページングの有効化（`CR0.PG = 1`, `CR4.PAE = 1`, `IA32_EFER.LME = 1`）を必須とする。憲法第2条（「WebAssemblyの検証可能性を唯一の隔離機構とする」）との整合を保つため、以下の不変条件を置く。

- **単一特権レベル（SPL）**: カーネルおよび Wasm インタプリタはすべて Ring 0（`CPL = 0`）で実行する。
- **固定恒等マッピング（SAS）**: ページテーブルは物理アドレスと仮想アドレスを 1:1 で対応づける恒等マッピング（2MiB HugePage。ガードページを置く領域だけ 4 KiB ページに分割する）の 1 組のみを起動時に構成し、通常実行中は `CR3` の書き換え（アドレス空間の切り替え・TLB フラッシュ）を一切行わない。
- **Wasm による空間的隔離**: Wasm モジュール間の空間的隔離は MMU ではなく、Wasm バリデータの静的型検証とインタプリタの線形メモリ境界検査によってのみ実現する。
- **非再帰コールフレーム**: `modal-one-core` の関数呼び出しはホスト Rust のコールスタック再帰を使わず、ヒープ上の明示的なフレーム配列（`Vec<CallFrame>`）で管理する。これにより、Wasm の呼び出し深さに比例して CPU スタック（`RSP`）が伸びる経路は無くなる。Wasm 側の深さは実装定義の上限（[ADR-000002](0002-crate-layout-and-execution-representation.md) 決定7）で検査し、超過は Safe Rust の `Trap` として確定的に処理する。
- **スタック溢れは、検出して止める**: 非再帰コールフレームだけではスタック溢れを防げない。デコーダやバリデータ、深い Rust の呼び出し連鎖、例外ハンドラの実行中にも `RSP` は伸び得る。UEFI が用意する恒等マップにはガードページが無いので、溢れは例外にならず、隣接するメモリを静かに破壊する。そこで Phase 1 では次の三段を設ける。
  1. 自前のカーネルスタックを確保し、その下端の 4 KiB を自前ページテーブルで非存在にする（ガードページ）。
  2. 64-bit TSS を設置し、`#DF`（ベクタ 8）と `#PF`（ベクタ 14）に IST の専用スタックを割り当てる。
  3. `#PF` ハンドラは、`CR2` がガードページ内なら "kernel stack overflow" をシリアルに出力し、`isa-debug-exit` で失敗コードを返して停止する。

  当初の本 ADR は「非再帰コールフレームにより、TSS の IST 等に頼らず Triple Fault を防ぐ」としていた。この主張は撤回する。

### 4. ベアメタル環境での浮動小数点（`soft-float` と IEEE 754 ビット演算）

Rust の `x86_64-unknown-uefi` ターゲットは、割り込み時のベクトルレジスタ（`xmm0`〜`xmm15`）退避コストと初期化前の `#UD` 例外を防ぐため、デフォルトで `"-mmx,-sse,+soft-float"`（`rustc-abi = "softfloat"`）となっている。

- **初期実装（Phase 0〜2）**: 標準ターゲットの `+soft-float` 設定をそのまま用い、カスタムターゲット JSON は作成しない。基本的な四則演算は `compiler_builtins` のソフトウェア実装で処理され、`core` の stable API に存在しない `f32.sqrt` / `f64.sqrt` および丸め・最小最大・NaN 正規化は、ハードウェア命令や外部クレート（`libm`）に依存しない Safe Rust の整数ビット演算（`to_bits` / `from_bits` と IEEE 754 開平アルゴリズム等）で手書きし、ホスト（`x86_64-unknown-linux-gnu`）とベアメタル（`x86_64-unknown-uefi`）の間でビット単位の決定論的一致を保つ。`compiler_builtins` のソフトフロートと SSE2 は、どちらも IEEE 754 の正しく丸められた演算なので、NaN 以外の結果ではビットの一致が期待できる。NaN のビットには保証が無いので、値をビット列で保持し、算術結果の NaN を正の canonical NaN に置き換えることで吸収する（[ADR-000002](0002-crate-layout-and-execution-representation.md) 決定6）。
- **stable で使える範囲（`rustc 1.100.0` 時点）**: `core::f32::math`／`core::f64::math` の `floor`／`ceil`／`round`／`round_ties_even`／`trunc`／`sqrt`／`mul_add`／`fract`／`div_euclid`／`rem_euclid`／`powi`／`cbrt` は nightly 限定である（`core_float_math`、tracking issue [#137578](https://github.com/rust-lang/rust/issues/137578)）。`abs`／`copysign`／`signum`／`to_bits`／`from_bits` は `core` で安定している。`#[target_feature]` を付けても、このターゲットの soft-float は解除されない。
- **性能実験（Phase 3 以降）**: `CR0`（`EM = 0`, `MP = 1`）および `CR4`（`OSFXSR = 1`, `OSXMMEXCPT = 1`）による SSE2 有効化とハードウェア命令（`sqrtss` / `sqrtsd` 等）の比較が必要になった時点で、個別実験または追加 ADR として扱う。

### 5. Phase 3 におけるサイクル計測プロトコルと実験設計

Phase 3 の境界コストおよび IPC 計測は、KVM が使える x86_64 実機上の `qemu-system-x86_64 -enable-kvm -cpu host,+invtsc` で行う。2026-11-15 のレビューは、手持ちの AMD 機（Ryzen 7 3800X = Zen 2、Ryzen 7 6800H = Zen 3+、Ryzen AI 9 HX 370 = Zen 5）を計測機に想定している。当初の本 ADR は Intel SDM（VMX）だけを根拠にしていたので、AMD（SVM）でも成り立つ手順に改める。TCG 上の `rdtsc` の値は、サイクル計測として意味を持たない。Phase 3 の数値はすべて KVM 上で取得する。

1. **`CPUID` をフェンスに使わない**:
   - Intel VMX では `CPUID` は無条件に VM-Exit する（Intel SDM Vol. 3C §26.1.2。旧版では §25.1.2）。AMD SVM では傍受は VMCB の設定次第だが、KVM は常に `CPUID` を傍受する（Linux `arch/x86/kvm/svm/svm.c` の `init_vmcb()`）。したがって、どちらのベンダでも `CPUID` を計測用のバリア命令に使わない。
   - KVM の SVM 実装は、既定で `RDTSC` を傍受しない。`RDTSCP` も、ゲストの CPUID に `RDTSCP` があれば傍受しない。
2. **`LFENCE` の直列化を確認してから使う**:
   - AMD では、`LFENCE` がディスパッチを直列化することは自明ではない。AMD のホワイトペーパー "Software Techniques for Managing Speculation on AMD Processors"（緩和策 G-2）によれば、family 10h/12h/14h/15h/16h/17h では MSR `C001_1029`（`DE_CFG`）のビット1を立てて初めて `LFENCE` がディスパッチ直列化命令になる。Zen 2（family 17h）はこの対象に入る。新しい世代は、CPUID `Fn8000_0021_EAX` のビット2（`LfenceAlwaysSerializing`）で常時直列化を報告する。KVM は、ホストが `LFENCE_RDTSC` を持つ場合にこのビットをゲストへ見せる。ゲストによる `DE_CFG` への書き込みは仮想値であり、ハードウェアには反映されない。
   - Intel の CPU では、`LFENCE` が先行命令の完了を待つことを SDM（Vol. 2A の `LFENCE` の項）が定めているので、以下の確認は AMD の CPU に対して行う。
   - 起動時にゲスト内で CPUID `0x8000_0021` の EAX ビット2を読み、ログに残す。立っていれば開始側を `lfence; rdtsc; lfence`（先行ストアの完了まで待つ場合は `mfence; lfence; rdtsc; lfence`）とする。立っていなければ `mfence; rdtsc` を使う。
   - 終了側は `rdtscp; lfence` とする。
3. **TSC と実行環境の条件を固定して記録する**:
   - CPUID `0x8000_0007` の EDX ビット8（`TscInvariant`）を確認する。QEMU は `-cpu host,+invtsc` で公開するが、`invtsc` はマイグレーションと savevm を阻止する。
   - vCPU スレッドを固定コアにピン留めする。ホストでは cpufreq ガバナを performance に固定し、可能ならブーストを無効にする。TSC は名目周波数で進むので、コアクロックのサイクル数とは一致しない。この点も報告に書く。
4. **統計の取り方**:
   - ウォームアップを捨てる。N 回まとめて計測して割る。`rdtsc` 自体のオーバーヘッドは空ループで測って引く。
   - 数千試行の分布（最小値、中央値、p90、p99）を報告する。最小値は下限、中央値は典型として使い分ける。バッファサイズ別には傾きを報告する。
   - ホスト割り込み、SMI、周波数変動、TSC オフセット／スケーリングといったノイズは排除しきれない。結果は単一の数値ではなく分布として報告する。
   - KVM + NPT 下の `mov cr3` は、ベアメタルの基準として楽観寄りにも悲観寄りにもなり得る。この限界を報告に書く。
5. **インタプリタ方式における比較実験の切り分け**:
   - 先行研究の数値はネイティブコード間の境界コストである。Modal One のインタプリタ実行時間と単純な絶対値で比較すると、ディスパッチループ自体のコストと混同される。
   - [RedLeaf (OSDI 2020)](https://www.usenix.org/system/files/osdi20-narayanan.pdf) は、ドメイン間呼び出しを 124 サイクル、`RRef<T>` を渡す場合を 141 サイクルと報告する（Table 1。本 ADR では論文本文との照合が未了）。
   - [Li et al. (PLOS '21)](https://dl.acm.org/doi/10.1145/3477113.3487275) の測定条件は、Intel Haswell（Xeon E5-2660 v3）のベアメタル、`mitigations=off` である。同論文は、unwind 版の RedLeaf IPC（`RRef<T>` 引数1個の所有権移動を含む）を 1 交差 16〜18 サイクルと報告する。ハードウェア隔離の IPC は、拡張レジスタを退避しない構成で 111〜164 サイクル、`fxsave`／`fxrstor` による退避・復元が 216〜242 サイクルを加えて計 327〜406 サイクルである。
   - そのため、Modal One では **(a) 同一インスタンス内 `call` とインスタンス間 import `call` の純粋な境界差分 $\Delta_{\text{IPC}}$**、**(b) バッファ長に対するコピー渡しと共有・ケイパビリティ受け渡しのスケーリング差**、および **(c) 同一カーネル内で実測するハードウェア隔離操作（`mov cr3` による TLB フラッシュや例外往復 `int`/`iretq`）のサイクル数** を測定し、単一アドレス空間・検証ベース隔離の構造的コストを定量化する。

### 6. 開発環境の三層

- **日常開発**（Phase 0 全体と Phase 1 の機能確認）: KVM を使えないホスト（ARM64 機など）では、`qemu-system-x86_64`（TCG）＋ OVMF で行う。TCG は KVM より桁違いに遅いが、機能確認には足りる。本ターゲットは SSE を使わない（soft-float）ので、TCG の命令サポート差にも左右されにくい。
- **KVM での確認と計測**: TCG で通っても KVM や実機で落ちる場合がある。Phase 1 の各 Step の完了時には、KVM が使える機械でも同じイメージを起動する。Phase 3 の計測は決定5に従い KVM 上で行う。
- **CI**: `/dev/kvm` の有無を実行時に検出し、あれば KVM、なければ TCG で同じテストを回す。CI は TCG だけでも成立するように組む（[ADR-000003](0003-verification-and-ci.md) 決定4）。

## 影響・トレードオフ

**得るもの**:
- KVM ハードウェア仮想化と `lfence` / `rdtscp` により、バイナリ変換エミュレータを介さず、計測区間に VM-Exit を挟まない実測が可能になる。ノイズは残るので、結果は分布として報告する（決定5）。
- [Theseus](https://www.usenix.org/system/files/osdi20-boos.pdf)、[RedLeaf](https://www.usenix.org/system/files/osdi20-narayanan.pdf)、[Mewz](https://arxiv.org/abs/2411.01129)、[Munal OS](https://github.com/Askannz/munal-os) 等の先行研究および Rust の `x86_64` OS 開発資料と同じアーキテクチャ上で設計・数値を対照できる。
- `x86_64-unknown-uefi`（Red Zone 無効・PE32+ 直リンク）と OVMF UEFI 直起動により、外部クレート・カスタムリンカスクリプト・ターゲット JSON 依存ゼロかつ最小限のインラインアセンブリ（`in`/`out`/`cli`/`lgdt`/`lidt`/`mov cr3`/`rdtscp` 等）で 64-bit カーネルを段階的に構築できる。

**受け入れるもの**:
- RISC-V の `satp = 0`（MMU 完全無効）とは異なり、64-bit Long Mode を維持するためだけに最小限の GDT（とセレクタ再ロード）および固定の恒等マッピングページテーブル（先頭 4 GiB で 24 KiB。メモリマップに応じて増える）を管理する必要がある。スタック溢れを検出するために、TSS・IST・ガードページも Phase 1 で用意する（決定3）。
- `ExitBootServices()` 時のメモリマップ取得リトライ、`DescriptorSize` 走査、`EfiConventionalMemory` の選別、および IDT の分割オフセット構造や Port I/O（`0x3F8` / `0xF4`）など、`x86_64` + UEFI 固有の仕様を Step 1・Step 2・Step 4 で学ぶ必要がある。

## 関連するADR

- [ADR-000001: Modal One 憲法 — プロジェクト定義と基本原則](0001-constitution.md)（第3条4項・第7条2項を本 ADR により部分改正）
- [ADR-000002: クレート構成・実装順序・実行表現](0002-crate-layout-and-execution-representation.md)
- [ADR-000003: 検証体制とCI](0003-verification-and-ci.md)（決定4: KVM 検出と TCG フォールバック、決定8: 浮動小数点の検証）
