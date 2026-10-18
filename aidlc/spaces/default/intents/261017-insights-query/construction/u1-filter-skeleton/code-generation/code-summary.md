# Code Summary — U1 filter の薄い一本（u1-filter-skeleton）

上流：`functional-design/rules.md`（BR1.1〜BR7.3）、`functional-design/functional-spec.md`、`functional-design/entities.md`、承認済みの `code-generation-plan.md`（Step 1〜18）、`unit-test-instructions.md`。進め方は計画の Testing Contract（custom）のとおり。純粋なロジック（読み取り・決まった項目・日時の文字列・評価・結果の行の窓）はテストを先に書いて失敗を確かめてから実装し、QueryStore・QueryRunner・InsightsSession・AppSession・Tauri のコマンド・画面は実装してからテストを書いて実行した。

## 作ったもの・変えたもの

| 部品 | ファイル | 内容 |
|------|----------|------|
| QueryPlan | `crates/local-sights-core/src/query/plan.rs` | 読み取った結果の型（Condition・FieldRef・CompareOp・Literal・QueryRegex・LikePattern）と読み取りのエラー（QueryProblem：Syntax／Unsupported、場所つき） |
| QueryParser | `crates/local-sights-core/src/query/parser.rs` | `parse_query`。`\|` でつないだ filter の並び、注記、文字列・数値・正規表現・値の並び、優先順位 not > and > or、`toMillis(@timestamp)`、ほかの命令・関数・演算子は対応外（BR3.1〜BR3.8、BR5.6、BR5.7）。メッセージのキー 31 個を `keys::ALL` に持つ |
| 決まった項目 | `crates/local-sights-core/src/query/fields.rs` | `read_field`：@timestamp・toMillis(@timestamp)・@message・@logStream・@log（ロググループ名）（BR4.1） |
| 日時の文字列 | `crates/local-sights-core/src/query/instant.rs` | `is_date_text`・`parse_date_text`・`resolve_date_texts`。`YYYY-MM-DD HH:MM:SS[.sss]` は選んだタイムゾーン、末尾 `Z` つきは UTC（BR5.9）。`time_range::instant_in_zone` を共用 |
| QueryEvaluator | `crates/local-sights-core/src/query/evaluator.rs` | 比べる・and/or/not・like・正規表現・in の判定（BR5.1〜BR5.5、BR5.10）。10,000 件ずつの評価（`Evaluation`、BR5.8）。結果は QueryHoldings の位置の並び（`QueryResult::window`、レビュー R-05、BR6.4） |
| QueryStore | `crates/local-sights-core/src/query/store.rs` | クエリの画面専用の保持。実行のたびに捨てて取り直し、捨てるたびに epoch を 1 増やす（BR2.1）。TimelineStore・HeldEvents を実装し、今の取得の流れの行き先になる |
| QueryRunner | `crates/local-sights-core/src/query/runner.rs` | `run_query`：今の `run_fetch_with_cache` で QueryStore に取得し（BR2.2）、Completed／CompletedWithFailures で、しかも取り消されていないときだけ評価する（BR2.5〜BR2.7、レビュー R-01） |
| InsightsSession | `crates/local-sights-core/src/insights_session.rs` | クエリの画面の入力と状態（BR1.2、BR1.4）、[Run query] を押せない理由の順（BR1.5）、取得に入るときに前の結果を消す（BR2.9）、最後の実行の結果だけを出す（BR6.2）、接続を変えたときに捨てるもの（BR7.2） |
| AppSession | `crates/local-sights-core/src/session.rs` | FetchOwner と `any_fetch_running`（取得は 2 つの画面で同時に 1 つ、BR2.3、レビュー R-02）、`begin_query`（ロックの外でクエリを読み、1 回の呼び出しで理由と revision を確かめて取得に入る、レビュー R-04）、閉じる確認と接続の変更の条件（BR2.8、BR7.1、BR7.3）、`SessionError::StaleInput` |
| Tauri のコマンド | `src-tauri/src/lib.rs`、`src-tauri/build.rs`、`src-tauri/capabilities/default.json` | `update_analytics_input`・`run_query`・`cancel_query`・`get_query_rows`。評価は区切りごとに spawn_blocking。進み具合の通知は 250 ms に間引く。行は runId と epoch を添えて見えている分だけ返す（BR6.4） |
| 画面 | `src/App.tsx`、`src/components/AppNavigation.tsx`・`LogAnalyticsPage.tsx`・`LogGroupMultiselect.tsx`・`QueryEditor.tsx`・`QueryResultTable.tsx`、`src/analytics.ts`・`platform.ts`・`aceLoader.ts`・`api.ts` | ナビゲーションと Ctrl/Cmd+1・2 の切り替え（BR1.1）、ロググループは 1 つ（BR1.3）、ボタンと Ctrl/Cmd+Enter だけで実行（BR1.6）、ログ管理の画面は隠すだけで残す（BR1.7）、処理中の表示と [Cancel]（BR2.4）、一部失敗の注意（BR2.5）、全体の失敗（BR2.6）、取り消し（BR2.7）、読み取りのエラーの場所に移る（BR3.9）、0 件の表示（BR6.1）、結果の表（BR6.3） |
| 文言 | `src/i18n/messages.ts`、`src/dateTime.ts`、`src/components/ConfirmDialog.tsx`・`ConditionArea.tsx` | nav.*・insights.*・query.*（英語・日本語）。接続を変えるときの確認の文に両方の画面を書く（BR7.1）。クエリの取得中はログ管理の画面の入力を止める |
| 確認用プログラム | `crates/local-sights-core/examples/fetch_check.rs` | `--query TEXT`。QueryStore に取得して評価し、`matched N of M events` と先頭 10 件を出す（team.md Walking Skeleton の 2）。手順は `unit-test-instructions.md` |

