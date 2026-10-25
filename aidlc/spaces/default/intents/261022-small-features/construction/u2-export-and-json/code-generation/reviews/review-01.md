## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-24T16:41:24Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src-tauri/src/lib.rs > `start_export`（ログの画面の分岐）と crates/local-sights-core/src/session.rs > `begin_export` の `row_count` | 通知に出す行数は、錠の中で session の件数（`shown_log_row_count`）から決めて `ExportJob` に入れている。実際に書き出す行は、その後 LogView から `rows(0, total)` で複製したもの（`begun.source.row_count()` は `with_events` の前は 0）。両方を同じ session 錠の下で読むので今は一致するが、複製した行数との一致を確かめる箇所がない。 | `with_events` で足したあとに、複製した行数と `begin_export` に記録した行数が等しいことを確かめる（食い違うときは複製した行数を使う）か、食い違いが起きない前提を `start_export` のコメントに書く。 | New |
| R-02 | Minor | src/jsonPretty.ts > `findJson` と定数 `MAX_CANDIDATES` / `MAX_MESSAGE_LENGTH` | 候補は最大 64 個で、1 つの走査は最大 1 MiB まで読んでから失敗しうる。`{"a":` のような開き括弧が多く、終わりで壊れている 1 MiB のメッセージでは、行を開いた描画の同期処理で最悪およそ 64 MiB 分の走査になる。上限はあるが、画面が止まる時間を測った証拠はない。 | 走査した文字数の合計にも上限を置く（例：候補をまたいだ合計 4 MiB）か、最悪の入力で `findJson` が許容できる時間で終わることをテストで確かめる。 | New |
| R-03 | Minor | crates/local-sights-core/src/export/formatter.rs > `markdown_cell`、`csv_cell` | 値は括りと逃がし以外に加工しない（AS3）。`=SUM(1)` のような先頭の式はそのまま出る（Excel で開くと式として評価される）。Markdown は `` を逃がさないため、末尾が `` のセルの直後の区切りが `\|` と読まれうる。どちらも設計どおりだが、利用者向けの文書には書かれていない。 | Build and Test で `docs/` に「値は加工せずに出す」旨と、CSV を表計算ソフトで開くときの注意を一言書く。末尾 `` の扱いは受け入れるか、`` も逃がすかを決める。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261022-small-features/construction/u2-export-and-json/code-generation/code-summary.md > テスト結果の表、「残っていること」 | macOS と Windows の目視確認、`cargo deny check` の未実行、`npm audit` の high 2 件（undici、zod。再現を確かめた。この作業単位の変更ではない）が未了。保存ダイアログとクリップボードは公式プラグインを Rust 側から呼ぶ薄い境界で、自動テストの外にある。要約の記載は実際の結果と一致していて、隠しごとはない。 | Build and Test の前に、`cargo deny check` の実行と `npm audit` の high の扱い、両 OS での 4 通りの書き出しと取り消し・失敗の目視を実施して記録する。1 回目に timeout した既存テストが CI で再発したら timeout を見直す。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -p local-sights-core export` | PASS（30 本。formatter 9、exporter 8、session・insights_session 13 の合計と一致） | 要約の本数と一致する。 |
| `cargo test -p local-sights export` | PASS（5 本、`export_boundary`） | 一時ファイル → rename、失敗時の一時ファイルの除去、元のファイルの保持を一時フォルダの実ファイルで確かめている。 |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし | 問題なし。 |
| `cargo fmt --all -- --check` | 差分なし | 問題なし。 |
| `npx vitest run`（指定の 10 ファイル） | PASS（196 本、0 失敗） | 問題なし。全体の 549 本と 90.48% の行カバレッジは要約の値で、再計測していない（80% の下限を大きく超える）。 |
| `npx tsc --noEmit`、`npx eslint .` | エラーなし | 問題なし。 |
| `npx prettier --check .` | 通過 | 問題なし。 |
| `npm audit --audit-level=high` | high 2 件（undici、zod） | 要約の記載どおり。`package.json` はこの作業単位で触っておらず、既存の依存による。 |
| 本番コードの `unwrap()` / `expect()` と `cfg` の分岐 | なし（exporter.rs、formatter.rs、export_boundary.rs、lib.rs の追加分。`unwrap` は `#[cfg(test)]` の中のみ） | チームの規則に合う。 |

### Summary

設計の R-01〜R-07 は実装に反映されている。錠の中は可否の確認と行の複製だけで、文字への変換と書き込みは錠を離した blocking スレッドで行う。ロックの順序は session → LogView で既存の呼び出し元と同じ。失敗は種類の列挙と path で返り、画面が鍵から文を組み立てる。WebView にはプラグインの権限を与えない。`rows` / `window` に件数の上限はなく、行が黙って切れることはない。JSON の整形は字句を保ち、再帰を使わず、上限もある。Critical と Major はなく、Minor は 4 件（行数の一致の確認、走査量の合計の上限、値を加工しないことの周知、実機・CI の確認の未了）で、READY とする。
