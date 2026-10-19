## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-18T13:20:00Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | scripts/compat/insights-export.sh > start-query が失敗したときの分岐（`Error` を書く部分）、crates/local-sights-core/examples/compat/compare.rs > compare_query | start-query が失敗した理由を区別せず、すべて `{"status":"Error"}` として書いている。認証切れ・アクセス拒否・スロットリング・ロググループなし・ネットワーク断でも「Logs Insights がクエリを断った」と同じ扱いになり、compat_check はこれを BR2.4 の断り方の比べに使う。アプリが読めない p05 では BothRejected（互換とみなす）に、それ以外のクエリでは OnlyInsightsRejected（違い）になる。R-02 が避けたかった「動かせなかったのに比べた」ことが、この経路では起きる。 | 断りとして扱うのは、CLI のエラーが「クエリの文法の誤り」（MalformedQueryException など）を示すときだけにする。それ以外の失敗は status `Unknown` などにして書き出し、NotCompared（終了コード 2）に回す。エラー種別を確認する分岐に対するテストを偽の `aws` で足す（認証切れ相当の失敗で `Error` にならないこと）。 | Resolved (reviewer) |
| R-02 | Minor | scripts/compat/insights-export.sh > OUT の扱い（`mkdir -p` のあと、`settings.json` だけを上書き） | OUT を使い回すと、前回の実行で書いた `<id>.json` が残る。スクリプトが途中で止まった（Ctrl-C や設定の誤り）場合、今回の実行で書かれなかったクエリには前回の期間・ロググループの結果が残り、compat_check は Missing（NotCompared）ではなく古い結果を新しい取得と比べて Same／Different を出す。 | 実行の最初に OUT の中の `*.json` を消すか、各 `<id>.json` に実行の識別子（`exportedAtSeconds` など）を入れて compat_check が settings.json と食い違うファイルを NotCompared にする。 | Resolved (reviewer) |
| R-03 | Minor | scripts/compat/insights-export.sh > `aws logs start-query` の引数 | `--limit` を渡していない。`limit` を書かないクエリ（q03〜q08、q11、p01〜p07、p09）の行数の上限は StartQuery の既定値に任せており、compat_check 側は 10,000 を前提にしている（`row_cap`）。AWS の既定値が 10,000 より小さい場合、10,000 件未満でも Logs Insights だけが途中で切れ、`Different` の誤報になる。既定値は確かめていない。 | `--limit 10000` を明示して前提を固定する（上限は 10,000）。そのうえで、スクリプトの説明か code-summary に前提を 1 行書く。 | Resolved (reviewer) |
| R-04 | Minor | scripts/compat/insights-export.sh > `query_id=$(aws_logs start-query ... 2>&1)` | 成功時にも標準エラーを標準出力と混ぜて `query_id` に取り込んでいる。CLI が警告（古い版の注意など）を標準エラーに出すと、`query_id` が壊れ、get-query-results が失敗して Unknown（NotCompared）になる。壊れ方としては安全側だが、原因が分かりにくい。 | 標準エラーを一時ファイルか別の変数に分け、失敗時だけ断りの文として使う。（Major の対応と同時に直せる。） | Resolved (reviewer) |
| R-05 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/code-generation/code-summary.md > 「確かめたこと」の `cargo fmt --check` の行と「残っていること」 | 要約は `crates/local-sights-core/src/query/store.rs` の書式の違いが残っていて U4 では直していない、と書いている。しかし今回の作業ではこのファイルに rustfmt をかけており、`cargo fmt --check` はいま通る。要約と実際が合わず、source-manifest.json の `writes` にも store.rs がない。 | code-summary の該当の 2 か所を「store.rs を rustfmt で整えた」に直し、source-manifest.json の `writes` に store.rs を足す（または、整形だけの変更だと明記する）。 | Resolved (reviewer) |
| R-06 | Minor | aidlc/spaces/default/intents/261017-insights-query/construction/u4-compat-check/code-generation/unit-test-instructions.md、code-summary.md > 「計画や設計との違い」の最後の項目 | スクリプトのテスト（偽の `aws`）は `#[cfg(unix)]` かつ例のテストなので、CI では Linux のランナーの `cargo test --examples` だけで流れる。macOS の bash 3.2 での動作（`set -u` と空の配列、`${a[@]+...}` の書き方など）は自動では確かめられず、手元の初回実行に頼る。また、テストは実際の `jq` と `git` が PATH にあることを前提にしている。 | 初回の手元の実行を macOS の bash 3.2 で行うことを code-summary の手順に明記する。`jq` がなければテストを飛ばす（または失敗の理由を分かりやすくする）扱いを検討する。 | Accepted risk |
| R-07 | Minor | scripts/compat/insights-export.sh > start-query 失敗時の `grep -q 'MalformedQueryException' "$ERR_FILE"` | 断りかどうかを、標準エラー全体に `MalformedQueryException` という文字列があるかで決めている。CLI の警告や別のエラーの本文にこの語が偶然含まれると `Error`（断り）と見なされ、BR2.4 の比べに使われうる。実際に起きる見込みは低く、アプリが読めないクエリでの誤判定に限られる。 | 余裕があれば `An error occurred (MalformedQueryException) when calling the StartQuery` のように、エラーの行の形で照合する。直さない場合は、手元の初回の実行で `Error` と出たクエリの message を目で確かめる手順を code-summary に 1 行足す。 | New |
| R-08 | Minor | crates/local-sights-core/examples/compat/settings.rs、scripts/compat/insights-export.sh > `exportedAtSeconds` の付け方 | 実行の識別子は秒単位の開始時刻（`date +%s`）である。同じ OUT で、同じ秒に 2 回実行を始める場合は区別できないが、手で流す道具であり、同じ秒に重ねて流す使い方は現実にはない。参考として記録する。 | 対応は不要。 | New |