## テストの数

| 部品 | 新しいテスト |
|------|-------------|
| query/parser.rs | 11 |
| query/fields.rs | 5 |
| query/instant.rs | 8 |
| query/evaluator.rs | 13 |
| query/store.rs | 6 |
| query/runner.rs | 7 |
| insights_session.rs | 9 |
| session.rs（`query_`・`insights_` で始まるもの） | 9 |
| src-tauri/src/lib.rs | 6 |
| examples/fetch_check.rs | 2 |
| AppNavigation.test.tsx | 6 |
| LogGroupMultiselect.test.tsx | 7 |
| QueryEditor.test.tsx | 6 |
| QueryResultTable.test.tsx | 6 |
| LogAnalyticsPage.test.tsx | 12 |
| App.test.tsx | 7 |
| messages.test.ts | 1 |

全体：中核ライブラリの単体テスト 451 → 519（ほかに速さのテスト 5 件は ignored）、結合テスト 78（変化なし）、アプリ 20 → 26、確認用プログラム 3 → 5、画面 392 → 437。

## 確かめたこと（Step 17）

| コマンド | 結果 |
|----------|------|
| `cargo fmt --check` | 通った |
| `cargo clippy --workspace --all-targets` | 警告 0 |
| `cargo test --workspace` | すべて通った（上の数） |
| `cargo test --release -p local-sights-core --lib -- --ignored` | 速さのテスト 5 件が通った（10 万件 10 秒・100 万件 100 秒を含む） |
| `cargo llvm-cov -p local-sights-core --fail-under-lines 80` | 行カバレッジ 97.28% |
| `npx prettier --check .` | 通った |
| `npx eslint .` | 通った |
| `npx tsc --noEmit` | 通った |
| `npx vitest run` | 41 ファイル・437 件が通った |
| `npm run test:coverage` | 行カバレッジ 89.3% |
| `npm run build` | 通った（ace は別の塊に分かれる） |
| `npm audit --audit-level=high` | 脆弱性 0 |

自動テストは実際の AWS に接続しない（取得はすべて偽のゲートウェイ）。呼ぶ AWS の API は今の流れの DescribeLogStreams と GetLogEvents だけで、新しい API は足していない。

## 計画や設計との違い

- 読み取りのエラーの場所（Span）は QueryProblem と日時の文字列だけが持ち、条件のすべてには付けていない。評価のときのエラーは日時の文字列から出るものだけのため。
- 既定のクエリは `filter @message like /ERROR/`（filter だけ）とした。U1 は filter だけを読むため。
- 取得中の進み具合は、ストリームの一覧のページ数ではなく、選んだストリームの数と終わった数で出す。
- 今の App のテストのうち、Tab の順をすべて確かめる 2 件に、ナビゲーションの入る位置（`app-navigation-close-button`・`app-navigation-links`）を足した。確かめる中身は弱めていない。
- `confirm.message` の文を両方の画面のログと結果が消える文に変えた（BR7.1）。ConfirmDialog のテストの文も合わせた。
- クエリの画面の取得中は、ログ管理の画面の条件の入力も止める（取得は同時に 1 つ、BR2.3）。理由の文は `session.queryRunning`。
- InsightsView に、ロググループの選択肢（`log_group_options`）と一覧の状態（`log_group_status`）を足した。AppSession が今の一覧から入れる。
- 取り消しの文は「Query cancelled.」／日本語の対訳とした。
- `ace-builds`（^1.44.0）を package.json の dependencies に足した（CodeEditor の読み込み）。
- `tauri build` が 4 つのコマンドの権限ファイルを `src-tauri/permissions/autogenerated/` に作った（今のコマンドと同じく、リポジトリに置く）。
- 手元で Tauri のアプリをビルドするため、CI と同じ GTK・WebKitGTK の開発用パッケージを入れた（リポジトリは変えていない）。

## 残っていること

- team.md Walking Skeleton の 2（実際の AWS で確認用プログラムを動かす）と 3（macOS と Windows で GUI を目で確かめる）は、開発者本人の手元で行う。この作業ではしていない。
- 範囲の記録と取り足し・キャッシュの一部の利用・複数のロググループは U3、コンソールの細部の確定は U2 と U4（project.md の Corrections のとおり）。
