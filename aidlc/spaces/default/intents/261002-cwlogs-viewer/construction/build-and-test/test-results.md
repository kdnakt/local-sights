# Test Results — ビルドとテストの結果（2026-10-10）

実行環境：クラウドのセッション（Linux x86_64、4 CPU、WebKitGTK・GDK なし、AWS の認証情報なし）。コマンドはすべて `build-instructions.md`・各手順書のとおり。実際の AWS には接続していない。

## ビルド

| コマンド | 結果 | 出力の要点 |
|----------|------|------------|
| `npm ci` | 済（前のステージで実施） | — |
| `npx tsc --noEmit` | **成功** | 出力なし |
| `npx vite build` | **成功** | `dist/assets/index-*.js` 271.20 kB（gzip 83.96 kB）、`index-*.css` 3.65 kB。838ms |
| `CARGO_INCREMENTAL=0 cargo build -p local-sights-core --all-targets` | **成功**（`cargo test -p local-sights-core` のビルドとして） | 警告なし |
| `CARGO_INCREMENTAL=0 cargo build -p local-sights` | **失敗（環境の都合）** | `error: failed to run custom build command for gdk-sys v0.18.2` — `Package gdk-3.0 was not found in the pkg-config search path`。この環境に GDK 3・WebKitGTK 4.1 がなく、apt にもパッケージがない。コードの問題ではない。macOS で実行する（Q1） |

## 静的検査と整形

| コマンド | 結果 |
|----------|------|
| `cargo fmt --all --check` | **差分なし** |
| `CARGO_INCREMENTAL=0 cargo clippy -p local-sights-core --all-targets` | **警告・エラーなし** |
| `cargo clippy --workspace --all-targets` | **未実行**（`src-tauri` が上と同じ理由でビルドできない。macOS で実行する） |
| `npx eslint .` | **問題なし** |
| `npx prettier --check .` | **問題なし**（All matched files use Prettier code style） |
| `npm audit` | **0 vulnerabilities** |
| `cargo deny check` | **未実行**（`cargo-deny` と `deny.toml` は次の CI Pipeline のステージで用意する） |

## 単体テスト

ライブラリ側は単位ごとの絞り込みのコマンドを 1 回ずつ実行した（モジュールが重なるため、合計は 305 件を超える）。全体は `cargo test -p local-sights-core` の 1 回。

| 単位 | コマンド（`CARGO_INCREMENTAL=0 cargo test -p local-sights-core --lib -- …`） | 結果 |
|------|------|------|
| U1 | `request:: time_range:: paging:: event:: timeline:: failure:: gateway:: session::` | 162 成功・0 失敗・2 ignored |
| U2 | `catalog:: log_groups:: connection:: session:: gateway::` | 132 成功・0 失敗 |
| U3 | `streams:: retry:: timeline:: coordinator:: fetcher:: session:: request:: gateway::` | 162 成功・0 失敗・2 ignored |
| U4 | `time_range:: time_zone:: date_input:: request:: session::` | 124 成功・0 失敗 |
| U5 | `filter:: log_view:: session::` | 104 成功・0 失敗・3 ignored |
| U6 | `cache:: coordinator:: session:: filter::should_report_progress` | 135 成功・0 失敗 |
| U7 | `log_view:: session:: coordinator:: cache::` | 145 成功・0 失敗・3 ignored |
| 全体 | `cargo test -p local-sights-core --lib` | **305 成功・0 失敗・5 ignored**（ignored は速さの計測。下の「性能」で実行） |

画面側（Vitest）。単位ごとのファイルの組を 1 回ずつ、全体を 1 回。

| 単位 | ファイル数 | 結果 |
|------|-----------|------|
| U1 | 5 | 64 成功 |
| U2 | 6 | 72 成功 |
| U3 | 7 | 89 成功 |
| U4 | 5 | 69 成功 |
| U5 | 4 | 56 成功 |
| U6 | 6 | 69 成功 |
| U7 | 14 | 123 成功 |
| 全体 | `npx vitest run` 21 ファイル | **171 成功・0 失敗**（11.87 秒） |

## 統合テスト

