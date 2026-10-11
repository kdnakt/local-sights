# Integration Test Instructions — 画面を AWS マネジメントコンソールに寄せる

テスト戦略は Standard：主要な境界のテストを行う。この作業の境界は次の 3 つ。

| 境界 | 何を確かめるか | どのテストで |
|---|---|---|
| 画面 ↔ コア（Tauri のコマンドとイベント） | 画面が送る `updateInput`／`startFetch`／`selectTimeZone` と、コアが返すセッションの形（`startInput.text` など）が一致し、日付と時刻の 2 欄に分けて表示・結合できる | `src/App.test.tsx`（`src/test/` のモックで Tauri の `invoke`／`listen` を差し替える） |
| コアの入力 ↔ 取得の範囲 | スラッシュ書式の入力から取得の範囲が組み立てられ、起動時の初期値（昨日〜今日）でそのまま取得できる | `crates/local-sights-core/tests/u1_fetch_flow.rs`、`u3_fetch_flow.rs`、`session.rs` の Gate fix 16a 節 |
| コア ↔ AWS 呼び出しの境界（trait） | 取得・一覧・絞り込み・キャッシュの流れが変わっていない（AWS はスタブ） | `crates/local-sights-core/tests/u2_log_group_listing.rs`、`u5_filter_flow.rs`、`u6_cache_flow.rs` |

## 実行

```
cargo +stable test --workspace
npx vitest run
```

どちらも実際の AWS には接続しない（project.md の禁止事項）。AWS 呼び出しは trait の裏にあり、テストではスタブに差し替わる。

## 手元の実機で行う結合の確認

Tauri の WebView（WKWebView）とコアをつないだ動きは jsdom では確かめられない。README の手動確認チェックリストに従い、開発者本人の AWS 認証情報で次を行う。

1. `RUSTUP_TOOLCHAIN=stable npm run tauri dev` で起動し、開始が昨日 `00:00:00`、終了が今日 `23:59:59` で入っていること。
2. ロググループを選んで [取得] し、時刻列が `yyyy/mm/dd hh:mm:ss.SSS` で出ること。
3. タイムゾーンを切り替えると、両欄が同じ瞬間を指したまま書き換わること。
4. 取得中にステータス行が枠の下端に見え続けること。

## 期待するカバレッジ

数値の下限は設けない（team.md、スコープ bugfix）。境界ごとに上のテストが通り、既存のスイートが緑のままであること。
