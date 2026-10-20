# コードの品質の見立て（Local Sights）

> 2026-10-20、コミット `3208713`。今回はテスト・ビルド・lint を実行していないため、ここに書くのはコードと設定から読み取れることと、CI の仕組みが保証していることである。物差しは `project.md` の Forbidden と `team.md` の Code Style・Testing Posture。

## テストとカバレッジ

- **置き方**：中核は各モジュールの `#[cfg(test)]` と `tests/` の結合テスト 9 本（`u1_concurrent_fetch`、`u1_fetch_flow`、`u2_call_pacing`、`u2_log_group_listing`、`u3_fetch_flow`、`u3_give_up_rule`、`u5_cancel`、`u5_filter_flow`、`u6_cache_flow`）。例（`fetch_check`、`compat_check`）も `cargo test --examples` でテストされる。アプリは `src-tauri/src/lib.rs` と `sso_cli/mod.rs` の `#[cfg(test)]`。画面は部品ごとの `.test.tsx` と補助の `.test.ts`、スクリプトは `scripts/release/*.test.ts`。
- **AWS につながない**：結合テストはすべて `FakeGateway`、CLI は偽のスクリプト、時計は `tokio::test(start_paused = true)` の止まった時計（`u3_fetch_flow.rs:5`）。README の「Tests and checks」も明記。`project.md` の Forbidden（CI・テストから AWS につながない）に合う。
- **純粋なロジックの比率**：時刻順マージ、ページの終端、再試行、ペース、諦めの規則、絞り込み、クエリの読みと評価、日付、SSO の出力の抜き出し、画面側の `elapsed.ts` / `rowLayout.ts` / `dateTime.ts` / `expansionState.ts` は副作用のない関数で、テストが先に書ける形になっている（team.md の Testing Posture の「純粋なロジックは先」に沿う）。
- **カバレッジ**：CI で `cargo llvm-cov -p local-sights-core --fail-under-lines 80`（Linux）と Vitest の `thresholds: { lines: 80 }`。アプリ側（`src-tauri`）はカバレッジを測っていない。`cfg(windows)` のコードは Linux で測るため下限の外（README が明記）。
- **OS ごとのテスト**：`sso_cli/windows.rs` の偽の `aws.cmd`、キャッシュの Windows のテストは `Tauri app (windows-latest)` でだけ動く。Unix のテストは Linux と macOS のランナー。
- **速度のテスト**：`--ignored` の 100 万件のテスト（`timeline`、`filter`、`log_view`）と、`src/dev/perfRows.ts` の手元の測定。CI では走らない。
- **テストの決まり**：テスト名は英語の文で振る舞いを書く。`unwrap()` / `expect()` はテストだけ。

## lint・整形・型・依存の検査

- Rust：`cargo fmt --all --check` 必須、`cargo clippy` は警告をエラーにしない（`RUSTFLAGS: ""`、team.md のとおり）。中核は `#![warn(missing_docs)]`。
- 画面：`tsc --noEmit`、ESLint 10（typescript-eslint の recommended、react-hooks、react-refresh）、Prettier、`npm audit --audit-level=high`。
- 依存：cargo-deny（`deny.toml`）、Dependabot（週次）。
- 版の一致：`npm run check:version`（CI と Release の verify）。
- 改行：LF 固定（`.gitattributes`）。
- CI のジョブ名は `main` の必須チェックと一致させる決まり（team.md）。今の 4 つの名前は `Rust library (fmt, clippy, test)`、`Front end (tsc, eslint, prettier, vitest, build)`、`cargo-deny`、`Tauri app (${{ matrix.os }})`。

## 3 段階の見立て

| 観点 | 見立て | 根拠 |
|---|---|---|
| 境界の設計 | 良い | AWS・ファイル・GUI・時計・乱数が trait か関数で差し替えられる。OS 依存は 3 か所に閉じている |
| テストの量と質 | 良い | 2 層で 80% の行カバレッジを CI が守る。結合テストは要件 ID を引き、止まった時計で速い |
| 文書 | 良い | README 593 行、`docs/`、doc コメントに要件 ID。英語で統一 |
| 大きさ | 注意 | `session.rs` 6,147 行、`lib.rs` 2,701 行、`parser.rs` 2,553 行。1 ファイルに状態機械と転送が集まる |
| 不安定さ | 注意 | 時間依存の結合テストが 1 本（下の (1)） |
| 記述の古さ | 注意 | 定数の説明と設計の記録に実装と食い違う箇所（下の (3)） |

