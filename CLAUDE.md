# modal-one — CLAUDE.md (project-scope)

Modal One は学習目的の個人プロジェクトである（[ADR-0001 第2条・第6条](docs/adr/0001-constitution.md)）。全体方針は `~/.claude/CLAUDE.md`（user-scope）を継承し、ここには本プロジェクト固有の制約のみを置く。

## 実装コードの扱い

**実行パス（`modal-one-core/src/`・`kernel/src/` 配下、および ADR-0002 が今後「実行パス」と定めるクレート）の実装コードは、ユーザーから明示的に依頼された場合を除いて書き換えない。**

根拠は ADR-0001 第6条: 「原則として、実行パスの実装コードは作者が自分の手で書く」。目的は第2条の不変条件（作者が全行を理解している状態を保つ）を守ることであり、この制約は AI 利用の裁量（第6条）に対する author 自身の選択である。

`spec-harness`／`kernel`（ブート等の器具コードを除く）／`xtask` は [ADR-0002 決定1・決定5](docs/adr/0002-crate-layout-and-execution-representation.md) の「器具」区分であり、この制約の対象外。

### 自由に行ってよいこと（第6条で明示的に許可されている範囲）

- 仕様・概念の解説
- 設計の壁打ち・レビュー（ユーザーが書いたコードへのコメント。直接編集はしない）
- デバッグの支援（原因の指摘・修正案の提示。修正コードを直接適用はしない）
- 器具側コード（`spec-harness`／`xtask`／CI設定）の生成
- ドキュメント（ADR・README等）の共同執筆

### 「明示的な依頼」の目安

同一メッセージ内で「実装して」「書いて」「直して」など、コード変更そのものを求める発話があった場合のみ実装してよい。設計相談・レビュー依頼・エラーの説明要求は実装許可に含まない。迷ったら先に確認する。

### 技術的な補強

`.claude/settings.json` の PreToolUse hook（`.claude/hooks/guard-execution-path.sh`）が、`modal-one-core/src/`・`kernel/src/` 配下への Edit/Write に対して、auto-approve 設定の有無に関わらず確認プロンプトを強制する。これは方針判断を代替するものではなく、判断が誤った場合の技術的なバックストップ。
