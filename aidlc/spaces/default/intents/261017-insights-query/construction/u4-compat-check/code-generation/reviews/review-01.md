## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T09:43:55Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | scripts/compat/insights-export.sh > start-query が失敗したときの分岐（`Error` を書く部分）、crates/local-sights-core/examples/compat/compare.rs > compare_query | start-query が失敗した理由を区別せず、すべて `{"status":"Error"}` として書いている。認証切れ・アクセス拒否・スロットリング・ロググループなし・ネットワーク断でも「Logs Insights がクエリを断った」と同じ扱いになり、compat_check はこれを BR2.4 の断り方の比べに使う。アプリが読めない p05 では BothRejected（互換とみなす）に、それ以外のクエリでは OnlyInsightsRejected（違い）になる。R-02 が避けたかった「動かせなかったのに比べた」ことが、この経路では起きる。 | 断りとして扱うのは、CLI のエラーが「クエリの文法の誤り」（MalformedQueryException など）を示すときだけにする。それ以外の失敗は status `Unknown` などにして書き出し、NotCompared（終了コード 2）に回す。エラー種別を確認する分岐に対するテストを偽の `aws` で足す（認証切れ相当の失敗で `Error` にならないこと）。 | New |
| R-02 | Minor | scripts/compat/insights-export.sh > OUT の扱い（`mkdir -p` のあと、`settings.json` だけを上書き） | OUT を使い回すと、前回の実行で書いた `<id>.json` が残る。スクリプトが途中で止まった（Ctrl-C や設定の誤り）場合、今回の実行で書かれなかったクエリには前回の期間・ロググループの結果が残り、compat_check は Missing（NotCompared）ではなく古い結果を新しい取得と比べて Same／Different を出す。 | 実行の最初に OUT の中の `*.json` を消すか、各 `<id>.json` に実行の識別子（`exportedAtSeconds` など）を入れて compat_check が settings.json と食い違うファイルを NotCompared にする。 | New |
| R-03 | Minor | scripts/compat/insights-export.sh > `aws logs start-query` の引数 | `--limit` を渡していない。`limit` を書かないクエリ（q03〜q08、q11、p01〜p07、p09）の行数の上限は StartQuery の既定値に任せており、compat_check 側は 10,000 を前提にしている（`row_cap`）。AWS の既定値が 10,000 より小さい場合、10,000 件未満でも Logs Insights だけが途中で切れ、`Different` の誤報になる。既定値は確かめていない。 | `--limit 10000` を明示して前提を固定する（上限は 10,000）。そのうえで、スクリプトの説明か code-summary に前提を 1 行書く。 | New |
| R-04 | Minor | scripts/compat/insights-export.sh > `query_id=$(aws_logs start-query ... 2>&1)` | 成功時にも標準エラーを標準出力と混ぜて `query_id` に取り込んでいる。CLI が警告（古い版の注意など）を標準エラーに出すと、`query_id` が壊れ、get-query-results が失敗して Unknown（NotCompared）になる。壊れ方としては安全側だが、原因が分かりにくい。 | 標準エラーを一時ファイルか別の変数に分け、失敗時だけ断りの文として使う。（Major の対応と同時に直せる。） | New |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/code-generation/code-summary.md > 「確かめたこと」の `cargo fmt --check` の行と「残っていること」 | 要約は `crates/local-sights-core/src/query/store.rs` の書式の違いが残っていて U4 では直していない、と書いている。しかし今回の作業ではこのファイルに rustfmt をかけており、`cargo fmt --check` はいま通る。要約と実際が合わず、source-manifest.json の `writes` にも store.rs がない。 | code-summary の該当の 2 か所を「store.rs を rustfmt で整えた」に直し、source-manifest.json の `writes` に store.rs を足す（または、整形だけの変更だと明記する）。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/code-generation/unit-test-instructions.md、code-summary.md > 「計画や設計との違い」の最後の項目 | スクリプトのテスト（偽の `aws`）は `#[cfg(unix)]` かつ例のテストなので、CI では Linux のランナーの `cargo test --examples` だけで流れる。macOS の bash 3.2 での動作（`set -u` と空の配列、`${a[@]+...}` の書き方など）は自動では確かめられず、手元の初回実行に頼る。また、テストは実際の `jq` と `git` が PATH にあることを前提にしている。 | 初回の手元の実行を macOS の bash 3.2 で行うことを code-summary の手順に明記する。`jq` がなければテストを飛ばす（または失敗の理由を分かりやすくする）扱いを検討する。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS（gaps・orphans・invalid なし、findings 0） | BR1.1〜BR3.3 と要件の対応は欠けも余りもない。 |
| cargo test -p local-sights-core --example compat_check | PASS（42 件、AWS に接続しない） | 偽のゲートウェイと偽の `aws` で流れている。unwrap は `#[cfg(test)]` の中だけ。 |
| bash -n scripts/compat/insights-export.sh | PASS | 構文は正しい。bash 3.2 向けの書き方（空の配列の展開、`local`、`case` の判定）を目で確認した。 |
| cargo clippy --workspace --all-targets | 警告・エラーなし | — |
| cargo fmt --check | PASS | — |

**設計レビューの指摘 R-01〜R-06 の反映の確認**

- R-01: `next_row_after_cut` が最後の `limit` を 1 つ大きくして切り目のすぐ後ろの行を求め、`tie_at_cut` と `compare_rows` が同順位の行を比べから除いて件数だけ比べる。sort のない上限切れは Skipped。反映されている。
- R-02: Complete 以外と書き出しなしは NotCompared、NotCompared があるか比べた本数が 0 なら終了コード 2。反映されている（ただし start-query の失敗の区別は上の Major を参照）。
- R-03: 期間を START の 0 ms から END の 999 ms にし、境目の秒にイベントがあるクエリに印を付ける。反映されている。
- R-04: p09 の目的を書き直し、JSON の数が確かめられないことを code-summary に書き、`--queries` と `QUERIES=` で別の組を使える。反映されている。
- R-05: スクリプトは `git rev-parse` と `pwd -P`、確認用プログラムは `.git` を探して `canonicalize` し、大文字・小文字を区別しない OS では無視して比べる。反映されている。
- R-06: 列のない行を両側で同じ扱いにしている（format.rs、compare.rs のテストあり）。反映されている。

**プロジェクトの制約の確認**

- 呼ぶ API は DescribeLogStreams と GetLogEvents だけ（ゲートウェイの `describe_log_groups` はテスト用のゲートウェイで panic、本体は呼ばない）。Logs Insights を呼ぶのは手で流すスクリプトだけ。
- 書き出しの置き場所は、スクリプトと確認用プログラムの両方でリポジトリの外を確かめている。`.gitignore` の追加は念のための守り。
- 秘密の認証情報・アクセスキー ID は、コード・報告・テストにない。断りの文は Request ID を除き、報告には先頭 40 文字までを出す。
- テスト以外のコードに `unwrap()`・`expect()`・`panic!` はない（検索で確認）。
- テストは実際の AWS に接続しない。

### Summary

設計レビューの 6 件はコードで確かに扱われ、ツールもすべて通る。気になる点は、start-query の失敗がすべて「クエリの断り」として比べに使われ、認証切れなどの失敗が BothRejected（互換）や OnlyInsightsRejected として報告されうることで、手元の比べの結果の信頼性に関わる。残りは軽い指摘で、助言の位置づけのため READY とした。
