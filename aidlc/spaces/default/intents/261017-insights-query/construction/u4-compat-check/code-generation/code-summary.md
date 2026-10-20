# Code Summary — U4 互換の確認用プログラム（u4-compat-check）

上流：`code-generation-plan.md`（Step 1〜Step 9、Loop-back 1 の Step 10〜Step 14、Loop-back 2 の Step 15〜Step 19）、互換の確かめの結果（`construction/build-and-test/test-results.md` の 11 章と Loop-Back Log の Loop-back 2、下の「Loop-back 2 の直し」）、コードのレビュー 1 回目（`reviews/review-01.md`）の指摘 R-01〜R-05（下の「Loop-back 1 の直し」）、`functional-design/`（`rules.md` の BR1.1〜BR3.3、`functional-spec.md`、`entities.md`、質問票 Q1 = A）、機能設計のレビュー 1 回目の指摘 R-01〜R-06（計画の表のとおり手順の中で直した）、`unit-test-instructions.md`。Testing Contract は `custom`（比べ方などの純粋なロジックはテスト先行、取得の配線とスクリプトは実装してからテスト）。コミットと push はしていない。

## 作ったもの・変えたもの

| ファイル | 中身 | 決まり |
|---|---|---|
| `crates/local-sights-core/examples/compat_queries.txt`（新規） | クエリの組 23 本（`id<TAB>order\|set<TAB>目的<TAB>クエリ`）。functional-spec の 3 章の 20 本で、p09 の目的を「数字だけの文字列（JSON の文字列）と文字列の =」に直した。Loop-back 2 で p05 の目的を直し、p10〜p12 を足した。クエリの文だけで、ログの中身は含まない | BR3.1、R-04 |
| `crates/local-sights-core/examples/compat/queries.rs`（新規） | クエリの組の読み込み。`#` の注記と空行を飛ばす。id は英数字と `-` だけで重ならない。クエリの文は 10,000 文字まで | BR3.1 |
| `crates/local-sights-core/examples/compat/settings.rs`（新規） | `settings.json`（RunSettings）の読み込みと確かめ、END が 30 分以上前か、書き出しのフォルダがリポジトリの外か（部品ごとの前方一致、大文字・小文字を無視する指定つき） | BR1.2〜BR1.4、R-05 |
| `crates/local-sights-core/examples/compat/export.rs`（新規） | `<id>.json` の読み込み。Complete は行（`@ptr` を除く）、Error は断った文、それ以外の status は「走り切らなかった」。`exportedAtSeconds` が settings.json と違うか、ないファイルは「ほかの回の書き出し」（Loop-back 1） | BR2.1、R-02、コードのレビュー R-01・R-02 |
| `crates/local-sights-core/examples/compat/format.rs`（新規） | アプリの行を Logs Insights の書き方に合わせる。`@timestamp` は UTC の `YYYY-MM-DD HH:MM:SS.sss`、JSON の数は serde_json の書き方、真偽値は `1`・`0`（Loop-back 2。U2 の直しで結果の一覧が出す値のまま）、値のない列は行に入れない | BR2.2、R-06 |
| `crates/local-sights-core/examples/compat/compare.rs`（新規） | 1 本のクエリの比べ。NotCompared → 断り方 → sort のない上限の Skipped → 列の名前 → 切り目の同順位を除く → 多重集合 → sort の値の並び | BR2.1、BR2.3〜BR2.5、R-01、R-02、R-06 |
| `crates/local-sights-core/examples/compat/report.rs`（新規） | 終了コード（0 互換、1 違いあり、2 比べられなかった）、クエリごとの報告の行、行の要約（時刻と値の先頭 40 文字）、最後の数の要約 | BR2.6、BR3.2、R-02、R-03 |
| `crates/local-sights-core/examples/compat/mod.rs`（新規） | 上の 6 つのモジュールをまとめる | — |
| `crates/local-sights-core/examples/compat_check.rs`（新規） | 確認用プログラム。`--export-dir`（と `--queries`）を受け取り、フォルダと期間を確かめ、settings.json の条件で、クエリの画面と同じ流れ（QueryStore と QueryRunner、DescribeLogStreams・GetLogEvents だけ）で期間を 1 回取得し、クエリごとにアプリの中核で読み取りと評価をして比べ、報告を出す。期間は START の 0 ms から END の 999 ms | BR1.1〜BR1.4、BR2.1〜BR2.6、BR3.2、R-01、R-03、R-05 |
| `scripts/compat/insights-export.sh`（新規、実行の権限つき） | 書き出しのスクリプト。PROFILE（なくてもよい）・REGION・GROUP・START・END・OUT・QUERIES・WAIT_SECONDS。OUT の実際の場所がリポジトリの外か、END が 30 分以上前かを確かめてから、settings.json と、クエリごとの `<id>.json`（status と results、Error のときはリクエスト ID を除いた断った文、どのファイルにも settings.json と同じ `exportedAtSeconds`）を書く。start-query には、クエリに `limit` のコマンドがないときだけ `--limit 10000` を渡し（Loop-back 2）、標準エラーは一時ファイルに分ける。Error はクエリの書き方の誤り（`MalformedQueryException`）のときだけで、ほかの失敗は Unknown。macOS の bash 3.2 でも動く書き方にした | BR1.1〜BR1.4、R-03、R-05、コードのレビュー R-01〜R-04 |
| `crates/local-sights-core/Cargo.toml` | `[[example]] compat_check` を登録した（新しいクレートは足していない） | — |
| `.gitignore` | `insights-export/` を念のため足した | BR1.2 |
| `README.md` | 「Compare Log Analytics with the console's Logs Insights」の節と、テストのコマンドの 1 行 | — |
| `crates/local-sights-core/src/query/store.rs`（書式だけ） | U3 で足した計測のテストの行が rustfmt の 100 桁を超えていたため、`rustfmt` をかけて整えた。振る舞いは変えていない（コードのレビュー R-05） | — |

