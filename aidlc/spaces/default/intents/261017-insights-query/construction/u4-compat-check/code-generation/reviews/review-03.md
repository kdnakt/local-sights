## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-19T02:02:28Z
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
| R-07 | Minor | scripts/compat/insights-export.sh > start-query 失敗時の `grep -q 'MalformedQueryException' "$ERR_FILE"` | 断りかどうかを、標準エラー全体に `MalformedQueryException` という文字列があるかで決めている。CLI の警告や別のエラーの本文にこの語が偶然含まれると `Error`（断り）と見なされ、BR2.4 の比べに使われうる。実際に起きる見込みは低く、アプリが読めないクエリでの誤判定に限られる。 | 余裕があれば `An error occurred (MalformedQueryException) when calling the StartQuery` のように、エラーの行の形で照合する。直さない場合は、手元の初回の実行で `Error` と出たクエリの message を目で確かめる手順を code-summary に 1 行足す。 | Accepted risk |
| R-08 | Minor | crates/local-sights-core/examples/compat/settings.rs、scripts/compat/insights-export.sh > `exportedAtSeconds` の付け方 | 実行の識別子は秒単位の開始時刻（`date +%s`）である。同じ OUT で、同じ秒に 2 回実行を始める場合は区別できないが、手で流す道具であり、同じ秒に重ねて流す使い方は現実にはない。参考として記録する。 | 対応は不要。 | Accepted risk |
| R-09 | Minor | scripts/compat/insights-export.sh > has_limit_command (正規表現のマスク規則) | 正規表現として隠すのは `like`、`=~`、`(`、`,` の直後の `/` だけである。`parse @message /(a\|limit 3)/ as x` のように `parse` の正規表現の中に `\| limit` があると、`limit` コマンドと誤って判定され（実際に確かめて true になった）、そのクエリに `--limit 10000` が付かない。同梱のクエリの組（q01〜p12）には `parse` がなく今は起きないが、`QUERIES=` で別の組を渡す使い方は説明に書かれている。 | `parse <項目>` の直後の `/` も正規表現として隠す規則を足すか、`QUERIES=` の組に `parse` の正規表現を入れないことをスクリプトの説明に 1 行書く。 | New |
| R-10 | Minor | crates/local-sights-core/examples/compat_check.rs > the_script_leaves_a_query_with_its_own_limit_to_that_limit | 判定のテストは 7 件で、`limit` ありの側は「先頭」「`\| limit` の後ろ」「大文字」の 3 件だけである。`#` の注記・バッククォートの名前・引用符の中のエスケープ・`\|limit`（空白なし）・`not like /…limit…/` といった、マスク処理が実際に分岐する箇所のテストがない（手で確かめた 24 件はすべて期待どおりだった）。 | 注記・バッククォート・エスケープ・空白なしの `\|limit` を 1〜2 件ずつ同じ表に足す。 | New |
| R-11 | Minor | scripts/compat/insights-export.sh > has_limit_command (移植性) | awk の書き方は POSIX の範囲（`tolower`、`sub`、`047`、`exit(式)`）で、BSD awk と bash 3.2 で動くはずだが、確かめたのは mawk 1.3.4 と bash 5.2、jq 1.7 だけである。macOS の BSD awk、bash 3.2、jq 1.6 での実行は手元の初回になる（計画の既知の残りで、前回の Loop-back でも同じ扱い）。 | 手元の互換の確かめで、まず `has_limit_command` を数件のクエリで試してから本番の期間を流す。結果を code-summary.md に記録する。 | New |

> R-01 Earlier decision: Accepted risk

> R-02 Earlier decision: Accepted risk

> R-03 Earlier decision: Accepted risk

> R-04 Earlier decision: Accepted risk

> R-05 Earlier decision: Accepted risk

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| cargo fmt --check / bash -n / cargo test --workspace / examples 51+7 / clippy / traceability sensor（orchestrator 実行済み） | PASS | 依頼文のとおり。再実行はしていない。 |
| has_limit_command の手元実験（mawk、24 ケース） | 23 件が期待どおり、1 件（`parse` の正規表現内の `\| limit`）が偽陽性 | 文字列・バッククォート・`#` 注記・`like` と `=~` の正規表現・大文字・`\|limit`・`limitx` / `limit_x` / `limit = 3`（名前）を正しく区別した。偽陽性は 1 件目の Minor。 |

### Summary

Step 15 の `limit` 判定は、先頭と `\|` の後ろの `limit` だけをコマンドとみなし、文字列・名前・注記・`like` / `=~` の正規表現を先に隠すので、q01・q02・q09・q10 の原因（`--limit` が `limit` を上書き）を正しく解消し、誤判定は `parse` の正規表現という同梱の組の外の場合に限られる。Step 16 の真偽値 1/0 とテスト、Step 17 の p10〜p12 と `like eventSource` への差し替えも整合しており、ログの中身や認証情報はリポジトリに入っておらず、Logs Insights を呼ぶのは開発者が手で流すスクリプト（`start-query` と `get-query-results` のみ）だけで、Forbidden の規則に反しない。
