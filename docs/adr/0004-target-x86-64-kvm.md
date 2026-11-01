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

- **更新日**: 2026-11-01

## コンテキスト

[ADR-000001（憲法）](0001-constitution.md) 第3条4項および第7条2項は、本プロジェクトの基準ターゲットを `riscv64`（QEMU `virt` マシン）と定めていた。

しかし、憲法が掲げる第3の動機（「WASMの『安全な実行』『モジュラー性』『高速な立ち上がり』を、宣伝文句としてではなく、自分の実装と計測で検証可能な仮説として扱う」）および Phase 3（複数WASMモジュール間のIPCと、サンドボックス境界コストの計測）の実験設計を具体化する過程で、以下の構造的課題が明確になった。

1. **エミュレーション（QEMU TCG）による計測の歪み**: 開発ホスト（`x86_64` Linux）上で `qemu-system-riscv64` を実行する場合、QEMU の動的バイナリ変換（TCG）およびソフトウェア MMU（SoftMMU）が介在する。そのため、ハードウェア本来のパイプライン、キャッシュ階層、TLB フラッシュ、および関数呼び出し境界が支払う実クロックサイクル数を直接測定できない。
2. **先行研究および一次資料との対照性**: 単一アドレス空間（SAS）や言語・検証ベース隔離を扱う先行研究（[Theseus (OSDI 2020)](https://www.usenix.org/system/files/osdi20-boos.pdf)、[RedLeaf (OSDI 2020)](https://www.usenix.org/system/files/osdi20-narayanan.pdf)、[Li et al. (PLOS '21)](https://dl.acm.org/doi/10.1145/3477113.3487270)、[Mewz (arXiv:2411.01129)](https://arxiv.org/abs/2411.01129)、[Munal OS](https://github.com/Askannz/munal-os)）の実装および境界コストの計測値は、`x86_64` 上のタイムスタンプカウンタ（`rdtsc` / `rdtscp`）を基準としている。同一 ISA 上で実装・計測することで、先行研究の知見や数値と直接対照できる。
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
   - Rust 標準の Tier 2 ターゲット `x86_64-unknown-uefi` と QEMU + OVMF（UEFI ファームウェア）を用いる。`rust-lld` によりカスタムリンカスクリプトなしで PE32+（`.efi`）実行ファイルが生成され、エントリポイントは `#[unsafe(no_mangle)] pub extern "efiapi" fn efi_main(...)`（Microsoft x64 呼び出し規約）となる。
   - `x86_64-unknown-uefi` はターゲット仕様で `"disable-redzone": true`（Red Zone 無効）が既定となっているため、単一特権レベル（Ring 0）のまま CPU 例外・割り込みが発生して現在の `RSP` に例外フレームがプッシュされても、スタック下方のローカル変数が破壊されない。
   - [UEFI Specification 2.10 §2.3.4 (x64 Platforms)](https://uefi.org/specs/UEFI/2.10/02_Overview.html#x64-platforms) により、`efi_main` 到達時点で CPU は 64-bit Long Mode（Ring 0）・全メモリ恒等マッピング（$\text{VA} = \text{PA}$）有効・方向フラグ `DF = 0`・初期スタック確保済み（最低 128 KiB）・タイマー割り込み有効（`RFLAGS.IF = 1`）の状態となる。
2. **外部クレートなしの UEFI 離脱と物理メモリ選別**:
   - `uefi` や `bootloader`、`x86_64` 等の外部クレートは使用しない。`kernel` 内に最小限の `EFI_SYSTEM_TABLE` / `EFI_BOOT_SERVICES` / `EFI_MEMORY_DESCRIPTOR`（`#[repr(C)]` 構造体）を手書き定義する（先行事例: [Munal OS](https://github.com/Askannz/munal-os)）。
   - [UEFI Specification 2.10 §7.4](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#efi-boot-services-exitbootservices) に従い、`GetMemoryMap()` と `ExitBootServices()` は間に他の Boot Services 呼び出しを挟まず連続実行し、`MapKey` 不一致（`EFI_INVALID_PARAMETER`）時は再取得・再試行する。メモリマップ走査時は構造体サイズではなくファームウェアが返した `DescriptorSize` バイト単位でポインタを進める。
   - `ExitBootServices()` 成功直後に `cli` 命令で割り込みを無効化（`RFLAGS.IF = 0`）し、Step 4 で自前 IDT を据えるまでファームウェア由来のタイマー割り込みを遮断する。
   - `ExitBootServices()` 直後は、実行中の初期スタック（`RSP`）および OVMF のページテーブル（`CR3`、EDK2 では Read-Only かつ `CR0.WP = 1` で保護されうる）が `EfiBootServicesData` 領域内に残っている。自己上書きを防ぐため、初期ヒープには **`EfiConventionalMemory`（タイプ 7）かつ物理アドレス `0x10_0000`（1 MiB）以上** の空き領域のみを使用する。
3. **段階的なハードウェア掌握（Phase 1 の縦切り）**:
   - **Step 1**: COM1 シリアルポート（16550A UART、I/O ポート `0x3F8`）の最小初期化・ポーリング文字出力（LSR `0x3FD` の `THRE` ビット確認）と、QEMU 自動終了デバイス（`isa-debug-exit`、I/O ポート `0xF4`、終了コード変換式 `(code << 1) | 1`）を数行のインラインアセンブリ（`out` / `in` 命令）で手書きする。
   - **Step 2**: `ExitBootServices()` と `cli` でファームウェアを停止し、選別した `EfiConventionalMemory` 領域上に `#[global_allocator]`（まずは Bump Allocator）を実装する。
   - **Step 3**: `modal-one-core` をリンクし、Phase 0 と同一の最小 Wasm モジュール（`answer.wasm`）をベアメタル上で実行して `42` をシリアル出力する。
   - **Step 4**: 動く状態を保ったまま、(a) 自前の最小 GDT（Null / 64-bit Kernel Code `0x08` / Kernel Data `0x10`）のロードと `lretq`（far return）による `CS` およびデータセレクタの明示的リロード、(b) 2MiB HugePage（PDE `PS = 1`）を用いた静的ページテーブル（PML4 1 ページ + PDPT 1 ページ + PD 4 ページ = 計 24 KiB で 0〜4 GiB を恒等マッピング）の構築と `CR3` へのロード、(c) 最小 IDT（CPU 例外ハンドラ）の登録を行い、ファームウェア由来の構造から完全に独立する。

### 3. 単一アドレス空間（SAS）・単一特権レベル（SPL）とスタック安全性の整合

`x86_64` の 64-bit Long Mode は、CPU 仕様上ページングの有効化（`CR0.PG = 1`, `CR4.PAE = 1`, `IA32_EFER.LME = 1`）を必須とする。憲法第2条（「WebAssemblyの検証可能性を唯一の隔離機構とする」）との整合を保つため、以下の不変条件を置く。

- **単一特権レベル（SPL）**: カーネルおよび Wasm インタプリタはすべて Ring 0（`CPL = 0`）で実行する。
- **固定恒等マッピング（SAS）**: ページテーブルは物理アドレスと仮想アドレスを 1:1 で対応づける恒等マッピング（2MiB HugePage）の 1 組のみを起動時に構成し、通常実行中は `CR3` の書き換え（アドレス空間の切り替え・TLB フラッシュ）を一切行わない。
- **Wasm による空間的隔離**: Wasm モジュール間の空間的隔離は MMU ではなく、Wasm バリデータの静的型検証とインタプリタの線形メモリ境界検査によってのみ実現する。
- **非再帰コールフレームによる Ring 0 スタック保護**: Ring 0 単一特権レベルでは、CPU スタック（`RSP`）が溢れると Double Fault（`#DF`）から Triple Fault（即時リセット）に直結する。これをハードウェア機構（TSS の IST 等）に頼らず防ぐため、`modal-one-core` の関数呼び出しはホスト Rust のコールスタック再帰を使わず、ヒープ上の明示的なフレーム配列（`Vec<CallFrame>`）で管理する。これにより CPU スタック消費をゲストの呼び出し深さに対して $O(1)$ に保ち、深さ上限を Safe Rust の `Trap` として確定的に処理する。

### 4. ベアメタル環境での浮動小数点（`soft-float` と IEEE 754 ビット演算）

Rust の `x86_64-unknown-uefi` ターゲットは、割り込み時のベクトルレジスタ（`xmm0`〜`xmm15`）退避コストと初期化前の `#UD` 例外を防ぐため、デフォルトで `"-mmx,-sse,+soft-float"`（`rustc-abi = "softfloat"`）となっている。

- **初期実装（Phase 0〜2）**: 標準ターゲットの `+soft-float` 設定をそのまま用い、カスタムターゲット JSON は作成しない。基本的な四則演算は `compiler_builtins` のソフトウェア実装で処理され、`core` の stable API に存在しない `f32.sqrt` / `f64.sqrt` および丸め・最小最大・NaN 正規化は、ハードウェア命令や外部クレート（`libm`）に依存しない Safe Rust の整数ビット演算（`to_bits` / `from_bits` と IEEE 754 開平アルゴリズム等）で手書きし、ホスト（`x86_64-unknown-linux-gnu`）とベアメタル（`x86_64-unknown-uefi`）の間でビット単位の決定論的一致を保つ。
- **性能実験（Phase 3 以降）**: `CR0`（`EM = 0`, `MP = 1`）および `CR4`（`OSFXSR = 1`, `OSXMMEXCPT = 1`）による SSE2 有効化とハードウェア命令（`sqrtss` / `sqrtsd` 等）の比較が必要になった時点で、個別実験または追加 ADR として扱う。

### 5. Phase 3 における実機サイクル計測プロトコルと実験設計

Phase 3 の境界コストおよび IPC 計測では、`qemu-system-x86_64 -enable-kvm -cpu host,+invtsc`（および単一物理コアへのピン留め）を用い、以下のプロトコルと対照軸を採用する。

1. **VM-Exit を誘発しないシリアライズ計測**:
   - [Intel SDM Vol. 3C §25.1.2](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) により、KVM ゲスト内での `CPUID` 命令は無条件の VM-Exit（数千サイクル）を引き起こすため、バリア命令として `CPUID` は使用しない。
   - [Intel SDM Vol. 2B (`RDTSC` / `RDTSCP` / `LFENCE`)](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) に従い、アウトオブオーダ実行および投機実行による測定歪みを排するため、開始側は `lfence; rdtsc; lfence`（先行ストアの完了まで待つ場合は `mfence; lfence; rdtsc; lfence`）、終了側は `rdtscp; lfence` の組で物理 CPU のクロックサイクル数を測定する。
2. **インタプリタ方式における比較実験の切り分け**:
   - 先行研究（[RedLeaf (OSDI 2020)](https://www.usenix.org/system/files/osdi20-narayanan.pdf) の cross-domain 呼び出し 124 cycles / `RRef<T>` 141 cycles、[Theseus (OSDI 2020)](https://www.usenix.org/system/files/osdi20-boos.pdf)、[Li et al. (PLOS '21)](https://dl.acm.org/doi/10.1145/3477113.3487270)）はネイティブコード間の境界コストであるため、Modal One のインタプリタ実行時間と単純な絶対値で比較するとディスパッチループ自体のコストと混同される。
   - そのため、Modal One では **(a) 同一インスタンス内 `call` とインスタンス間 import `call` の純粋な境界差分 $\Delta_{\text{IPC}}$**、**(b) バッファ長に対するコピー渡しと共有・ケイパビリティ受け渡しのスケーリング差**、および **(c) 同一カーネル内で実測するハードウェア隔離操作（`mov cr3` による TLB フラッシュや例外往復 `int`/`iretq`）の固定サイクル数** を測定し、単一アドレス空間・検証ベース隔離の構造的コストを定量化する。

## 影響・トレードオフ

**得るもの**:
- KVM ハードウェア仮想化と `lfence` / `rdtscp` により、バイナリ変換エミュレータや VM-Exit のノイズを排した 1 サイクル単位の実測が可能になる。
- [Theseus](https://www.usenix.org/system/files/osdi20-boos.pdf)、[RedLeaf](https://www.usenix.org/system/files/osdi20-narayanan.pdf)、[Mewz](https://arxiv.org/abs/2411.01129)、[Munal OS](https://github.com/Askannz/munal-os) 等の先行研究および Rust の `x86_64` OS 開発資料と同じアーキテクチャ上で設計・数値を対照できる。
- `x86_64-unknown-uefi`（Red Zone 無効・PE32+ 直リンク）と OVMF UEFI 直起動により、外部クレート・カスタムリンカスクリプト・ターゲット JSON 依存ゼロかつ最小限のインラインアセンブリ（`in`/`out`/`cli`/`lgdt`/`lidt`/`mov cr3`/`rdtscp` 等）で 64-bit カーネルを段階的に構築できる。

**受け入れるもの**:
- RISC-V の `satp = 0`（MMU 完全無効）とは異なり、64-bit Long Mode を維持するためだけに最小限の GDT（とセレクタ再ロード）および固定の恒等マッピングページテーブル（24 KiB）を管理する必要がある。
- `ExitBootServices()` 時のメモリマップ取得リトライ、`DescriptorSize` 走査、`EfiConventionalMemory` の選別、および IDT の分割オフセット構造や Port I/O（`0x3F8` / `0xF4`）など、`x86_64` + UEFI 固有の仕様を Step 1・Step 2・Step 4 で学ぶ必要がある。

## 関連するADR

- [ADR-000001: Modal One 憲法 — プロジェクト定義と基本原則](0001-constitution.md)（第3条4項・第7条2項を本 ADR により部分改正）
- [ADR-000002: クレート構成・実装順序・実行表現](0002-crate-layout-and-execution-representation.md)
- [ADR-000003: 検証体制とCI](0003-verification-and-ci.md)