中核ライブラリ（`src/`）の振る舞いと画面は変えていない（`store.rs` は書式だけ）。

## クエリの組

| 種類 | id | 中身 |
|---|---|---|
| 資料の例（DocExample） | q01〜q05 | fields・sort・limit、like、`(?i)`、in、and |
| 基本 | q06〜q11 | JSON の入れ子、`=~`、`not (…)`、自然順の sort、ない値の sort、sort のないとき（集まりだけ） |
| 仮の決まり（Provisional） | p01〜p12 | 数字だけの文字列と数の `>`・`<`、数字でない文字列どうしの順、真偽値と文字列、引用符のない `true`（項目）、AND・OR・NOT・NOT LIKE・NOT IN の大文字、`limit 0`、数字だけの JSON の文字列と文字列の `=`、項目どうしの `=`、右辺の項目がないときの `!=`、数字でない文字列と数字だけの文字列の大小 |

- 比べ方は、sort のある 4 本（q01、q02、q09、q10）が order、残りが set。組のファイルのテストで「order ⇔ sort がある」を確かめる。
- p05（`readOnly = true`）は、U2 の Loop-back 2 の直しで、アプリも引用符のない `true` を項目の名前として読む（項目どうしの比べ）。組のファイルのテストで、p05 と p10〜p12 をアプリが読めることを確かめる。今の組には、アプリが断るクエリはない（断り方の比べ BR2.4 は、`compare.rs`・`report.rs` のテストで、今もアプリが断る `filter eventName like eventSource` の形の例で確かめる）。

### 確かめられないまま残る組

