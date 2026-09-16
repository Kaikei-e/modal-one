#!/usr/bin/env bash
# PreToolUse guard for Edit/Write on execution-path source files.
# ADR-0001 第6条: 実行パスの実装コードは作者が自分の手で書く。
# Forces a confirmation prompt even when auto-approve settings would
# otherwise let Edit/Write through silently.
set -euo pipefail

file=$(jq -r '.tool_input.file_path // empty')

case "$file" in
  */modal-one-core/src/*|*/kernel/src/*)
    reason='ADR-0001 第6条: 実行パス (modal-one-core / kernel) の実装コードは作者が自分の手で書く方針。ユーザーから明示的に依頼された変更か確認してください。'
    printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"%s"}}' "$reason"
    ;;
esac
