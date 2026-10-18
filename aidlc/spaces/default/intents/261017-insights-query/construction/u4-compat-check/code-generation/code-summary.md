# Code Summary — U4 互換の確認用プログラム（u4-compat-check）

上流：`code-generation-plan.md`（Step 1〜Step 9、Loop-back 1 の Step 10〜Step 14）、コードのレビュー 1 回目（`reviews/review-01.md`）の指摘 R-01〜R-05（下の「Loop-back 1 の直し」）、`functional-design/`（`rules.md` の BR1.1〜BR3.3、`functional-spec.md`、`entities.md`、質問票 Q1 = A）、機能設計のレビュー 1 回目の指摘 R-01〜R-06（計画の表のとおり手順の中で直した）、`unit-test-instructions.md`。Testing Contract は `custom`（比べ方などの純粋なロジックはテスト先行、取得の配線とスクリプトは実装してからテスト）。コミットと push はしていない。

## 作ったもの・変えたもの

| ファイル | 中身 | 決まり |
|---|---|---|
| `crates/local-sights-core/examples/compat_queries.txt`（新規） | クエリの組 20 本（`id<TAB>order\|set<TAB>目的<TAB>クエリ`）。functional-spec の 3 章のとおりで、p09 の目的を「数字だけの文字列（JSON の文字列）と文字列の =」に直した。クエリの文だけで、ログの中身は含まない | BR3.1、R-04 |
| `crates/local-sights-core/examples/compat/queries.rs`（新規） | クエリの組の読み込み。`#` の注記と空行を飛ばす。id は英数字と `-` だけで重ならない。クエリの文は 10,000 文字まで | BR3.1 |
| `crates/local-sights-core/examples/compat/settings.rs`（新規） | `settings.json`（RunSettings）の読み込みと確かめ、END が 30 分以上前か、書き出しのフォルダがリポジトリの外か（部品ごとの前方一致、大文字・小文字を無視する指定つき） | BR1.2〜BR1.4、R-05 |
| `crates/local-sights-core/examples/compat/export.rs`（新規） | `<id>.json` の読み込み。Complete は行（`@ptr` を除く）、Error は断った文、それ以外の status は「走り切らなかった」。`exportedAtSeconds` が settings.json と違うか、ないファイルは「ほかの回の書き出し」（Loop-back 1） | BR2.1、R-02、コードのレビュー R-01・R-02 |
| `crates/local-sights-core/examples/compat/format.rs`（新規） | アプリの行を Logs Insights の書き方に合わせる。`@timestamp` は UTC の `YYYY-MM-DD HH:MM:SS.sss`、JSON の数は serde_json の書き方、真偽値は `true`・`false`、値のない列は行に入れない | BR2.2、R-06 |
| `crates/local-sights-core/examples/compat/compare.rs`（新規） | 1 本のクエリの比べ。NotCompared → 断り方 → sort のない上限の Skipped → 列の名前 → 切り目の同順位を除く → 多重集合 → sort の値の並び | BR2.1、BR2.3〜BR2.5、R-01、R-02、R-06 |
| `crates/local-sights-core/examples/compat/report.rs`（新規） | 終了コード（0 互換、1 違いあり、2 比べられなかった）、クエリごとの報告の行、行の要約（時刻と値の先頭 40 文字）、最後の数の要約 | BR2.6、BR3.2、R-02、R-03 |
| `crates/local-sights-core/examples/compat/mod.rs`（新規） | 上の 6 つのモジュールをまとめる | — |
| `crates/local-sights-core/examples/compat_check.rs`（新規） | 確認用プログラム。`--export-dir`（と `--queries`）を受け取り、フォルダと期間を確かめ、settings.json の条件で、クエリの画面と同じ流れ（QueryStore と QueryRunner、DescribeLogStreams・GetLogEvents だけ）で期間を 1 回取得し、クエリごとにアプリの中核で読み取りと評価をして比べ、報告を出す。期間は START の 0 ms から END の 999 ms | BR1.1〜BR1.4、BR2.1〜BR2.6、BR3.2、R-01、R-03、R-05 |
| `scripts/compat/insights-export.sh`（新規、実行の権限つき） | 書き出しのスクリプト。PROFILE（なくてもよい）・REGION・GROUP・START・END・OUT・QUERIES・WAIT_SECONDS。OUT の実際の場所がリポジトリの外か、END が 30 分以上前かを確かめてから、settings.json と、クエリごとの `<id>.json`（status と results、Error のときはリクエスト ID を除いた断った文、どのファイルにも settings.json と同じ `exportedAtSeconds`）を書く。start-query には `--limit 10000` を渡し、標準エラーは一時ファイルに分ける。Error はクエリの書き方の誤り（`MalformedQueryException`）のときだけで、ほかの失敗は Unknown。macOS の bash 3.2 でも動く書き方にした | BR1.1〜BR1.4、R-03、R-05、コードのレビュー R-01〜R-04 |
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
| 仮の決まり（Provisional） | p01〜p09 | 数字だけの文字列と数の `>`・`<`、数字でない文字列どうしの順、真偽値と文字列・`true`、AND・OR・NOT・NOT LIKE・NOT IN の大文字、`limit 0`、数字だけの JSON の文字列と文字列の `=` |

