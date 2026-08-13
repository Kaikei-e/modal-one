# Modal One

> An experimental WebAssembly kernel for RISC-V, written from scratch in Rust.
実験的なRISC-V用WebAssemblyカーネル、Rustで手書きされたものです。


[English](README.md)

### これは何か

Modal Oneは、WebAssemblyを唯一の実行環境とする最小のカーネルです。ハードウェアによるプロセス隔離の代わりに、Wasmの検証とサンドボックスに隔離を委ねます。ユーザランドのプログラムはWasmモジュールであり、それ以外のバイナリ形式は存在しません。

これは個人の長期学習プロジェクトです。手書きでゆっくり開発します。年単位の時間がかかる見込みで、その大半の期間は未完成のままです。

### 現状

**プレアルファ。まだ何も動きません。** 現在のリポジトリには設計ドキュメントのみが含まれます。

大まかなロードマップ（拘束力なし）:

| Phase | 目標 | 状態 |
|-------|------|------|
| 0 | `no_std`のWasmコアインタプリタ（デコーダ／バリデータ／実行器）をホスト上で動かす | 未着手 |
| 1 | ベアメタルRISC-V（QEMU `virt`）でのブート: UART・アロケータ・インタプリタ | 未着手 |
| 2 | WASI最小サブセットの手書き実装。`wasm32-wasip1`のHello, World!を実行 | 未着手 |
| 3 | モジュール間呼び出し（IPC）とサンドボックス境界コストの計測 | 未着手 |

### 設計上の制約

- **インタプリタのみ。** JITは行いません。
- **初日から`no_std`。** 実行パスは`core`と`alloc`のみに依存します。
- **ターゲット:** riscv64、QEMU `virt`マシン。物理ハードウェアはスコープ外です。
- **Wasm 1.0（MVP）サブセットから開始。** Wasm 2.0/3.0の機能は個別に採用し、ADRに記録します。
- **薄いシステムインターフェース層。** WASIはp1相当の小さな手書きサブセットから始め、コアから分離して、進行中のWASI / Component Model標準に追従できる構造を保ちます。
- **当面はシングルコア。** SMPは現在の目標ではありませんが、意図的に排除もしていません。

### 手書きと、その例外の定義

実行パス — バイナリデコーダ（LEB128を含む）、バリデータ、インタプリタ、カーネル、WASI実装 — は`wasmparser`等の実装クレートを使わず手書きします。

器具は別扱いです。テストハーネスは公式のWebAssemblyスペックテストスイート（およびそれを駆動するwastパーサ）を使い、ゲストプログラムはRust標準の`wasm32`ターゲットでビルドし、[wasmtime](https://github.com/bytecodealliance/wasmtime)は差分テストの基準器および参照実装として利用します。

AIツールは学習・設計の壁打ち・レビューに使い、実行パスのコード生成には使いません。

### 非目標

- プロダクション利用
- JITコンパイル
- 物理ハードウェアのサポート
- 既存のランタイム・OSとの競合

### 先行研究・参考文献

Modal Oneは[k23](https://github.com/JonasKruckenberg/k23)、[wasmtime](https://github.com/bytecodealliance/wasmtime)、[Theseus](https://github.com/theseus-os/Theseus)、[Mewz](https://github.com/mewz-project/mewz)、[WebAssembly仕様](https://webassembly.github.io/spec/)などの先行する仕事を参照しています。これらは参考文献であって競合ではありません。動くものが必要なら、それらを使ってください。

### ドキュメント

意思決定は`docs/adr/`配下にADRとして記録します（日本語）。ADR-000001がプロジェクトの原則を定義します。

### ライセンス

MITまたはApache-2.0のデュアルライセンスです。