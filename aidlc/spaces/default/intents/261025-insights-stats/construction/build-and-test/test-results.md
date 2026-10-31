# Test Results — 261025-insights-stats

流した日：2026-10-30。場所：このコンテナ（Linux）。コミット：U4 のコード生成を承認したあとの `claude/zen-goldberg-0ogpm0` の先頭。

## 1. ビルドと検査

| コマンド | 結果 |
|---|---|
| `cargo build --workspace --all-targets` | 成功 |
| `cargo fmt --check` | 成功 |
| `cargo clippy --workspace --all-targets` | 成功（警告 0） |
| `npx prettier --check .` | 成功 |
| `npx eslint .` | 成功 |
| `npx tsc --noEmit` | 成功 |
| `cargo deny check` | 成功（advisories ok, bans ok, licenses ok, sources ok） |
| `npm audit --audit-level=high` | 成功（found 0 vulnerabilities） |

## 2. テスト

各作業単位の unit-test-instructions.md のコマンドはどれもワークスペース全体のテストに含まれるため、同じものを何度も流さず、次の 2 つにまとめて 1 回ずつ流した。

| コマンド | 通過 | 失敗 | 無視 |
|---|---|---|---|
| `cargo test --workspace --all-targets`：中核ライブラリ | 848 | 0 | 11（手で流す測定のテスト。下の 4） |
| 同：中核の結合テスト（tests/）とアプリ・examples の合計 | 85（`compat_check`）・11（`fetch_check`）ほか、すべて通過 | 0 | 0 |
| `npx vitest run --coverage` | 597（47 ファイル） | 0 | 0 |

作業単位ごとの、unit-test-instructions.md のコマンドの最後の数（code-summary.md より）は次のとおり。

- U1：`query::` 187、`insights_session` 34、`export::` 20、`local-sights --lib` 40、`compat_check` 54、Vitest 16
- U2：`query::` 237、`insights_session` 37、`local-sights --lib` 42、Vitest 6 ファイル 120
- U3：`query::store` 29、`query::runner` 39、`insights_session` 51、`local-sights --lib` 45、`fetch_check` 11、Vitest 9 ファイル 182
- U4：`compat_check` 85、Vitest 2 ファイル 29

## 3. カバレッジ（行）

| 対象 | 結果 | 下限 |
|---|---|---|
| 中核ライブラリ（`cargo llvm-cov -p local-sights-core --lib`） | 96.17%（28,086 行のうち 1,077 行が未実行） | 80% |
| 画面（Vitest v8） | 91.03%（2,154 行のうち 1,961 行） | 80% |

新しいファイル・大きく変えたファイルの行カバレッジは次のとおり。

- 80% 台：`query/store.rs` 87.61%（未実行は主に手で流す測定のテスト）、`query/catalog.rs` 88.13%
- 90% 台：`query/evaluator.rs` 90.85%、`query/parser.rs` 95.76%、`query/runner.rs` 96.30%、`query/plan.rs` 98.45%、`session.rs` 97.90%、`query/stats.rs` 99.22%

## 4. 速さと重さ（`cargo test -p local-sights-core --release --lib -- --ignored`）

11 本すべて通過した（13.58 s）。

| テスト | 目標 | 結果 |
|---|---|---|
| `the_heavy_stats_takes_at_most_three_times_the_sum_by_stream` | 重いクエリが基準の 3 倍以内（U2 BR2.6、Q1 A） | 通過。U2 の作る段階の 3 回の測定で 2.14〜2.37 倍 |
| `the_copy_of_a_large_holder_under_fetch` | 取得中の保持の写しの 1 回の鍵が 50 ms 以内（U3 Step 22） | 通過。U3 の作る段階の測定で、塊に分けたあと 8〜13 ms |
| `four_queries_over_one_hundred_thousand_events`・`four_queries_over_one_million_events`・`a_cancel_during_a_json_sort_of_one_million_events`・`both_screens_full_and_a_previous_result_evaluate_a_json_sort` | 前の作業の速さの目標（10 万件 10 秒・100 万件 100 秒、止められる） | 通過 |
| ログ管理の画面と保持の速さのテスト 5 本 | 前の作業の目標 | 通過 |