| コマンド | 結果 |
|----------|------|
| `cargo test -p local-sights-core --test u1_fetch_flow` | 11 成功 |
| `cargo test -p local-sights-core --test u2_log_group_listing` | 9 成功 |
| `cargo test -p local-sights-core --test u3_fetch_flow` | 14 成功 |
| `cargo test -p local-sights-core --test u5_filter_flow` | 4 成功 |
| `cargo test -p local-sights-core --test u6_cache_flow` | 3 成功 |
| 合計 | **41 成功・0 失敗** |
| `npx vitest run src/App.test.tsx` | 全体の 171 件に含まれ成功 |
| Doc-tests | 0 件 |

合計：**ライブラリ 346 件成功（305 + 41）、画面 171 件成功、失敗 0**。

## 性能（参考値。達成の判定は手元の Mac で行う — Q2）

`CARGO_INCREMENTAL=0 cargo test -p local-sights-core --release --lib -- timeline:: filter:: log_view:: --ignored --nocapture`、5 件すべて成功（2.33 秒）。実行後に `target/release` を削除。

| 測ったもの | この環境の値 | 要件の期待値（開発者の Mac で） |
|------------|--------------|--------------------------------|
| 10 万件の絞り込み | 3.73 ms | NFR1：10 秒以内 |
| 100 万件の絞り込み | 36.0 ms | NFR2：100 秒以内 |
| 100 万件・絞り込みあり・1 万件のページの切り出し | 103.8 ms | NFR2：スクロールに 1 秒以内に反応 |
| 100 万件を保持したまま行の位置の読み出し | 行 19.2 µs、位置 7.5 µs（絞り込みあり：行 94.5 µs、位置 3.0 µs） | NFR2 |
| 100 万件に重なるページ 1 件のマージ | 成功（時間は出力なし） | NFR2：100 万件を超えても取得を止めない |

純粋なロジックの部分は、要件の値に対して 3 桁以上の余裕がある。GUI を通した値（描画・IPC を含む）は手元で測る。

## セキュリティの確認（静的）

| 確認 | 結果 |
|------|------|
| AWS SDK の操作 | `gateway/aws.rs` の `get_log_events`・`describe_log_groups`・`describe_log_streams` の 3 つだけ。`aws_sdk_cloudwatchlogs` を使うファイルはこの 1 つ |
| README の IAM 権限 | `logs:DescribeLogGroups`、`logs:DescribeLogStreams`、`logs:GetLogEvents` の 3 つを記載 |
| 画面の外部通信 | `fetch(`・`XMLHttpRequest`・`sendBeacon`・`WebSocket` の利用なし |
| CSP（`tauri.conf.json`） | `connect-src ipc: http://ipc.localhost`、`default-src 'self'`、`object-src 'none'` |
| 画面の権限（`capabilities/default.json`） | `core:event:default` とこのアプリの `allow-*` コマンド 21 個だけ |
| 診断ログ | `eprintln!` だけ（`coordinator.rs`・`cache/mod.rs`・`src-tauri/src/lib.rs`）。ログのクレート・ファイル出力なし。`println!` は `#[ignore]` の速さのテストの中だけ |
| アクセスキー ID らしい文字列 | リポジトリに一致なし（practices-discovery の記録にある AWS 公式の例示用キーの言及のみ） |
| 秘密情報の扱いのテスト | `catalog::credential_values_are_never_kept`、`gateway::aws::credentials_failures_require_authentication`、`failure::redact_access_key_ids` の利用、`cache` のテスト — すべて成功（上の単体テストに含む） |
| キャッシュ無効でディスクに書かない | `coordinator::a_disabled_cache_reads_and_writes_nothing_and_fetches_as_u3`、`cache::settings::no_settings_file_means_disabled` ほか — 成功 |

## 失敗の詳細

テストの失敗はない。ビルドの失敗は `cargo build -p local-sights` の 1 件で、原因は環境（GDK 3 なし）。スタックトレースではなく pkg-config の探索失敗のメッセージ：

```
error: failed to run custom build command for `gdk-sys v0.18.2`
  pkg-config exited with status code 1
  > PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1 pkg-config --libs --cflags gdk-3.0 'gdk-3.0 >= 3.22'
    Package gdk-3.0 was not found in the pkg-config search path.
```