## 技術的な負債と気になる点

依頼の 5 項目を、コードの場所と証拠つきで挙げる。番号は依頼文の番号。

### (1) 時間依存で不安定な結合テスト

- 場所：`crates/local-sights-core/tests/u5_cancel.rs:518-559` `a_cancel_during_the_cache_lookup_completes_a_hit_and_cancels_a_miss`。
- 仕組み：`fetch_and_stop`（100〜143 行）は取得と「`Duration::ZERO` の後に止める」タスクを `join` する。当たりの側はキャッシュのファイルを置き、外れの側は `FakeGateway::default()` でストリームの台本を持たない。
- なぜ落ちるか：`coordinator.rs:829-844` の `look_up` は `cache.lookup` を `tokio::task::spawn_blocking` に出す。blocking スレッドは止まった時計の外で動くため、止める合図が届く前に `run_fetch` が `DescribeLogStreams` を出す順になることがあり、`tests/support/fake_gateway.rs:293-297` の `expect("FakeGateway received more DescribeLogStreams calls than scripted")` で落ちる（`scripted_answer`、205〜216 行も同じ形）。PR #22 で入り、Dependabot の PR #32 でも一度落ちた（依頼文）。
- 直す向き：外れの側にも 1 ページの台本を持たせて落ちないようにするか、検索の終わりまで止める合図を待たせる順序の保証を入れるか、検索中の中断の規則（当たれば Completed、外れれば列挙前の中断）を 2 つのテストに分ける。

### (2) QueryStore の 3 点（IB-11 U3 の残り）

- `read_overlaps` がファイルを丸ごと読む：`crates/local-sights-core/src/cache/mod.rs:362-403`。重なりが 1 つでもあると `EventSelection::All` で読み、呼び出し側（`query/runner.rs:364-400` `read_cache`、blocking スレッド）が範囲で選ぶ。コメントは書きの検査と同じ読みにした理由（review R-06）を書く。
- 保持が接続を変えるまで増え続ける：`query/store.rs:291-316` `begin_run` は staging だけを捨て、`discard_all`（308 行）を呼ぶのは接続の変更（`src-tauri/src/lib.rs:1332` `clear_held_logs`）だけ。選ばれなくなったロググループの committed も残る（`store.rs:888` のテスト `a_log_group_no_longer_chosen_keeps_its_holdings_until_the_connection_changes` がそれを決まりとして固定している）。
- `holdings_epoch` が未使用：`query/evaluator.rs:777-780` の `QueryResult.holdings_epoch` に「行はもう依存しない」と書かれ、`insights_session.rs:301-320` の `ResultSummary`、`src-tauri/src/lib.rs:1098-1150` の `get_query_rows` / `get_query_row_detail`（「送り返すだけ」）、`src/api.ts:300-317`、`src/components/QueryResultTable.tsx:111-168`（鍵の一部として使う）で 3 層を往復している。`lib.rs` のテスト（2463〜2469 行）が「複製は世代に依存しない」と確かめている。

### (3) 記述の古さ

- `DEFAULT_RESULT_LIMIT` の説明：`crates/local-sights-core/src/query/plan.rs:19-21` は「`limit` が 1〜`MAX_LIMIT` で書かれていないときの行数」と書くが、`has_row_limit`（443〜447 行）は `limit 0` を 0 行の上限として扱う（Loop-back 3、p08）。
- `limit 0` を絞りなしとする古い記述：`examples/compat_check.rs:510` 付近の注記と `row_cap` / `next_row_after_cut`（366、473、519 行付近）は `limit 0` を 10,000 件までの絞りなしとして扱う（比べには影響しない）。IB-11 U2 の code-summary の部品の表（AI-DLC の記録、`aidlc/spaces/default/intents/261017-insights-query/construction/u2-query-language/code-generation/code-summary.md:9, 96`）も同じ古い記述。
- `getSession` の番号付けの古い説明：`src/elapsed.ts:60-88` `nextAnchor` は `fetchNumber` を等しいかどうかだけで比べ、doc コメント（60〜69 行）は「それ以外（新しい取得）は受け取った値」とだけ書く。起動直後に `getSession` の古い応答が、後から届いた新しい取得の running より後に着くと、小さい番号の running が新しい取得として受け取られ、時計が一度戻る（parallel-fetch U4 code-generation review R-01。「次に手を入れるときに直すか、既知の制約として code-summary に残す」とされたが、どちらも行われていない）。コードの中に「番号付け」を説明する古い文言そのものは見つからず、上の解釈で記す。
- `docs/releasing.md:46-48` と `team.md` の「次の版は v0.2.0」は、2026-10-20 の v0.2.0 の公開で古くなった（依頼文の前提。team.md の直しは学びの儀式の範囲）。