## 5. セキュリティ（security-test-instructions.md）

- **アプリが呼ぶ AWS API**：中核とアプリのソースに、Logs Insights の API（`start_query`・`get_query_results`）や書き込みの API の呼び出しはなかった。
  - 見つかったのは、ファイルを開くときの `create_new` と、コメントの中の `StartQuery` の語だけ。
  - 呼ぶのは今までどおり読み取りの 3 つだけ（NFR5）。
- **差分の秘密の情報**：`main` からの差分（`aidlc/`・生成されたスキーマ・ロックファイルを除く）を探した。
  - 12 桁の数・`arn:aws`・アクセスキーの形（`AKIA…`・`ASIA…`）・`reqwest`・`http(s)://` を足した行は、1 つもなかった（NFR4・NFR12）。
- **画面の部品**：新しく足した import は、`../hooks/expansionState` と `../test/fixtures` だけだった。新しい UI のライブラリや独自のデザインは足していない（NFR11）。
- **新しい Tauri のコマンド**：`get_query_catalog`・`find_query_row` は読み取りだけ。権限の一致のテストが通った。

## 6. まだ確かめていないもの（このコンテナでは流せない）

| 目標 | 理由 | 流す場所 |
|---|---|---|
| NFR1（途中結果で取得が遅くならない。合否の数値を実測から決める） | 実際の AWS が要る | 開発者の手元（`fetch_check --partial-results on/off` を 3 回ずつ） |
| SM1・NFR3（コンソールとの互換） | 本人の AWS 認証情報とコンソールの書き出しが要る | 開発者の手元（`insights-export.sh` → `compat_check`。結果は U4 の code-summary.md の表に書く） |
| NFR6・NFR8 の OS（macOS・Windows の CI のランナーで Rust のテストが通る） | このコンテナは Linux だけ。CI は PR で流れる | PR の CI |
| Walking Skeleton の 2・3（macOS・Windows で画面を目で確かめる） | 本物の GUI と実際の AWS が要る | 開発者の Mac と Windows 11 の PC |

どれもコードの誤りではない。このコンテナでは流せない確かめで、コードを直して変わるものではない。そのため Code Generation に戻る直しはない（Loop-back なし）。

## 7. 判断の記録

2026-10-30：上の 6 の 4 つが `Unverified` のため、Build and Test は失敗の扱い（Step 9 の失敗の条件）。

- 失敗の段の 1：この段で直せるものはない。
- 失敗の段の 2：原因はコードの誤りでも作る段階の選び方でもない。変えて直せるもの（版・イメージ・フラグなど）もない。
- 失敗の段の 4：直す手のない場合の問いで、開発者に扱いを諮る。答えはこの下に書く。

2026-10-30：開発者の答えは「Abort」。ここで止め、手元で確かめたあとに続きから再開する。Build and Test の承認は開いていない。

再開したら、次の 4 つの手元の確かめの結果で Target Verification Matrix を更新する。

- macOS・Windows の CI（T-NFR8-OS）
- NFR1 の実測と合否の数値（T-NFR1。U3 のレビュー R-01 の扱いも決める）
- コンソールとの互換（T-SM1。U4 の code-summary.md の表）
- 両 OS の画面の確かめ（T-WS-OS）

2026-10-31：開発者の答えは「PR作ってCI結果みよう」。PR [kdnakt/local-sights#50](https://github.com/kdnakt/local-sights/pull/50) を作り、macOS・Windows の CI の結果で T-NFR8-OS を確かめる。結果が出たら Target Verification Matrix を更新する。残りの手元の確かめ（T-NFR1、T-SM1、T-WS-OS）は、まだ終わっていない。
