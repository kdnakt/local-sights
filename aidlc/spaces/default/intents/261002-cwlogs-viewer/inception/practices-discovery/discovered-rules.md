# Discovered Rules（ドラフト）

> **ステータス: ドラフト（人間へのインタビュー前）**
> ここに載せるのは、人間が明言したハードな制約だけ。グリーンフィールドのため現時点では該当なし。下の「確認候補」はルールではなく、インタビューで尋ねる論点のメモである。

## Mandated

None.

## Forbidden

None.

<!--
確認候補（ルールではない。インタビューで人間が明言した場合のみ ALWAYS / NEVER の形で上に昇格させる）:
- 候補: AWS の認証情報・アクセスキーをリポジトリやログ、キャッシュファイルに書き込まない（OSS 公開・ログに機密が含まれうる：decision-log D-07）
- 候補: 取得したログはディスクに保存しない（利用者がキャッシュを有効にした場合を除く：decision-log D-07）
- 候補: ログを変更・削除する AWS API を呼ばない（対象外「ログの変更操作」：decision-log D-13）
- 候補: FilterLogEvents API と Logs Insights API を使わない（対象外：decision-log D-13）
- 候補: マージ前に `cargo fmt --check` と `cargo clippy` を通す
- 候補: `main` に直接 push しない（PR 経由に限る）か、個人開発として直接 push を許容するか
-->