- **JSON の数（R-04）**：CloudTrail の項目にはほとんど JSON の数がない（eventVersion も文字列）。JSON の数と文字列・数との比べは確かめられないまま残る。JSON の数を持つロググループで確かめるときは、組のファイルを分けて足し、スクリプトに `QUERIES=`、確認用プログラムに `--queries` で同じファイルを渡す。
- **JSON の数の書き方**：アプリは JSON に書かれた文字のままの数を持たず、serde_json の書き方（整数はそのまま、小数はいちばん短い形）で書く。ログに `1.0` や `1e3` と書かれた数は、アプリでは `1.0`・`1000.0` になり、Logs Insights の書き方と食い違うことがある。違いが出たら、この点を先に疑う。
- **`@log` の値と日時の文字列との比べ**：U1 の決まりと BR3.1 のとおり、組に入れていない。

## テストの数

`cargo test -p local-sights-core --example compat_check`：51 件（Step 9 の時点で 42 件、Loop-back 1 で 6 件、Loop-back 2 で 3 件を足した。AWS には接続しない）。

| 部品 | 件数 | 進め方 |
|---|---|---|
| `compat/queries.rs`（クエリの組の読み込みと、入れた組そのもの、`like` の後ろの項目をアプリが断ること） | 7 | テスト先行 |
| `compat/settings.rs`（settings.json と `exportedAtSeconds`、END、フォルダ） | 5 | テスト先行 |
| `compat/format.rs`（@timestamp、JSON の値、真偽値の `1`・`0`、@ptr） | 4 | テスト先行 |
| `compat/export.rs`（書き出しの読み込み、start-query の失敗を書いた Unknown、ほかの回の書き出し） | 5 | テスト先行 |
| `compat/compare.rs`（多重集合、列、並び、同順位、切り目の同順位、列のない行、断り方、Skipped、NotCompared、ほかの回の書き出し） | 11 | テスト先行 |
| `compat/report.rs`（終了コード、行の要約、報告の行、数の要約） | 6 | テスト先行 |
| `compat_check.rs` の配線（偽のゲートウェイと一時フォルダ：同じなら 0、違えば 1、取得の失敗と書き出しの欠けは 2、前の回の書き出しが残っていれば 2、切り目の同順位と境目の秒、prepare のフォルダと期間、引数） | 7 | 実装してからテスト |
| `compat_check.rs` のスクリプト（`bash -n`、偽の `aws` で settings.json と `<id>.json`、リポジトリの中の OUT と近すぎる END で止まる、認証切れ・アクセスの拒否は Unknown で比べない、`--limit 10000` と標準エラーの警告、`limit` のあるクエリには `--limit` を渡さない） | 6 | 実装してからテスト（`#[cfg(unix)]`） |

テスト先行の部品は、仮の中身でテストが失敗することを確かめてから実装した。比べ方のテストを流す中で、Logs Insights の結果が 0 行のときに列の違いとして扱ってしまう誤りが見つかり、列は両方に行があるときだけ比べるように直した。

## 確かめたこと（Step 9）

| コマンド | 結果 |
|---|---|
| `cargo fmt --check` | 通る。既存の `crates/local-sights-core/src/query/store.rs`（U3 のコミット 7fdb731 から）に残っていた書式の違いは、U4 で `rustfmt` をかけて直した（書式だけ。コードのレビュー R-05 で記録を直した） |
| `cargo clippy --workspace --all-targets` | 警告なし |
| `cargo test --workspace` | すべて通る（ライブラリ 622 件ほか）。例のテストは既定では流れない |
| `cargo test -p local-sights-core --examples` | compat_check 42 件、fetch_check 7 件、すべて通る（CI の「example tests (library)」と同じコマンド） |
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 通る。中核ライブラリの行カバレッジ 96.60%（例は対象の外で、下限は変えていない） |
| `npx prettier --check .` | 通る |
| `npx eslint .` | 通る |
| `npx tsc --noEmit` | 通る |
| `npx vitest run` | 41 ファイル、469 件、すべて通る |
| `bash -n scripts/compat/insights-export.sh` | 通る |
| `aidlc engine sensor-traceability --output-path …/traceability.json` | pass（gaps・orphans なし） |

## 手元で流す手順（自動テストではない）

