# Security Requirements — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

U4 は時刻と長さだけを扱い、AWS の呼び出し・認証・保存先に触れない。

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR8.10 | SessionView の `elapsed` に入るのは真偽値（running）と数値（elapsedMs）だけとし、秘密の認証情報・アクセスキー ID・プロファイル名・ロググループ名などを入れない | 型の定義と、SessionView を JSON にしたときの `elapsed` の形を確かめるライブラリのテスト | project.md Forbidden、NFR8、C5 |
| NFR8.11 | 新しい API の呼び出し、新しい依存、新しいログ・ファイルへの書き込み、外への送信を足さない | コードの差分の確認。CI の cargo-deny と `npm audit --audit-level=high` が今のまま通る | project.md Forbidden、team.md Code Style |
| NFR6.2 | 自動テストと CI は実際の AWS に接続しない。経過のテストは、ライブラリも画面も時計を差し替えて行う | 新しいテストは時計の差し替えと既存の偽物だけを使う | project.md Forbidden、NFR6 |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 画面へ渡す項目に接続の情報が混ざる | `elapsed` の形を真偽値と数値に限る（NFR8.10） |
| 経過の記録のために新しいログやファイルを書く | 記録は AppSession のメモリの中だけ。ログ・キャッシュに書かない（NFR8.11） |

## Assumptions & Open Questions

None.
