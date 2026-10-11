# Test Results — 画面を AWS マネジメントコンソールに寄せる

実行日：2026-10-11。実行者：Build and Test（手元の Mac、Node 22.21.1、Rust stable）。実際の AWS には接続していない。

## ビルド

| コマンド | 結果 | 要点 |
|---|---|---|
| `npx vite build` | 成功 | `dist/assets/index-*.js` 915.01 kB（gzip 251.07 kB）、`index-*.css` 917.10 kB（gzip 212.52 kB）。500 kB 超の警告は従来どおり |
| `cargo +stable build --workspace` | 成功 | Tauri アプリ `local-sights` を含めて 30.84 秒 |

## 静的検査

| コマンド | 結果 |
|---|---|
| `npx tsc --noEmit` | 成功（出力なし） |
| `npx eslint .` | 成功（指摘 0） |
| `npx prettier --check .` | 成功（All matched files use Prettier code style） |
| `cargo +stable fmt --all --check` | 成功（出力なし） |
| `cargo +stable clippy --workspace --all-targets` | 成功（警告 0） |

## テスト

ステージ単位の unit-test-instructions.md のコマンドは、どれも下の 2 つの全体実行に含まれるため、重複を除いて全体を 1 回ずつ実行した。

| コマンド | 合計 | 成功 | 失敗 | 省略 |
|---|---|---|---|---|
| `npx vitest run` | 246（27 ファイル） | 246 | 0 | 0 |
| `cargo +stable test --workspace` | 363 | 358 | 0 | 5（`#[ignore]` の既存テスト） |

Rust の内訳：ライブラリ 316（省略 5）、`u1_fetch_flow` 11、`u2_log_group_listing` 10、`u3_fetch_flow` 14、`u5_filter_flow` 4、`u6_cache_flow` 3、Tauri アプリ 0。

カバレッジの数値は測っていない（team.md：数値の下限なし）。

## セキュリティの確認

| ID | 結果 | 根拠 |
|---|---|---|
| SEC-1 | 合格 | `npm audit --audit-level=high`：found 0 vulnerabilities |
| SEC-2 | 未確認 | 手元に cargo-deny がない。Rust の直接依存は追加していない（Cargo.lock はパッチ版の更新と `base64 0.23.1` の追加のみ）。PR の CI で確かめる |
| SEC-3 | 合格 | Cloudscape 3 パッケージは Apache-2.0。`@resvg/resvg-js` は MPL-2.0 の devDependency で、配布物には入らない |
| SEC-4 | 合格 | コアの AWS 呼び出しは `get_log_events`・`describe_log_groups`・`describe_log_streams` だけ（`create_new` はファイルの作成で AWS ではない） |
| SEC-5 | 合格 | CSP は `default-src 'self'` のまま、追加は `font-src 'self' data:` だけ。外部の接続先なし |
| SEC-6 | 合格 | `errorText.test.ts`・`FetchErrorBanner.test.tsx` を含む vitest がすべて通過 |
| SEC-7 | 合格 | `ci.yml` に AWS の認証情報や AWS のアクションがない |

## 実機でしか確かめられない項目

次は Tauri の実機（WKWebView）での操作か、開発者本人の判断が必要で、このステージでは実行できなかった。後続の検証ステージは計画にない。

- NFR2 の 100 万行の性能測定（PERF-1〜PERF-5）
- NFR7 の最小ウィンドウ 1024×640 での崩れ
- NFR3 のマウスなしの一連の操作
- SM6 のコンソールと並べた最終判定
- README の手動確認チェックリストの各項目（カレンダーの幅、起動時の昨日〜今日、Dock の名前とアイコンの大きさ、未来日の無効、ステータス行が常に見える、など）

## 判定

ビルド・静的検査・自動テストはすべて成功した。測れる目標のうち実機確認が必要なものが `Unverified` のため、Build and Test の失敗条件（`Unverified` が残る）に当たる。原因はコードの欠陥ではなく、実機での確認が未実施であることで、コードの修正で解消できる候補はない。

## 利用者の判断

2026-10-11：利用者は「Accept failure」を選んだ。`Unverified` の 6 件（NFR1、NFR2、NFR3、NFR5、NFR7、SM6）は未確認のまま記録し、承認のゲートに進む。実機の確認は README の手動確認チェックリストと performance-test-instructions.md に従って手元で行う。目標そのものは緩めていない。