開発者の Mac で、本人の認証情報で流す。CI と自動テストは流さない。

```bash
# 1. Logs Insights の結果を書き出す（OUT はリポジトリの外、END は 30 分以上前。
#    境目の秒にはイベントの少ない時刻を選ぶ。Logs Insights は読んだ量で課金される）
PROFILE=<プロファイル> REGION=us-east-1 GROUP=CloudTrail START=<秒> END=<秒> OUT=~/insights-export \
  ./scripts/compat/insights-export.sh

# 2. アプリの結果と比べる
cargo run -p local-sights-core --example compat_check -- --export-dir ~/insights-export
```

- スクリプトは `aws` と `jq` を使う。終了コードは 0（すべて Complete か Error）、1（Complete・Error 以外のクエリがある。start-query が書き方の誤り以外の理由で失敗したもの（認証切れなど）を含む）、2（設定の誤りで何も流さなかった）。Error は start-query が `MalformedQueryException` で断ったときだけ。
- 1 回目は開発者の Mac の bash 3.2（`/bin/bash`）で流す。スクリプトのテスト（偽の `aws`）は CI の Linux のランナーでだけ流れ、macOS の bash 3.2 での動きは自動では確かめていないため（コードのレビュー R-06）。
- 同じ OUT を使い回してよい。前の回の `<id>.json` が残っていても、`exportedAtSeconds` が settings.json と違えば確認用プログラムは比べず NotCompared にする（ファイルは消さない）。
- 件数の上限は、クエリに `limit` がなければ 10,000（スクリプトは `--limit 10000` を渡し、確認用プログラムも 10,000 とみなす）。`limit` のあるクエリには `--limit` を渡さず、クエリの `limit` に任せる（Loop-back 2）。
- 確認用プログラムの報告は 1 行が 1 本のクエリ（`id  結果  insights=件数  app=件数`、`tied-at-cut=…`・`first-out-of-order-row=…`・`boundary-events` の印つき）。違いがあれば片方にだけある行を最初の 5 つまで出す。最後に結果ごとの数を出す。終了コードは 0（互換）、1（違いあり）、2（比べられなかった）。
- `boundary-events` の印のあるクエリで違いが出たら、まず期間の端の秒の扱い（R-03）を疑う。

## 手元の比べの結果（BR3.3）

1 回目（2026-10-18、開発者の Mac）：CloudTrail のロググループ、期間 1 時間、イベント 992 件。Same 13、Different 6、OnlyAppRejected 1、NotCompared 0（終了コード 1）。違いの原因の見立てと直し方は `test-results.md` の 11 章と下の「Loop-back 2 の直し」のとおり。Loop-back 2 の直しのあと、23 本の組でもう一度流す。

流したら、ここと Build and Test の記録に次を残す（ログの中身は残さない）。

- ロググループの種類、期間の長さ、期間の中のイベントの数
- 結果ごとの数（Same・Different・BothRejected・OnlyAppRejected・OnlyInsightsRejected・Skipped・NotCompared）
- 違いのあったクエリと、原因の見立て（アプリの読み取り・評価の誤り、仮の決まりの違い、取得の違い）と、アプリを直すか違いを受け入れて README に書くか

2 回目（2026-10-19、`b088e2b`）：同じ条件、23 本。Same 20、Different 3（p08・p11・p12、どれも U2 の仮の決まり）。Loop-back 3 で U2 を直した。`test-results.md` の 13 章。

3 回目（2026-10-19、`8af7d04`）：同じ条件、23 本すべて Same。T-SM1 を Met にした。`test-results.md` の 15 章。

4 回目（2026-10-20、配布した v0.2.0 の `cd453f0`）：同じ条件、23 本すべて Same。依存関係の更新のあとも互換が保たれている。`test-results.md` の 16 章。

## 計画や設計との違い

