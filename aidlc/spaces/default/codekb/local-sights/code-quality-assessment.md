# コードの品質の見立て（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直しを土台に、2026-10-25、コミット `793da17` の焦点を絞った読み直し（261025-insights-stats）で更新。今回もテスト・ビルド・lint を実行していないため、ここに書くのはコードと設定から読み取れることと、CI の仕組みが保証していることである。物差しは `project.md` の Forbidden と `team.md` の Code Style・Testing Posture。

## テストとカバレッジ

- **置き方**：中核は各モジュールの `#[cfg(test)]` と `tests/` の結合テスト 9 本（`u1_concurrent_fetch`、`u1_fetch_flow`、`u2_call_pacing`、`u2_log_group_listing`、`u3_fetch_flow`、`u3_give_up_rule`、`u5_cancel`、`u5_filter_flow`、`u6_cache_flow`）。例（`fetch_check`、`compat_check`）も `cargo test --examples` でテストされる。アプリは `src-tauri/src/lib.rs`・`sso_cli/mod.rs`・`export_boundary.rs` の `#[cfg(test)]`。画面は部品ごとの `.test.tsx` と補助の `.test.ts`（新しく `ExportMenu`、`ExportNoticeAlert`、`ExpandedMessage`、`jsonPretty`、`messages`、`regions` のテスト）、スクリプトは `scripts/release/*.test.ts`。
- **AWS につながない**：結合テストはすべて `FakeGateway`、CLI は偽のスクリプト、時計は `tokio::test(start_paused = true)` の止まった時計。README の「Tests and checks」も明記。`project.md` の Forbidden（CI・テストから AWS につながない）に合う。
- **純粋なロジックの比率**：時刻順マージ、ページの終端、再試行、ペース、諦めの規則、絞り込み、クエリの読みと評価、日付、SSO の出力の抜き出し、書き出しの文字づくり（`export::formatter`、`ExportSnapshotSource::into_snapshot`）、既定のファイル名、画面側の `elapsed.ts` / `rowLayout.ts` / `dateTime.ts` / `expansionState.ts` / `jsonPretty.ts` / `regions.ts` / `resolveLocale` は副作用のない関数で、テストが先に書ける形になっている（team.md の Testing Posture の「純粋なロジックは先」に沿う）。OS に触る `export_boundary.rs` と Cloudscape のプラグイン呼び出しは実装後にテストする層。
- **カバレッジ**：CI で `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（Linux）と Vitest の `thresholds: { lines: 80 }`。アプリ側（`src-tauri`）はカバレッジを測っていない。`cfg(windows)` のコードは Linux で測るため下限の外（README が明記）。
- **OS ごとのテスト**：`sso_cli/windows.rs` の偽の `aws.cmd`、キャッシュの Windows のテストは `Tauri app (windows-latest)` でだけ動く。書き出しは `cfg` を使わないため OS ごとのテストは増えていない。
- **速度のテスト**：`--ignored` の 100 万件のテスト（`timeline`、`filter`、`log_view`）と、`src/dev/perfRows.ts` の手元の測定。CI では走らない。
- **テストの決まり**：テスト名は英語の文で振る舞いを書く。`unwrap()` / `expect()` はテストだけ。

## lint・整形・型・依存の検査

- Rust：`cargo fmt --all --check` 必須、`cargo clippy` は警告をエラーにしない（team.md のとおり）。中核は `#![warn(missing_docs)]`。
- 画面：`tsc --noEmit`、ESLint 10（typescript-eslint の recommended、react-hooks、react-refresh）、Prettier、`npm audit --audit-level=high`。
- 依存：cargo-deny（`deny.toml`。`a2dd083` で `aws-sdk-cloudwatchlogs` の features を変え、古い rustls 0.21 系の RUSTSEC-2026-0104 を外した。Windows のクリップボードの crate のため `BSL-1.0` を許可に追加）、Dependabot（週次）。
- 版の一致：`npm run check:version`（CI と Release の verify）。
- 改行：LF 固定（`.gitattributes`）。
- CI のジョブ名は `main` の必須チェックと一致させる決まり（team.md）。今の 4 つの名前は `Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (${{ matrix.os }})`（`3208713` から変わっていない）。

## 3 段階の見立て

| 観点 | 見立て | 根拠 |
|---|---|---|
| 境界の設計 | 良い | AWS・ファイル・GUI・時計・乱数・OS のダイアログとクリップボードが trait か関数か境界モジュールで差し替えられる。書き出しは「鍵の中は複製だけ、文字づくりと OS は鍵の外」を明文化している（`export/mod.rs:11-17`） |
| テストの量と質 | 良い | 2 層で 80% の行カバレッジを CI が守る。結合テストは要件 ID を引き、止まった時計で速い。時間依存で不安定だった 1 本は `5ced3bf` で順序を固定した |
| 文書 | 良い | README 605 行、`docs/`、doc コメントに要件 ID。新しいモジュールも同じ書き方。英語で統一 |
| 大きさ | 注意 | `session.rs` 6,835 行（本体 2,791）、`lib.rs` 2,898 行、`parser.rs` 2,553 行、`insights_session.rs` 2,371 行。状態機械と転送と書き出しが 1 ファイルに集まる |
| 結果の形の硬さ | 注意 | クエリの結果は「1 行 = 1 イベント」を中核・画面・書き出し・互換の確かめの 4 か所が前提にしている（下の「この作業から見た構造上の制約」） |
| 記述の古さ | 良い | 前回の (3) の 3 点は直った。今回読んだ範囲で実装と食い違う説明は見つからなかった |

