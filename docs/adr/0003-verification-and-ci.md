---
title: 検証体制とCI
status: proposed
date: 2026-08-14
kind: adr
depends-on:
  - "0001"
  - "0002"
---

# ADR-000003: 検証体制とCI

- **更新日**: 2026-11-15（ロードマップレビューを受けて決定1〜4を改訂し、決定6〜8を追加）

## コンテキスト

憲法は「公式スペックテストスイートの合格」と「基準器との差分テストの一致」を正しさの根拠と定めた（第7条3項。基準器の扱いは [ADR-000005](0005-differential-oracle-selection.md) で改正）。これを実際に回すハーネスの構成、基準器との接続方法、CI、および静的な安全性保証（unsafe禁止・パニック抑止）を決める。

## 決定事項

### 1. spec-harnessの構成

- **テスト入力**: 公式のWebAssemblyスペックテストスイート（wast形式）を用いる。Wasm 1.0 の範囲は、`WebAssembly/spec` リポジトリの**タグ `wg-1.0`** の `test/core` に固定する（コミット `977f97014c962f7bd1291fcc6d28b41a924882bf`、`.wast` 73本。2026-11-15 に `git ls-remote` と取得物で確認）。取り込みはサブモジュールまたはベンダリングとし、コミット SHA を記録する。
- **パーサ**: wastスクリプトのパースにはBytecode Allianceの`wast`クレートを用いる（器具であり、憲法第5条の借用範囲）。版は `Cargo.lock` で固定する。
- **対応するディレクティブ**: 固定したタグに実在する集合を対象にする。`wg-1.0` の `test/core` に現れるのは、`module`（テキスト／`binary`／`quote`）、`register`、`invoke`、`assert_return`、`assert_trap`、`assert_malformed`、`assert_invalid`、`assert_unlinkable`、`assert_exhaustion`、`assert_return_canonical_nan`、`assert_return_arithmetic_nan` である。`assert_uninstantiable` は `wg-1.0` に無い。未対応ディレクティブはスキップとして明示的に集計する。
- **旧記法の NaN ディレクティブ**: `assert_return_canonical_nan` と `assert_return_arithmetic_nan` は `conversions.wast`／`f32.wast`／`f64.wast`／`float_exprs.wast`／`float_misc.wast` の5本で使われている。現行の書き方は `assert_return` と `nan:canonical`／`nan:arithmetic` の組であり、`wast` 263.0.0 のソースにこの2つのディレクティブは見当たらない。扱い（ハーネスの前処理で現行記法へ書き換える、対応する版のパーサを使う等）は、浮動小数点命令に着手する前に決めてこの ADR に追記する。
- **失敗の照合**: `assert_malformed` と `assert_invalid` は、失敗の分類（デコードエラーか検証エラーか）を照合する。メッセージ文字列の一致は求めない。

### 2. 合格リスト方式による回帰検知

Phase 0の間、スペックテストの大半は未実装機能により失敗する。CIを意味のあるゲートにするため、**通過済みwastファイルの明示的なリスト（合格リスト）**をリポジトリで管理し、CIは合格リストの全件通過を必須条件とする。合格リストに入れるのは、ファイル全体が通った `.wast` だけである。新しい命令範囲を実装するたびに、通過するようになったファイルを合格リストへ追加する。合格リストからの脱落（回帰）はCI失敗とする。

個別 ADR で 2.0 以降の機能を採用したときは、`WebAssembly/spec` の該当ファイルを個別に固定 SHA で取り込む。合格リストは「`wg-1.0` セット」と「追加セット」の二本立てにする。版によって期待結果が変わるテスト（例: 範囲外の data／elem セグメントは、1.0 では `assert_unlinkable`、bulk-memory 以降は実行時 trap）は、版の旗で切り替える。

### 3. 差分テスト — 基準器の割り当て

基準器は`spec-harness`の**dev-dependencyとしてライブラリ組み込み**する。同一モジュール・同一エントリポイント・同一引数での実行結果（戻り値またはトラップ種別）を、`modal-one-core`と基準器の間で比較する。割り当ては次のとおりとする（[ADR-000005](0005-differential-oracle-selection.md) 決定3）。

- **Wasm 1.0 の対応範囲**: 1.0 相当に機能を絞った基準器と比べる。第一候補は、機能ごとの個別トグルを持つ wasmi とする。wasmtime は、`Config::wasm_features` で 1.0 相当に絞れることを実測で確かめられた場合に、この範囲でも用いる。確かめた結果と採用した設定は、この ADR に追記する。
- **個別 ADR で採用した 2.0 以降の機能**: wasmtime を基準器とする。
- **受理・拒否だけの比較**: デコードと検証の結果だけを比べる補助として、wasmparser の検証器（`wasm-tools validate --features=mvp` に相当）を用いてよい。
- 基準器は実行パス（`modal-one-core`）の依存には一切現れない。
- 差分が出た場合、正はまず仕様書とスペックテストで判定する。基準器は仕様そのものではない。仕様とテストが Modal One 側を正とし、基準器が版の違いで食い違う行は、**既知差異リスト**に理由を添えて記録する。既知差異リストは合格リストとは別に管理する。
- 浮動小数点の結果は、NaN を分類して比べる（canonical NaN か、arithmetic NaN か、それ以外はビット列の一致）。値をビット列で持つ [ADR-000002](0002-crate-layout-and-execution-representation.md) 決定6がこの比較の前提になる。