- **モジュールを 2 つ足した**：計画の Step 1 は `export.rs`・`format.rs`・`compare.rs`・`report.rs` の 4 つだったが、クエリの組の読み込み（Step 2）を `queries.rs`、settings.json とフォルダ・期間の確かめ（Step 3）を `settings.rs` に分けた。
- **大文字・小文字を無視する OS**：計画（R-05）は Windows だけだったが、macOS の既定のファイルシステムも大文字・小文字を区別しないため、確認用プログラムもスクリプトも Windows と macOS で区別せずに比べる（リポジトリの中を誤って通さない、守りを強める向き）。
- **BR1.4 の「違う条件を渡したらエラー」**：計画のとおり、確認用プログラムは条件を settings.json からしか読まず、条件の引数を持たないため、この誤りは起きない。
- **R-01 の「切る前の並び」**：同じクエリの最後の `limit` を 1 つ大きくした（`limit` がなければ 10,001 の `limit` を足した）クエリを同じ取得のイベントで評価し、切り目のすぐ後ろの行を得る。sort の項目が `fields` の列にないクエリは、sort の値が両方に見えないため、上限で切れたら Skipped にする（今の組にはない）。
- **R-02 の書き出しが読めないとき**：`<id>.json` が JSON として読めないときも NotCompared（終了コード 2）にした。
- **スクリプトの細部**：PROFILE はなくてもよい（entities.md のとおり）。1 本のクエリを待つ時間の上限 `WAIT_SECONDS`（既定 600 秒）を足し、待ちきれなかったクエリは status が Running のまま書かれて NotCompared になる。リクエスト ID を除いた断った文は CLI のエラーの最後の 1 行。
- **テストの数**：計画の約 30 件に対して 42 件（配線とスクリプトのテストを足した）。Loop-back 1 で 48 件になった。
- **`cargo test --workspace` と例のテスト**：計画と unit-test-instructions は「`cargo test --workspace` でも流れる」としていたが、Cargo は既定で例をコンパイルするだけでテストは流さない。例を `test = true` にすると cargo-llvm-cov の対象に入りうるため変えず、`cargo test -p local-sights-core --examples`（CI の rust-core の手順と同じ）で流した。そのため CI でスクリプトのテストが流れるのは Linux のランナーだけ（macOS・Windows の `cargo test --workspace` では例のテストは流れない）。

## Loop-back 1 の直し（Step 10〜Step 13）

Build and Test から戻り、コードのレビュー 1 回目の指摘を直した。アプリ本体と中核ライブラリの振る舞いは変えていない。

| 指摘 | 直したこと | 手順 |
|---|---|---|
| R-01（Major）start-query の失敗をすべて断りとして書いていた | スクリプトは、CLI の標準エラーに `MalformedQueryException` があるときだけ `status: "Error"`（BR2.4 の断り）と書く。それ以外の失敗（認証切れ、アクセスの拒否、スロットリング、ロググループがない、ネットワークの断など）は `status: "Unknown"`、リクエスト ID を除いた断った文、空の `results` を書き、要約の other に数える（スクリプトの終了コードは 1）。確認用プログラムは Unknown を NotCompared にする（終了コード 2）。テストは偽の `aws` の `ExpiredTokenException` と `AccessDeniedException` で、Unknown になること、要約が `Error: 1  other: 2` になること、その書き出しを確認用プログラムにかけると NotCompared で終了コード 2 になることを確かめる | Step 10 |
| R-02 同じ OUT に前の回の `<id>.json` が残ると、古い結果と比べてしまう | スクリプトは、どの `<id>.json`（Complete・Error・Unknown のどれも）にも settings.json と同じ `exportedAtSeconds` を書く。確認用プログラムは、`<id>.json` の値が settings.json と違うか、どちらかにないとき、そのクエリを NotCompared にし、報告の note に両方の値を出す。ファイルは消さない。純粋なロジック（`parse_export_of_run`、比べ）はテストを先に書き、失敗を確かめてから実装した | Step 11 |
| R-03 `--limit` を渡していなかった | start-query に `--limit 10000` を渡す。確認用プログラムの上限 10,000（`row_cap`）と同じ前提をスクリプトの説明に書いた。テストは偽の `aws` の呼び出しの記録で確かめる | Step 12 |
| R-04 成功時も標準エラーをクエリの ID に混ぜていた | start-query の標準エラーは一時ファイル（`mktemp`、終わるときに `trap` で消す）に分け、クエリの ID には標準出力だけを使う。失敗のときだけ一時ファイルを断った文として読む。テストは、成功しつつ標準エラーに警告を出す偽の `aws` で Complete の書き出しになること、一時ファイルが残らないことを確かめる | Step 10、Step 12 |
| R-05 記録が store.rs の整形と合っていなかった | この要約の「確かめたこと」と「作ったもの・変えたもの」を直し、`source-manifest.json` に `store.rs`（書式だけ）を足した | Step 13 |
| R-06 スクリプトのテストは Linux の CI だけ | 計画のとおりここでは直さず、1 回目を macOS の bash 3.2 で流すことを上の手順に書いた | — |