> R-01 Reviewer note: insights-export.sh は start-query の標準エラーを ERR_FILE に取り、`MalformedQueryException` を含むときだけ `Error`（refused に数える）で書く。それ以外の失敗は `Unknown`（other に数え、終了コード 1）で書く。compat_check 側では `Unknown` は NotRun になり、NotCompared（終了コード 2）に回る。偽の `aws` による認証切れ（ExpiredTokenException）のテストが通っており、Request ID が報告に出ないことも確かめている。

> R-01 Earlier decision: Accepted risk

> R-02 Reviewer note: settings.json と全 `<id>.json` に同じ `exportedAtSeconds`（NOW）を入れ、`parse_export_of_run` が一致しないファイル、印のないファイル、settings.json に印がない場合を `OtherRun`（NotCompared）にする。前回の実行が途中で止まっても、古い結果は比べに入らない。

> R-02 Earlier decision: Accepted risk

> R-03 Reviewer note: start-query に `--limit 10000` を渡している。偽の `aws` のテストが、すべての start-query の呼び出しに含まれることを確かめている。説明にも前提を書いてある。

> R-03 Earlier decision: Accepted risk

> R-04 Reviewer note: start-query の標準エラーを `2>"$ERR_FILE"` で分け、失敗時だけ断りの文として読む。警告があってもクエリ ID が壊れないことをテストで確かめている。一時ファイルは `trap ... EXIT` で消す。

> R-04 Earlier decision: Accepted risk

> R-05 Reviewer note: code-summary.md の 2 か所は「store.rs に rustfmt をかけた（書式だけ）」に直り、source-manifest.json にも store.rs がある。`cargo fmt --all --check` は通る。

> R-05 Earlier decision: Accepted risk

> R-06 Reviewer note: 意図どおり開発者の手元の実行に残してある。偽の `aws` のテストは `#[cfg(unix)]` で Linux のランナーでしか流れないため、macOS の bash 3.2 での動作（空の配列の展開、`mktemp` のひな形など）はまだ自動では確かめられていない。初回の手元の実行を macOS で行うことが確認の代わりになる。

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| aidlc engine sensor-traceability | PASS（gaps・orphans・invalid なし、findings 0） | 要件との対応に欠けも余りもない。 |
| cargo test -p local-sights-core --example compat_check | PASS（48 件） | 前回の 42 件から、start-query 失敗、他の実行のファイル、`--limit`、警告の混入のテストが増えた。偽の `aws` と偽のゲートウェイだけで、実際の AWS には接続しない。 |
| bash -n scripts/compat/insights-export.sh | PASS | 構文は正しい。 |
| cargo fmt --all --check | PASS | — |
| cargo clippy --workspace --all-targets | 警告・エラーなし | — |

**プロジェクトの制約の確認**

- 呼ぶ API は DescribeLogStreams と GetLogEvents だけ。Logs Insights を呼ぶのは手で流すスクリプトだけ。今回の差分でこの点は変わっていない。
- 書き出しの置き場所はリポジトリの外の検査が残っている。失敗時の message にも Request ID を除いたものだけを入れ、秘密の認証情報やアクセスキー ID は出ない。
- 今回の差分のテスト以外のコードに `unwrap()` / `expect()` はない（`unwrap` はテストの中だけ）。
- bash 3.2 向けの書き方（配列の `+` 展開、`local`、`mktemp` のひな形が末尾の X、`trap ... EXIT`）を目で確かめた。実機の確認は R-06 のとおり手元に残る。

### Summary

R-01 から R-05 はコードとテストの両方で確かに直っており、R-01 の誤判定の経路（認証切れが断りとして比べに使われること）は塞がっている。ツールもすべて通る。残るのは、macOS の bash 3.2 での手元の初回実行（R-06）と、断りの判定を文字列の一致に頼る軽い指摘だけで、承認を止める理由にはならない。