### 4. CI

GitHub Actionsで、pushごとに以下を実行する。

1. `cargo fmt --check`
2. `cargo clippy`（warningをdenyに設定。決定6の lint を含む）
3. `modal-one-core`の単体テスト
4. spec-harnessによる合格リスト全件の実行
5. 合格リスト対象ファイルに対する差分テスト
6. `cargo build --target x86_64-unknown-uefi -p modal-one-core`

6は、同一ソースがベアメタル用ターゲットでもビルドできることの確認である。テストハーネスが `std` を引き込むのは test ビルドだけで、ライブラリ本体は `no_std` のままである。グローバルアロケータは `kernel` クレート側でだけ定義し、`modal-one-core` はアロケータ未定義のライブラリとしてビルドする。

Phase 1 以降に QEMU を使うテストを足すときは、実行時に `/dev/kvm` の有無を検出し、あれば KVM、なければ TCG で同じテストを回す（QEMU は `-accel kvm -accel tcg` のように複数指定すると先頭から順に試す）。CI は TCG だけでも成立するように組み、KVM は高速化の手段として扱う。

リポジトリ作成済みのため、CIは最初のコミット（ワークスペース骨格）と同時に有効化する。

### 5. unsafeの静的禁止

`modal-one-core`のクレートルートに `#![forbid(unsafe_code)]` を課す。インタプリタ・デコーダ・バリデータはunsafeなしで実装可能であり、「サンドボックスの土台が安全なRustのみで書かれている」ことをコンパイラが機械的に保証する状態を保つ。

- `forbid`であり`deny`ではない。属性による局所解除は不可能とする。
- 解除（`deny`への緩和を含む）は、計測に基づく明確な理由を伴う将来のADRによってのみ行う。
- `kernel`クレートは本条の対象外とする。ベアメタル層でのunsafeの扱いはPhase 1着手時のADRで定める。

### 6. パニックの静的抑止

[ADR-000002](0002-crate-layout-and-execution-representation.md) 決定6は、実行パスでのpanicをバグとして扱うと定めた。Ring 0 ではパニックがカーネル停止に直結する。これを自己申告にしないため、`modal-one-core` に次の clippy lint を課す。

- `clippy::indexing_slicing`／`clippy::unwrap_used`／`clippy::expect_used`／`clippy::panic`: `deny`
- `clippy::arithmetic_side_effects`: `warn` から始め、デコーダとバリデータで `deny` に上げる

### 7. ファジング

器具側に `cargo-fuzz` のターゲットを置き、wasm-smith が生成したモジュールと純粋なランダムバイト列を `decode` と `validate` に流す。確認するのは「任意の入力でパニックしない」ことである。`cargo-fuzz` は nightly を要するが、器具側なので憲法第5条の範囲に収まる。導入は、最小のバリデータを置いた後、命令範囲を広げる前とする。

### 8. 浮動小数点の検証

手書きの浮動小数点演算（[ADR-000004](0004-target-x86-64-kvm.md) 決定4）は、次の3層で検証する。

1. **スペックテスト**: `f32.wast`／`f64.wast`／`conversions.wast`／`float_exprs.wast`／`float_misc.wast` を合格基準にする。
2. **ホスト `std` との差分**: f32 の単項演算は 2^32 通りの全数比較を行う。
3. **Berkeley TestFloat のベクタ**。

## 影響・トレードオフ

**得るもの**: 合格リスト方式により、CIが最初の1件から意味を持ち、回帰を機械的に検知できる。差分テストのライブラリ組み込みにより、結果を型付きで比較でき、トラップ種別まで突き合わせられる。1.0 の範囲を 1.0 相当に絞った基準器と比べることで、差分の原因を「版の違い」と「実装の誤り」に分けて読める。forbid(unsafe_code)と決定6の lint により、Phase 0の安全性主張が自己申告ではなくコンパイラと lint の保証になる。

**受け入れるもの**: 基準器がdev-dependencyに入るため、spec-harnessのビルドは重い（実行パスのビルドには影響しない）。合格リストと既知差異リストの手動管理には更新の手間が伴う。forbidにより、値スタック等で速度目的のunsafe技巧は使えないが、Phase 0の目的（正しさと学習）に照らして制約とみなさない。決定6の lint は添字アクセスと `unwrap` を書けなくするので、`get` と `?` による明示的なエラー伝播が増える。

## 関連するADR

- ADR-000001: 憲法（第5条・第7条）
- ADR-000002: クレート構成・実装順序・実行表現
- ADR-000004: 基準ターゲットのx86_64（QEMU/KVM）への変更とベアメタル起動方針
- ADR-000005: 差分テスト基準器の選定を通常ADRへ委ねる憲法改正