計画との違い：計画の Step 10 は「先にテスト」と書いていたが、スクリプトは境界の層のため、Testing Contract（custom）の進め方に合わせて、スクリプトを先に直してからそのテストを書いて流した。確認用プログラムの側の判断（Unknown・ほかの回の書き出し）はテストを先に書いた。

### 確かめたこと（Step 14）

| コマンド | 結果 |
|---|---|
| `bash -n scripts/compat/insights-export.sh` | 通る |
| `cargo test -p local-sights-core --example compat_check` | 48 件、すべて通る |
| `cargo fmt --all --check` | 通る |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし |
| `cargo test -p local-sights-core` | すべて通る（ライブラリ 622 件ほか）。1 回目に、U4 と関係のない結合テスト `u5_cancel` の `a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss` が 1 度だけ落ちた（偽のゲートウェイが台本より多い DescribeLogStreams を受けた）。単独で 5 回、全体でもう 1 回流してすべて通ったため、時間に依存したたまたまの失敗と見ている。U4 はこのテストと中核ライブラリの振る舞いに触れていない |
| `npx prettier --check .` | 通る |
| `aidlc engine sensor-traceability --output-path …/traceability.json` | pass（gaps・orphans・invalid なし、findings 0） |

## Loop-back 2 の直し（Step 15〜Step 18）

Build and Test の互換の確かめ（`test-results.md` の 11 章）から戻った直し。開発者は p04（真偽値を `1`／`0` と出す）と p05（項目どうしの比べに対応する）をコンソールに合わせると決め、U2 は `fe4910a` で直し済み。U4 ではスクリプトの誤りと、U2 の直しに合わせた比べの道具を直した。アプリ本体と中核ライブラリ（`src/`）は変えていない。

