## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T08:14:11Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | crates/local-sights-core/src/cache/mod.rs > LogCache::read_overlaps（`EventSelection::All`） と crates/local-sights-core/src/query/runner.rs > read_cache | 重なる部分が 1 つでもあると、キャッシュのファイル全体のイベントをいったんメモリに読み込んでから範囲の外を捨てる（今の `lookup` は範囲だけを読む）。ファイルが大きいロググループでは、読んでいる間だけ一時的にメモリが増える。Step 13 のメモリの測り（保持 100 万件＋結果の写し 10 万件、ピーク 666 MiB）にはこの読み込みが入っていない。また、読み込みの間は [Cancel] を見ないため、巨大なファイルでは止まるまで待つことになる。計画の R-06 で「ファイル全体を読んで確かめる」と決めた結果であり、動作は設計どおりで、テストも通っている | 1 つのロググループのキャッシュが大きい場合のピークを手元で一度測り、`code-summary.md` に追記する。必要になれば、確かめだけ全体で行い、保持するのは重なる部分だけにする（今もそうなっている）か、読み込みの途中で停止の印を見る形に直す | New |
| R-02 | Minor | crates/local-sights-core/src/query/store.rs > QueryStore（BR1.6、モジュール冒頭の説明） | 保持は接続を変えたときとアプリを閉じたときにしか捨てず、手で捨てる手段もない。期間やロググループを替えながら何度も実行すると、committed のイベントが上限なく積み上がる。NFR2 の確かめは「100 万件」の 1 状態だけで、積み上がりの上限（件数やメモリ）に対するガードはない。設計で受け入れた方針（BR1.6、ADR-008）のためコードは設計どおりだが、使い続けたときの上限が利用者にも記録にも出ていない | `code-summary.md` の「残っていること」か README の制限事項に、積み上がる性質と、捨てる方法が接続の変更とアプリの再起動だけであることを書く。件数の目安を超えたら警告を出す案は、U4 以降の候補として残す | New |
| R-03 | Minor | crates/local-sights-core/src/insights_session.rs（`holdings_epoch` の項目）と crates/local-sights-core/src/session.rs > display_query_rows / display_query_row_detail | 結果の行の読み出しが runId の確かめだけになったため、`holdings_epoch` は返すだけで確かめには使っていない。API・Tauri のコマンド・画面の型に残っており、読む人には確かめに使われているように見える。コード上の説明（`only sent back with the rows`）はあるが、`code-summary.md` の「計画や設計との違い」にだけ書かれている | 次の作業単位で、`holdings_epoch` を使い道のある形に直すか、型と引数から外す。外さないなら、コマンドの説明にも「確かめには使わない」と書く | New |
| R-04 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u3-shared-store/code-generation/code-summary.md > 冒頭（「コミットと push はしていない」）と「メモリ」 | 冒頭に「コミットと push はしていない」とあるが、リポジトリには `7fdb731`（IB-11 U3：取得済みの範囲と取り足し…）がすでにあり、成果物もそのコミットに含まれている。また、メモリの測りは Linux の `VmHWM` だけで、macOS と Windows のピークは未測定（記録には書かれており、Walking Skeleton の手元確認と一緒に残っている） | 冒頭の文を実際のコミットに合わせて直す。macOS と Windows のメモリは、手元の確認の項目に「ピークを見る」と明記して持ち越す | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS（gaps・orphans・invalid すべて 0） | BR1.1〜BR5.3 と要件の ID の対応に抜けも余りもない |
| cargo test -p local-sights-core --lib query:: | PASS（121 件、無視 4 件） | FR2 の受け入れ基準 ac1〜ac6、認証情報の失敗で残りを始めない、一部失敗、中止、入れ子 100/101 段を含めて通る |
| cargo test -p local-sights-core --lib cache:: | PASS（46 件） | `read_overlaps` の 6 件と範囲の計算 8 件（R-07 の数値の例を含む）が通る。今の `lookup` と `write` は差分に変更がない |
| cargo test -p local-sights-core --lib insights_session:: | PASS（24 件） | 50 / 51 個、前の結果の保持、失敗で結果を消すことが通る。補助で流した `session::tests::query`（9 件）と `cargo test -p local-sights query`（8 件）、Vitest の 2 ファイル（36 件）も通る |
| cargo clippy --workspace --all-targets | 警告 0 | 型・借用・未使用の指摘はない。テスト以外のコードに `unwrap()` / `expect()` は見つからない |

### Summary

機能設計のレビューの指摘 R-01〜R-07 は、コードで確かめられた。R-01 は 1 範囲ごとの受け皿（`EventTimeline`）と `take_events` で、R-02 は and / or を並びで持つ繰り返しの読み取りと 100 段の上限で、R-03 は無視つきのメモリの測り（Linux のみ）で、R-04 は認証情報の失敗で残りを始めない動きと失敗したロググループの数で、R-05 は `StreamSequences` による番号の戻しで、R-06 は全体を確かめてから切り出す照会で、R-07 は実行開始の 5 分前の境目と UC3 の数値のテストで対応されている。呼ぶ AWS API は増えておらず、ログ管理の画面と今のキャッシュの照会・書き込みも変わっていない。残る指摘はどれも Minor で、設計どおりの振る舞いに対する記録と測りの補足である。