## 前回（2026-10-20）の技術的な負債の 5 項目の今

いずれもコミット `5ced3bf`（PR #36、261020-carryover-fixes）で解消された。証拠だけを残す。

- **(1) 時間依存で不安定な結合テスト**：`tests/u5_cancel.rs` に `fetch_already_stopped`（止める合図を取得の前に送る）が入り、キャッシュ検索中の中断のテストは時計や blocking スレッドの速さに依存しなくなった（FR1.1）。
- **(2) QueryStore の 3 点**：`read_overlaps` は求める範囲と重なる最後の範囲まで読んで止まる（`cache/plan.rs` の `last_overlapping_range`、`verify_through`、FR2.1）。`begin_run` が選ばれなくなったロググループの保持を捨てる（`store.rs:296-310`、FR2.2.1。前回のテストの決まりも反転）。`holdings_epoch` は `QueryResult`・`ResultSummary`・`get_query_rows`・`get_query_row_detail`・`QueryResultTable` から取り除かれ、結果は `run_id` だけで引く（FR2.3）。
- **(3) 記述の古さ**：`DEFAULT_RESULT_LIMIT` の説明（`plan.rs:19-23`）、`elapsed.ts` の説明、`docs/releasing.md` の版の記述（`v0.1.0`・`v0.1.1`・`v0.2.0`、次の版はタグの直前に決める）が直った。
- **(4) 画面の小物**：英語の単数形（`englishSingular`、`translate` の `templateOf`、FR4.1）、`CancelNotice` の常にある `role="status"`（FR4.2）、権限のテストの数の固定をやめて 3 か所の一致だけ（`same_commands`、FR4.3）、進み具合をクエリの容器に（FR4.4）。
- **(5) リージョンの選択欄**：`src/i18n/regions.ts` の `REGION_NAMES`（日英）と `TopBar` の `Select`（絞りあり）。中核は変えていない（FR5）。

## この作業（insights-stats）から見た構造上の制約

依頼の 2 点（`stats` による集計、途中結果の表示）に対して、今のコードが置いている前提を証拠つきで挙げる。設計の起点であり、欠点ではない。

### (A) `stats` と集計関数はパーサで拒まれる

- 場所：`crates/local-sights-core/src/query/parser.rs:186-199`（`OTHER_COMMANDS`）、`841-872`（`command()`：`stats` → `UNSUPPORTED_COMMAND`）、`1131-1157`（`function()`：`toMillis` 以外 → `UNSUPPORTED_FUNCTION`。`fields count(*)` のテスト `2318-2320`）、`202`（`KEYWORDS` に `by` はない）。
- 画面側で同じ一覧を持つ場所：`src/analytics.ts:116-143`（`QUERY_HIGHLIGHT_RULES` の `fields|filter|sort|limit` と `toMillis`）、`src/i18n/messages.ts:680-683`（`insights.error.supportedList`「使える命令：fields、filter、sort、limit」）、`README.md:184-204`（対応表）、中核の補完の候補（`InsightsSession::completion_candidates`、`CompletionKind::Command`）。`stats` を足すとこの 4 か所がそろって変わる。

### (B) 結果の行は「1 行 = 1 イベントの写し」で固定されている

- 中核：`CommandKind` は 4 種（`plan.rs:367-377`）、`QueryPlan::columns() -> Vec<FieldRef>`（`plan.rs:146-160`）、`Evaluation.rows: Vec<usize>`（保持の位置、`evaluator.rs:333-340`）、`copy_rows` → `ResultRow { event, log_group_name }`（`store.rs:528-544`、`evaluator.rs:761-786`）、`QueryResult.rows: Vec<ResultRow>`、セルは `row_cells(columns, EventFields, format_time)` が表示のたびに作る（`evaluator.rs:858-872`、`insights_session.rs:1130-1163`）、開いた行の詳細は `row_detail`（固定 4 項目 ＋ JSON の項目）。
- 画面：`QueryRow` は `LogEvent` を `#[serde(flatten)]` で含む（`insights_session.rs:402-415`、`src/api.ts:369-375`）。`QueryResultTable` は `@timestamp` と `@message` の列幅を特別扱い（`QueryResultTable.tsx:40-55`）。
- 書き出し：`ExportSnapshotSource::LogAnalytics { columns: Vec<FieldRef>, rows: Vec<ResultRow> }`（`exporter.rs:216-236`、`insights_session.rs:1245-1255`）。
- 互換の確かめ：`examples/compat_check.rs` も同じ形で比べる（今回は開いていない）。
- 集計の行（グループの鍵と集計値だけ、元のイベントがない）を表す型はどこにもない。`stats` の結果を通すには、`QueryResult` の行を列挙型にするか集計専用の結果型を足し、`ResultSummary.columns`（`Vec<String>`、こちらはそのまま使える）、`QueryRow`、`QueryRowDetail`、`ExportSnapshotSource` にその形を足す必要がある。