### (4) 画面の小物

- 英語の単複：`src/i18n/messages.ts:209`（`{count} streams selected so far`）、213（`{finished}/{planned} streams`）、217（`Failed in {count} streams`）、236（`{n} streams were not finished`）、603、607、627（`{count} streams could not be fetched`）、611（`{count} log groups`）。`translate`（839〜848 行）は `{name}` の置き換えだけで数の分岐がないため、1 のとき「1 streams」になる。日本語には影響しない。
- CancelNotice の読み上げ：`src/components/CancelNotice.tsx:25-31` は `<div role="status">` の囲みと中の `Alert` を同じ描画で出す。ライブリージョンは「すでにある領域の中身が変わった」ときに読むため、囲みと中身が同時に現れると一度も読まれない余地がある。囲みを常に置いて中身だけ出し入れする形が直し方の候補。テスト（`CancelNotice.test.tsx`）は role と文だけを確かめる。
- 権限のテストの固定値：`src-tauri/src/lib.rs:2238-2253` `the_manifest_the_capability_and_the_handler_list_the_same_commands` は `assert_eq!(manifest.len(), 32)` で数を固定し、コメント（2248〜2249 行）に 26 → 30 → 32 の経緯を残す。依頼文の「26」は parallel-fetch の時点の値で、今は 32。3 か所の一致（`handler == manifest`、`allowed == manifest`）だけで十分で、数の固定はコマンドを足すたびに直す手間になる。画面側の `src/commandRegistration.test.ts` は数を固定していない。

### (5) リージョンの選択欄

- `crates/local-sights-core/src/catalog/regions.rs:8-43` の `STANDARD_REGIONS` はコード（`ap-northeast-1` など 34 個）だけ。`region_options`（47〜56 行）がプロファイルの既定リージョンを足して並べ、`catalog/mod.rs:353-356` の `regions()`、`session.rs:1976` の `SessionView.regions` を経て、`src/components/TopBar.tsx:97-101` が `ButtonDropdown` の `text: region` にする。名前（東京、バージニア北部）はコードのどこにもなく、`ButtonDropdown` には検索がない。文言は `messages.ts:157-158`（`connection.region.label` / `placeholder`）。
- 直す向き：名前の表（日英）を中核か画面に持ち、`TopBar` の選択欄を検索できる部品（Cloudscape の `Select` の `filteringType`）に替える。`pacing.rs:80-89` の `SPECIAL_AWS_LIMITS` はコードで引くので影響しない。

### そのほか（今回の範囲外）

- 大きいファイル：`session.rs`（本体 3,120 行＋テスト 3,027 行）、`src-tauri/src/lib.rs`（2,701 行）。SSO・設定・クエリの転送は切り出せる。
- `deny.toml` の RUSTSEC-2024-0370（proc-macro-error、unmaintained）の無視は Tauri → gtk の経路で、Tauri の更新待ち。
- アプリ側（`src-tauri`）のカバレッジは測っていない。`cfg(windows)` のコードは Windows の手元の確認を今回も飛ばすため、CI の `windows-latest` だけが守り。
- 画面の `LogTableCloudscape` は試作のまま残っている（環境変数で切り替え）。
- `src/api.ts` の型は Rust の型の手写しで、自動生成がない。

## ドキュメント

- `README.md`（593 行）：導入、前提（AWS の設定、IAM の 3 権限）、起動、クエリの対応表、SSO、手元の確認手順（Walking Skeleton のチェックポイント）、速度測定、本物の AWS での確認、互換の確かめ、アイコン、テストと検査、リリース、秘密情報。
- `docs/install.md`（334 行）：macOS と Windows の節に分けて、入れ方・初回起動の警告（Gatekeeper、SmartScreen）・AWS CLI v2・更新・戻し方・アンインストール。
- `docs/releasing.md`（150 行）：版の決まり、手順、失敗時、同じタグでの再実行。
- コードの doc コメント：中核は `missing_docs` の警告で網羅され、要件 ID と作業名を引く。画面の部品も先頭に役割。英語で統一されており、日本語の利用者向け文書は画面の文言（`messages.ts` の `ja`）だけ。