| 違い・決まり | 直したこと | 手順 |
|---|---|---|
| q01・q02・q09・q10 が Different（Logs Insights だけが 992・68・992・992 行） | 原因はスクリプト。Loop-back 1 の R-03 の直しで start-query に必ず `--limit 10000` を渡していたため、API の上限がクエリの `limit 20` などより優先され、Logs Insights がすべての行を返した（アプリの 20・20・200・50 行が正しい）。スクリプトは、クエリに `limit` のコマンド（クエリの先頭か `\|` の後ろの最初の言葉が `limit`、大文字・小文字を問わない）がないときだけ `--limit 10000` を渡す。判定の前に、引用符の文字列、`` ` `` の名前、正規表現（`like`・`=~`・`(`・`,` の後ろの `/…/`）、`#` の注記を空白に置き換えるため、`like /limit/` や文字列の中の `\| limit` はコマンドとみなさない。判定は awk（macOS の awk でも動く書き方）。説明の前提の 1 行も直した | Step 15 |
| p04 が Different（値の書き方） | `format.rs` の BR2.2 の書き方の合わせで、真偽値を `1`／`0` とする。アプリの行は U2 の直しのあとの `row_cells`（結果の一覧が出す値）をそのまま使うため、実装の行は変わらず、説明だけを直した。テストは、落ちていた期待値を `1` に直し、`false` が `0`（入れ子の項目の `true` は `1`、文字列の `"false"` はそのまま）になる例を足した | Step 16 |
| p05 が OnlyAppRejected | U2 の直しで、アプリも引用符のない `true` を項目として読む。組のファイルの p05 の目的を「引用符のない true は項目（ない項目との比べ）」に直した | Step 17 |
| U2 のレビューで残った仮の決まり | p10 項目どうしの `=`（`filter userIdentity.invokedBy = eventSource`）、p11 右辺の項目がないときの `!=`（`filter eventName != errorCode`）、p12 数字でない文字列と数字だけの文字列の大小（`filter eventName > "5"`）を組に足した（20 本 → 23 本） | Step 17 |
| p05 を「アプリが断る例」に使っていたテスト | `compare.rs`・`report.rs` の断り方のテストの例を、今も中核が断る `filter eventName like eventSource`（`query.unsupported.fieldComparison (eventSource)`）に置き換えた。中核がこの形を本当に断ることを `queries.rs` のテストで確かめる。テストの中身（BothRejected・OnlyAppRejected・OnlyInsightsRejected、報告の行の長さ）は変えていない | Step 17 |

計画との違い：

- Step 16 の「先にテスト」：テストの期待値を `1`／`0` にした時点で、U2 の直しにより `app_row` はすでに `1`／`0` を返していたため、テストは失敗を経ずに通った。`format.rs` の実装の行は変えず、説明だけを直した。
- Step 15 の判定は、計画の「先頭か `|` の後ろの `limit`」に加えて、大文字の `LIMIT` もコマンドとみなす（Logs Insights の言葉の大文字・小文字の扱いが確かめられていないため、`LIMIT` のクエリの上限を API で上書きしない向きにした）。`` ` `` の名前と `#` の注記も、文字列と同じく判定の外にした（アプリの読み取りと同じ扱い）。
- 組のファイルの先頭の注記の項目名の一覧に `userIdentity.invokedBy` を足した。
- Step 19 の前に、Step 15・Step 17 で書いたテストの行が rustfmt の書式と違っていたため、`cargo fmt --all` をかけた（変わったのは `compat_check.rs` と `compat/compare.rs` だけ）。
- Step 18 の README：互換の確かめの節には `--limit 10000` の前提の行がもともとなかったため、直すのではなく、「`limit` のないクエリだけ `--limit 10000`（アプリの上限）で始め、`limit` のあるクエリはその `limit` に任せる」の 1 文を足した。

### 確かめたこと（Step 19）

| コマンド | 結果 |
|---|---|
| `bash -n scripts/compat/insights-export.sh` | 通る |
| `cargo test -p local-sights-core --examples` | compat_check 51 件、fetch_check 7 件、すべて通る（直す前は compat_check 47 件が通り、`json_numbers_booleans_and_strings_are_written_as_logs_insights_writes_them` の 1 件が落ちていた） |
| `cargo fmt --all --check` | 通る |
| `cargo clippy --workspace --all-targets` | 警告・エラーなし |
| `cargo test --workspace` | すべて通る（743 件、ライブラリ 634 件ほか、9 件は既定で無視） |
| `npx prettier --check .` | 通る |
| `aidlc engine sensor-traceability --output-path …/traceability.json` | pass（gaps・orphans・invalid なし、findings 0） |

## 残っていること

- **手元の比べ（BR3.3）**：済み。4 回流し、3 回目（`8af7d04`）と 4 回目（v0.2.0 の `cd453f0`）は 23 本すべて Same（上の「手元の比べの結果」と `test-results.md` の 11・13・15・16 章）。左辺の項目がないときの項目どうしの比べだけは組に例がなく、仮の決まりのまま。