### (C) 評価は取得がすべて終わってから 1 回だけ走り、途中結果の経路がない

- `run_query`（`runner.rs:224-270`）：`begin_run` → `fetch_missing`（ロググループを順に、範囲を順に）→ `on_fetch_ended` → `build_view` → `gather_field_names` → `evaluate_held`（`runner.rs:543-592`：10,000 行ずつ `spawn_blocking`、`on_evaluation_progress(evaluated, total)` で続けるか聞き、終わりに `copy_rows` → `on_evaluated(QueryResult)` を 1 回）。
- `QuerySink`（`runner.rs:152-177`）に部分的な結果を渡す通知はない。`on_batch` は `InsightsProgress.event_count` にしか写らない（`lib.rs:2001-2004`、`insights_session.rs:874-880`）。
- 取得中のイベントは `fetch_range` が範囲ごとに作る `Arc<Mutex<EventTimeline>>` に積まれ、範囲が終わってから `settle_range` で `QueryStore` に渡る（`runner.rs:405-466`）。取得の途中では `QueryStore` にもない。
- `InsightsSession` の相は `Fetching → Evaluating → Done`。`result` が入るのは `finish_evaluation` だけ（`insights_session.rs:991-1004`）。実行中は前回の結果を残す（`begin` の `result: self.run.result.take()`、`762-771`）。
- 画面：`ResultBody` は `result !== null` のときだけ表を描き、実行中は `previous`（`LogAnalyticsPage.tsx:628-668`）。`QueryResultTable` は `runId` で窓を読む（`QueryResultTable.tsx:113-132`）。
- 手本：Log Management の `FetchSink::on_batch(BatchProgress { timeline_version })` → `fetch-progress` → 画面が `timelineVersion` を見て `get_rows` を読み直す流れ（`coordinator.rs:218-250`、`src/api.ts:628-641`、`687-703`）。クエリの画面で「取得が進むにつれて行が増える」には、(a) 取得の途中で保持（または holder）から読める形、(b) 増分で進む評価（`filter` と `fields` は増分できる。`sort`・`limit`・`stats` は届いた分で作り直す）、(c) `QuerySink` の途中結果の通知と `InsightsSession` の「途中の結果」の状態と版、画面が実行中も表を読む変更、が要る。`TauriQuerySink::update_progress` の 250 ms（`lib.rs:251`、`1966-1982`）が押し出しの間隔の手持ち。

### そのほか

- 大きいファイル：`session.rs`（本体 2,791 行＋テスト 4,044 行）、`src-tauri/src/lib.rs`（2,898 行）。SSO・設定・クエリの転送・書き出しは切り出せる。`insights_session.rs` に `stats` と途中結果の状態を足すと、さらに大きくなる。
- `deny.toml` の RUSTSEC-2024-0370（proc-macro-error、unmaintained）の無視は Tauri → gtk の経路で、Tauri の更新待ち。
- アプリ側（`src-tauri`）のカバレッジは測っていない。`export_boundary.rs` のダイアログとクリップボードは OS に依存するためテストの外。
- 画面の `LogTableCloudscape` は試作のまま残っている（環境変数で切り替え）。
- `src/api.ts` の型は Rust の型の手写しで、自動生成がない。`stats` の結果の形を足すと、`api.ts` と `src/test/fixtures.ts` の 2 か所を手で合わせる。

## ドキュメント

- `README.md`（605 行）：導入、前提（AWS の設定、IAM の 3 権限）、起動、クエリの対応表（`stats` は「Not supported」）、SSO、手元の確認手順、速度測定、本物の AWS での確認、互換の確かめ、アイコン、テストと検査、リリース、秘密情報。冒頭に言語切替・書き出し・JSON 整形の説明が加わった。
- `docs/install.md`：macOS と Windows の節に分けて、入れ方・初回起動の警告・AWS CLI v2・更新・戻し方・アンインストール。
- `docs/releasing.md`：版の決まり（公開済みは `v0.1.0`・`v0.1.1`・`v0.2.0`、次の版はタグの直前に決める）、手順、失敗時、同じタグでの再実行。
- コードの doc コメント：中核は `missing_docs` の警告で網羅され、要件 ID と作業名を引く。画面の部品も先頭に役割。英語で統一されており、日本語の利用者向け文書は画面の文言（`messages.ts` の `ja`、`regions.ts` の `ja`）だけ。
