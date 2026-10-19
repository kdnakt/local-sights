# コードの品質の見立て（Local Sights）

走査したコミット：`2760b13b8bc5b6daba36a6c283b3958c974a5a2d`（2026-10-17）。テスト・ビルド・lint はこの走査では実行していない（`target/` と `node_modules/` が無く、AWS にもつながない）。件数は `grep` と `wc -l` で数えた。CI の結果は確かめていない。

## テストとカバレッジ

| 対象 | テストの場所 | 件数（関数） | 実行と計測 |
|------|--------------|--------------|------------|
| 中核の単体テスト | 各モジュールの `#[cfg(test)]` | 457（`session.rs` 124、`retry.rs` 20、`time_range.rs` 20、`sso_login.rs` 21 ほか） | CI の Linux で `cargo llvm-cov -p local-sights-core --fail-under-lines 80`、macOS と Windows で `cargo test -p local-sights-core` |
| 中核の流れの試験 | `crates/local-sights-core/tests/`（u1_fetch_flow、u1_concurrent_fetch、u2_call_pacing、u2_log_group_listing、u3_fetch_flow、u3_give_up_rule、u5_cancel、u5_filter_flow、u6_cache_flow） | 78 | 同上（偽ゲートウェイ、tokio の止めた時計） |
| 確認用プログラムの引数 | `examples/fetch_check.rs` | 3 | `cargo test -p local-sights-core --examples`（AWS にはつながない） |
| アプリ | `src-tauri/src/lib.rs`、`src-tauri/src/sso_cli/mod.rs` | 31 | 3 つの OS で `cargo test -p local-sights`（偽の CLI：Unix はシェルスクリプト、Windows はバッチと PowerShell）。行カバレッジは測っていない |
| 画面とスクリプト | `src/**/*.test.{ts,tsx}`、`scripts/release/*.test.ts` | 346 | Linux で `vitest run --coverage`、`vitest.config.ts` の `thresholds.lines: 80`（`src/**` が対象、テストと `src/test/` は除く） |
| 速さ | `log_view.rs`（10 万件 10 秒、100 万件 100 秒、絞り込み中の行と位置が 10 ms、1 ページの追加が 1 秒）、`timeline.rs`（100 万件の行と位置が 10 ms、重なるページの混ぜ込みが 1 秒） | 5（`#[ignore]`） | 手動：`cargo test -p local-sights-core --release --lib -- --ignored` |

- テストの書き方は team.md の Testing Posture（純粋なロジックはテスト先行、外部とつながる層は実装後）に合っている。判断は純粋な関数に寄せ、試験は偽物に差し替えている。
- 一覧の速さ（画面側）は `src/dev/perfRows.ts` の道具で手で測る（`README.md`「Performance check of the log list」）。

## lint・整形・型・依存の検査

| 検査 | 道具と設定 | CI での扱い |
|------|------------|-------------|
| Rust の整形 | `rustfmt`（既定の設定） | `cargo fmt --all --check`、通らなければ失敗 |
| Rust の lint | `clippy`（中核は `--all-targets`、ワークスペース全体は 3 OS） | 警告はエラーにしない（`RUSTFLAGS=""`）。deny レベルは失敗 |
| 画面の型 | `tsc --noEmit`（`tsconfig.json`） | 失敗で止まる |
| 画面の lint | ESLint（`eslint.config.js`：recommended、typescript-eslint、react-hooks、react-refresh） | 失敗で止まる |
| 画面の整形 | Prettier（`.prettierrc.json`、`.prettierignore`） | `prettier --check .` |
| 依存 | cargo-deny（`deny.toml`）、`npm audit --audit-level=high` | 失敗で止まる |
| 改行 | `.gitattributes` で LF に固定 | Windows でも同じ整形結果 |
| 版のそろい | `npm run check:version`（`scripts/release/check-version.mjs`） | frontend のジョブ |

## 3 段階の見立て

| 観点 | 見立て | 根拠 |
|------|--------|------|
| 名前の分かりやすさ | よい | 型と関数の名前が業務の言葉（`FetchJob`、`EventTimeline`、`ScanTicket`、`decide_job_status`）。説明に業務ルールの番号が残る |
| 関数の大きさ | ふつう | 多くの関数は短いが、`session.rs` の `view()` や `coordinator.rs` の `run_fetch`、`src-tauri/src/lib.rs` の `run()` は長い |
| エラー処理 | よい | テスト以外で `unwrap()`／`expect()` を使わない。失敗は `Result` と文言キー。毒されたロックは回復。境界のエラーは安全な詳細だけ |
| テストの有無 | よい | 中核と画面の行カバレッジ 80% が CI の止める条件。速さのテストもある。`src-tauri` は計測外 |
| 重複 | ふつう | `TimelineStore`／`HeldEvents` が `Mutex<EventTimeline>` と `Mutex<LogView>` の両方に実装されている。画面の一覧が仮想スクロール版と Cloudscape 版の 2 つある |
| 使われないコード | よい | `TODO`／`FIXME`／`HACK` は 0。抑止はテスト内の `#[allow(clippy::await_holding_lock)]` 1 件。`LogTableCloudscape` と `perfRows` は開発用として明示 |

## 技術的な負債と気になる点

1. **`AppSession` が大きい**：`session.rs` は 5,428 行（本体約 2,270 行）。intent ごとに責務が足されてきた（説明に「Since U2 … Since 261012-parallel-fetch U5」）。IB-11 でクエリの状態を足すと、さらに大きくなる。
2. **`src-tauri/src/lib.rs` が大きい**：1,764 行に 26 コマンド、走査、取得、SSO、閉じる処理、メニューが同居する。行カバレッジの計測外。
3. **絞り込みの判定の確保**：`FilterCondition::matches` は 1 件ごとに `message.to_lowercase()` で新しい文字列を作る（`filter.rs`）。U5 の速さの基準は満たしている（テストあり）が、クエリで条件が増えると効いてくる。
4. **取得は毎回まるごと**：`run_fetch` は最初に保持中を捨てる。タイムラインは取得済みの範囲を記録していない。キャッシュの照会は 1 つの記録範囲が要求全体を含むときだけ当たる。
5. **単一ロググループの前提**：`LogEvent`、並びキー、`RowKey`、`FetchRequest`、`CacheKey`、`AppSession.selected_log_group` がすべて 1 ロググループを前提にしている。
6. **README の記述の古さ**：冒頭の「streams are read one after another」は、同時取得（最大 8 本）になった今と食い違う。
7. **`coordinator.rs` と `log_view.rs` の相互参照**：同じクレート内なので動作に問題は無いが、層の向きが分かりにくい。

## ドキュメント

- `README.md`：目的、レイアウト、前提、起動、SSO、骨組みの手動確認、一覧の速さの確認、手元から AWS に対する確認、アイコン、テストと検査、リリース、秘密情報。冒頭の取得の説明だけ古い。
- `docs/install.md`：macOS と Windows の節を分け、入れ方・初回起動の警告（Gatekeeper、SmartScreen）・AWS CLI v2・更新・戻し方・アンインストールを書く。
- `docs/releasing.md`：版、手順、失敗時、同じタグでの再実行。
- コード内の説明：Rust は `#![warn(missing_docs)]`。画面の部品とフックにも冒頭の説明がある。
- クエリの対応範囲の一覧（IB-11 の B6）は README にまだ無い。