- 比べ方は、sort のある 4 本（q01、q02、q09、q10）が order、残りが set。組のファイルのテストで「order ⇔ sort がある」を確かめる。
- p05（`readOnly = true`）は今のアプリが読めない（`query.unsupported.fieldComparison`、`true` を項目の名前と読む）。BR2.4 で Logs Insights の答えと比べる（両方が断れば BothRejected、Logs Insights だけが読めれば OnlyAppRejected で違いとして出る）。

### 確かめられないまま残る組

- **JSON の数（R-04）**：CloudTrail の項目にはほとんど JSON の数がない（eventVersion も文字列）。JSON の数と文字列・数との比べは確かめられないまま残る。JSON の数を持つロググループで確かめるときは、組のファイルを分けて足し、スクリプトに `QUERIES=`、確認用プログラムに `--queries` で同じファイルを渡す。
- **JSON の数の書き方**：アプリは JSON に書かれた文字のままの数を持たず、serde_json の書き方（整数はそのまま、小数はいちばん短い形）で書く。ログに `1.0` や `1e3` と書かれた数は、アプリでは `1.0`・`1000.0` になり、Logs Insights の書き方と食い違うことがある。違いが出たら、この点を先に疑う。
- **`@log` の値と日時の文字列との比べ**：U1 の決まりと BR3.1 のとおり、組に入れていない。

## テストの数

`cargo test -p local-sights-core --example compat_check`：48 件（Step 9 の時点で 42 件、Loop-back 1 で 6 件を足した。AWS には接続しない）。

| 部品 | 件数 | 進め方 |
|---|---|---|
| `compat/queries.rs`（クエリの組の読み込みと、入れた組そのもの） | 6 | テスト先行 |
| `compat/settings.rs`（settings.json と `exportedAtSeconds`、END、フォルダ） | 5 | テスト先行 |
| `compat/format.rs`（@timestamp、JSON の値、@ptr） | 3 | テスト先行 |
| `compat/export.rs`（書き出しの読み込み、start-query の失敗を書いた Unknown、ほかの回の書き出し） | 5 | テスト先行 |
| `compat/compare.rs`（多重集合、列、並び、同順位、切り目の同順位、列のない行、断り方、Skipped、NotCompared、ほかの回の書き出し） | 11 | テスト先行 |
| `compat/report.rs`（終了コード、行の要約、報告の行、数の要約） | 6 | テスト先行 |
| `compat_check.rs` の配線（偽のゲートウェイと一時フォルダ：同じなら 0、違えば 1、取得の失敗と書き出しの欠けは 2、前の回の書き出しが残っていれば 2、切り目の同順位と境目の秒、prepare のフォルダと期間、引数） | 7 | 実装してからテスト |
| `compat_check.rs` のスクリプト（`bash -n`、偽の `aws` で settings.json と `<id>.json`、リポジトリの中の OUT と近すぎる END で止まる、認証切れ・アクセスの拒否は Unknown で比べない、`--limit 10000` と標準エラーの警告） | 5 | 実装してからテスト（`#[cfg(unix)]`） |

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
- 件数の上限は、クエリに `limit` がなければ 10,000（スクリプトは `--limit 10000` を渡し、確認用プログラムも 10,000 とみなす）。
- 確認用プログラムの報告は 1 行が 1 本のクエリ（`id  結果  insights=件数  app=件数`、`tied-at-cut=…`・`first-out-of-order-row=…`・`boundary-events` の印つき）。違いがあれば片方にだけある行を最初の 5 つまで出す。最後に結果ごとの数を出す。終了コードは 0（互換）、1（違いあり）、2（比べられなかった）。
- `boundary-events` の印のあるクエリで違いが出たら、まず期間の端の秒の扱い（R-03）を疑う。

## 手元の比べの結果（BR3.3）

まだ流していない。流したら、ここと Build and Test の記録に次を残す（ログの中身は残さない）。

- ロググループの種類、期間の長さ、期間の中のイベントの数
- 結果ごとの数（Same・Different・BothRejected・OnlyAppRejected・OnlyInsightsRejected・Skipped・NotCompared）
- 違いのあったクエリと、原因の見立て（アプリの読み取り・評価の誤り、仮の決まりの違い、取得の違い）と、アプリを直すか違いを受け入れて README に書くか

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

## 残っていること

- **手元の比べ（BR3.3）**：開発者が UC1・UC2 を流し、上の「手元の比べの結果」と Build and Test の記録に残す。違いが出たときにアプリを直すか受け入れるかは、そこで決める（functional-spec 8）。1 回目は macOS の bash 3.2 で流す（コードのレビュー R-06）。
