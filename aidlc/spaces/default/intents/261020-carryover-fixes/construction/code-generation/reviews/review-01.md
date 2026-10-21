## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-21T00:49:53Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | src/components/TopBar.tsx > region Select（`onLoadItems` と `regionFilter` の状態） | 絞り込みの文字列 `regionFilter` は `onChange`（選んだとき）でしか空に戻らない。文字を打ったあと何も選ばずに閉じて開き直したとき、Cloudscape が空の文字列で `onLoadItems` を呼び直すかどうかはコードからもテストからも確かめられていない。呼ばれなければ、入力欄は空なのに候補だけが前の絞り込みのまま残るおそれがある。TopBar.test.tsx に「打つ→閉じる→開き直す」を確かめるテストがない。 | 「文字を打つ→Escape で閉じる→開き直す」のテストを足して全候補が戻ることを確かめる。戻らなければ、閉じたとき（`onBlur` や `onChange` 以外）にも `setRegionFilter("")` を呼ぶ。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/construction/code-generation/code-summary.md > 計画との違い 1 | 要件 NFR1 は「FR2.2.5 と FR4.3 で直すテスト以外は期待値を変えない」と書くが、実際には FR4.1（StatusLine.test.tsx、LogAnalyticsPage.test.tsx）、FR4.2（App.test.tsx）、FR2.3（lib.rs、QueryResultTable.test.tsx）の既存のテストも直している。どれも、この作業で直す振る舞いそのものを固定していたものなので中身は妥当だが、要件の文面とは食い違う。 | 要件の NFR1 の例外の書き方を実態に合わせて直すか、Build and Test の記録に、この作業で直す振る舞いを固定していたテストも直したことを書いて追跡できるようにする。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261020-carryover-fixes/construction/code-generation/code-summary.md > 残っていること | ライブラリの行カバレッジ（`cargo-llvm-cov`、80%）を手元で測れておらず、CI の Linux のランナーに任せている。新しい関数（`last_overlapping_range`、`verify_through`、`EventSelection::Through` の分岐）は専用のテストが通っており、リスクは小さい。 | CI の結果で 80% 以上を確かめる。確かめられなければ Build and Test で記録する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -q -p local-sights-core --lib cache::` | 57 件通過 | `read_overlaps` の読み止め、複数の範囲にまたがる結果が全部読んだときと同じこと、読まない部分の壊れに気づかないこと、読んだ部分の壊れで Corrupt になることのテストが通る。 |
| `npx vitest run src/components/TopBar.test.tsx src/i18n` | 59 件通過 | リージョンの選択欄（表示・絞り込み・無効・キーボード）、英語の単数・複数、`STANDARD_REGIONS` との一致のテストが通る。 |
| `grep holdings_epoch / holdingsEpoch`（crates、src、src-tauri） | `src-tauri/src/lib.rs` の「送られないこと」を確かめる否定の確認 1 件だけ | FR2.3 の取り除きは完了している。 |
| 差分の追加行の `unwrap()` / `expect()` | すべてテストの中（`mod tests` と `tests/u5_cancel.rs`） | 製品のコードには足していない。 |
| 呼ぶ API | 追加行に AWS の SDK 呼び出しやネットワークからの取得はない | 読み取りの 3 つ以外を呼ばない決まりを守っている。名前の表は組み込み。 |

### Summary

FR1〜FR5 と NFR1〜NFR7 は計画どおり実装されていて、重大な欠陥は見つからなかった。`read_overlaps` は重なる一番後ろの範囲の終わりで止まり、そこまでのすべての範囲の件数を見出しと比べる（`verify_through`）。`begin_run` はランナーの最初の 1 回の呼び出しで選ばれていないものだけを捨てる。複数形は `translate` の 1 か所で選び、`STANDARD_REGIONS` の取り出しは取れないとき・0 件のときに例外で落ちる。計画との違いはどれも妥当だった。指摘は、リージョンの選択欄で何も選ばずに閉じたあとの絞り込みの文字の残りを確かめるテストがないことと、NFR1 の文面と実際の食い違い、カバレッジ未計測の 3 件で、いずれもブロックしない Minor である。