## カバレッジ

数値のカバレッジ下限は設けていない（team.md）。カバレッジの計測ツール（`cargo-llvm-cov` など）は入れていない。代わりの指標：ライブラリ側の全モジュールに `#[cfg(test)]` のテストがあり、結合テストが 5 つの境界を覆う（`integration-test-instructions.md`）。

## Target Verification Matrix（確定版）

判定は `Met`（達成）・`Not Met`（未達成）・`Unverified`（未確認）のいずれか。`Unverified` はこの環境で確かめられなかったもので、手元の macOS で確かめるまで残る（Q1）。

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|-----------|--------|----------|--------|----------|--------------|---------|
| TC-1 | U1〜U7 `code-generation-plan.md` § Testing Contract `strategy_volume` | 部品ごとに 5〜8 件のテスト | 各単位の `code-summary.md` の部品別の件数表はすべて目安以上。ライブラリ 305 件・画面 171 件 | 上の単体テストの表、各単位の `code-summary.md` | build-and-test | Met |
| TC-2 | 同 `strategy_volume` | 単体テストに加えて主要な境界の結合テスト | 結合テスト 5 ファイル 41 件、`App.test.tsx` | 上の統合テストの表 | build-and-test | Met |
| TC-3 | 同 `scope_floor` | 既存のテストがすべて通る | 346 + 171 件成功、失敗 0 | 上の単体・統合テストの表 | build-and-test | Met |
| TC-4 | team.md Testing Posture（契約の `applicable_notes`） | ライブラリの公開関数には必ずテスト | 各単位のコード生成レビュー（READY）で確認済み。全モジュールにテストあり | `construction/<unit>/code-generation/reviews/` | build-and-test | Met |
| TC-5 | team.md Testing Posture | テストと CI は実際の AWS に接続しない | 結合テストは偽物の gateway だけ。設定ファイルのテストは一時ファイル | `tests/support/fake_gateway.rs`、`security-test-instructions.md` | build-and-test | Met |
| B-1 | `build-instructions.md` | ライブラリがビルドできる | 成功 | `cargo test -p local-sights-core` のビルド | build-and-test | Met |
| B-2 | `build-instructions.md` | 画面がバンドルできる・型が通る | 成功 | `npx tsc --noEmit`、`npx vite build` | build-and-test | Met |
| B-3 | `build-instructions.md`、各単位の `unit-test-instructions.md` | Tauri アプリが組み立てられる（`cargo build -p local-sights`） | この環境では GDK 3 がなく失敗 | 上の「ビルド」 | build-and-test（手元の macOS） | Unverified |
| B-4 | team.md Code Style | `cargo fmt --check`・`prettier --check` が差分なし | 差分なし | 上の「静的検査と整形」 | build-and-test | Met |
| B-5 | team.md Code Style | clippy のエラーなし（ライブラリ） | エラー・警告なし | 同上 | build-and-test | Met |
| B-6 | team.md Code Style | clippy のエラーなし（ワークスペース全体） | 未実行（`src-tauri` がビルドできない） | 同上 | build-and-test（手元の macOS） | Unverified |
| B-7 | units-generation F2 | `tsc --noEmit`・ESLint・Prettier が通る | すべて問題なし | 同上 | build-and-test | Met |
| B-8 | team.md Code Style | 依存関係の脆弱性なし（npm） | 0 vulnerabilities | `npm audit` | build-and-test | Met |
| B-9 | team.md Code Style | 依存関係の脆弱性なし（cargo-deny） | 未実行（道具と `deny.toml` が未整備） | — | ci-pipeline で整備。それまで未確認 | Unverified |
| NFR1 | requirements.md NFR 表 | 10 万件の絞り込みが 10 秒以内（開発者の Mac） | この環境の参考値 3.73 ms。Mac では未計測 | 上の「性能」 | build-and-test（手元の macOS） | Unverified |
| NFR2 | 同 | 100 万件で絞り込み 100 秒以内・スクロール 1 秒以内・落ちない・取得を止めない（開発者の Mac） | 参考値 36.0 ms / 103.8 ms、マージ成功。GUI では未計測 | 同上 | build-and-test（手元の macOS） | Unverified |
| NFR3 | 同 | 取得中も 1 秒以内に反応し、取得中が表示される（開発者の Mac） | 未計測。取得中の表示は `StatusLine.test.tsx`・`App.test.tsx` で確認 | 単体テスト | build-and-test（手元の macOS） | Unverified |
| NFR4 | 同 | スロットリング時に待機と再試行、続けば FR4.10 のとおり | `retry.rs`・`u3_fetch_flow.rs` の再試行テスト成功（1 秒・2 秒の待ち、最大 5 回、3 ストリーム続けば残りを失敗） | 統合テスト | build-and-test | Met |
| NFR5 | 同 | 秘密の認証情報とアクセスキー ID を書かない・独自に保存しない | テスト成功、リポジトリに一致なし | 上の「セキュリティの確認」 | build-and-test | Met |
| NFR6 | 同 | 呼ぶ API は 3 つだけ、README に IAM 権限 | 3 つだけ、README に記載 | 同上 | build-and-test | Met |
| NFR7 | 同 | テレメトリを送らない、通信先は AWS と SSO だけ | 外部通信のコードなし、CSP は IPC だけ、通信系プラグインなし | 同上 | build-and-test | Met |
| NFR8 | 同 | キャッシュ無効ならログをディスクに書かない | テスト成功、既定は無効 | 同上 | build-and-test | Met |
| NFR9 | 同 | macOS で動く | この環境では確かめられない | — | build-and-test（手元の macOS） | Unverified |
| NFR10 | 同 | 標準部品の見た目だけ | `styles.css` は `color-scheme: light dark` と固定色なし。見た目は目視 | `src/styles.css` | build-and-test（手元の macOS） | Unverified |
| NFR11 | 同 | 最小 1024×640、ダークモードは OS に合わせる | `tauri.conf.json` に `minWidth: 1024`・`minHeight: 640`、`styles.css` に `color-scheme: light dark` | 設定ファイル | build-and-test | Met |
| NFR12 | 同 | 英語と日本語、OS の言語に合わせる、日本語以外は英語 | `detectLocale` と `messages.test.ts`（15 件）成功。`main.tsx` が `navigator.languages` を渡す | 単体テスト | build-and-test | Met |
| NFR13 | 同 | 主な流れをキーボードだけで操作、ダイアログは Escape で閉じる | 部品ごとのキーボード・Escape のテスト（10 ファイル）成功。流れ全体は手元で確かめる | 単体テスト | build-and-test（手元の macOS） | Unverified |
| NFR14 | 同 | ライブラリと GUI の分離、AWS は trait の裏、公開関数にテスト、AWS に接続しない | 構成どおり（`cross-unit-traceability.md`） | 静的確認 | build-and-test | Met |
| NFR15 | 同 | 読み取り API は無料の前提、転送料金は最初の実行後に Cost Explorer で確認、キャッシュを用意 | キャッシュあり（U6）。Cost Explorer の確認は実際の AWS での初回実行後 | — | build-and-test（手元） | Unverified |
| NFR16 | 同 | 診断ログは標準エラー出力だけ | `eprintln!` だけ | 上の「セキュリティの確認」 | build-and-test | Met |

集計：Met 21、Unverified 10、Not Met 0。`Unverified` が残るため、このステージの判定は**失敗**（ステージの定義どおり）。原因は生成したコードではなく、確認に開発者の macOS が要ること（B-3・B-6・NFR1〜NFR3・NFR9・NFR10・NFR13・NFR15）と、道具の整備が次のステージにあること（B-9）。コード生成へ戻して直せるものはない。

## 失敗の扱い（人間の判断）

2026-10-10：利用者は **Accept failure** を選んだ。失敗（`Unverified` 10 件）をこの記録に残したまま、このステージの承認に進む。未確認の項目は手元の macOS（`build-instructions.md`・`performance-test-instructions.md`・`security-test-instructions.md` の「手元で確かめること」）と、次の CI Pipeline のステージ（`cargo-deny`）で解消する。コード生成へのループバックは行っていない（Loop-Back Log なし）。
