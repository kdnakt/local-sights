# Integration Test Instructions — 統合テストの手順（全単位）

Test Strategy は Standard：主要な境界の結合テストを、単位ごとの単体テストに加えて実行する。上流は U1〜U7 の `unit-test-instructions.md`・`code-summary.md`。

## 枠組みと設定

| 側 | 枠組み | 境界の作り方 |
|----|--------|--------------|
| ライブラリ（Rust） | `cargo test` の結合テスト `crates/local-sights-core/tests/*.rs` | AWS は trait `CloudWatchLogsGateway` の偽物 `tests/support/fake_gateway.rs`（決めた応答を順に返し、受け取った引数と呼ばれた API を記録する）。ファイルは `tempfile` の一時フォルダ。再試行の待ちは `#[tokio::test(start_paused = true)]` で実時間を待たない |
| 画面（TypeScript） | Vitest + React Testing Library（jsdom）。`src/App.test.tsx` が画面全体のつなぎ | `src/api.ts`（Tauri のコマンドとイベント）を `vi.mock` で差し替える。Tauri は起動しない |
| Rust と画面のつなぎ（`src-tauri`） | 自動テストなし | `cargo build -p local-sights` で型が合うことを確かめ、動きは手元の確認（下の「手元で確かめる境界」） |

どのテストも実際の AWS に接続せず、認証情報も要らない（project.md Forbidden、team.md Testing Posture）。

## 実行方法

```bash
# ライブラリ側の結合テスト（5 ファイル、全単位）
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --tests

# 単位ごとに 1 ファイルずつ
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u1_fetch_flow
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u2_log_group_listing
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u3_fetch_flow
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u5_filter_flow
CARGO_INCREMENTAL=0 cargo test -p local-sights-core --test u6_cache_flow

# 画面側の全体のつなぎ
npx vitest run src/App.test.tsx

# すべて（単体を含む）
CARGO_INCREMENTAL=0 cargo test -p local-sights-core
npx vitest run
```

## 単位をまたぐ境界と、それを確かめるテスト

| 境界 | 単位 | テスト | 観点 |
|------|------|--------|------|
| 取得の流れ：FetchCoordinator → EventFetcher → Gateway → EventTimeline | U1・U3 | `tests/u1_fetch_flow.rs`（11 件）、`tests/u3_fetch_flow.rs`（14 件） | ページ終端（渡したトークンと同じトークン）、開始・終了時刻を必ず渡す、ストリームの選定（1 時間の余裕、時刻なしは含める）、スロットリングの再試行（1 秒・2 秒の待ち）、一部失敗しても取得できた分を残す、中断 |
| 接続の選択 → ロググループ一覧：AppSession → LogGroupBrowser → Gateway | U2 | `tests/u2_log_group_listing.rs`（9 件） | DescribeLogGroups のページング、接続を変えたら取り直す、エラーの分類、認証情報の値が結果に出ない |
| 取得 → 絞り込み：EventTimeline → FilterEngine → LogView | U3・U5 | `tests/u5_filter_flow.rs`（4 件） | 取り直しても絞り込み文字列を引き継ぐ（FR6.4）、絞り込み後の件数 / 全件数 |
| 取得 ⇄ キャッシュ：FetchCoordinator → LogCache | U3・U6 | `tests/u6_cache_flow.rs`（3 件）、`coordinator.rs` の `a_disabled_cache_reads_and_writes_nothing_and_fetches_as_u3` | Hit なら API を呼ばない（FR7.4）、失敗・中断した取得は記録しない（FR7.9）、無効なら書かない（FR7.7） |
| 時間範囲とタイムゾーン → 取得条件：TimeRangeModel → FetchRequest → AppSession | U1・U4 | `session.rs`・`request.rs` の単体テスト（結合テストのファイルはない） | 切り替えても同じ瞬間（FR3.3）、終了は秒の終わりまで（FR3.5）、夏時間（FR3.6） |
| 画面全体：ConnectionBar・LogGroupPane・FetchForm・LogTable・StatusLine・ダイアログ | U2〜U7 | `src/App.test.tsx` | 取得中は条件を変えられず [Fetch] を押せない（FR4.8）、接続を変えるときの確認、設定ダイアログ、閉じる確認、英日の切り替え |

## 手元で確かめる境界（自動テストがないもの）

`src-tauri`（Tauri のコマンド・イベント・メニュー）は自動テストを置かない方針なので、macOS で `npm run tauri dev` を起動して確かめる。README の確認項目と同じ。

1. プロファイルとリージョンを選ぶとロググループ一覧が取り直される（FR1.4）。
2. ロググループを選び、開始・終了日時を入れて [Fetch] を押すと、ステータス行に取得中が出て、一覧に時刻順で行が増えていく（FR4.7・FR4.8）。
3. 取得中にウィンドウを閉じようとすると確認が出る。Cmd+Q でも出る（FR4.9。Dock からの終了は確認なしの既知の制約）。
4. 絞り込みの文字列を入れると「絞り込み後 / 全件」が出る（FR6.3）。
5. 設定ダイアログでキャッシュを有効にし、同じ条件で 2 回目の [Fetch] をすると API を呼ばずに表示される（FR7.4。`--example fetch_check` ではなく GUI で確かめる）。

## 期待する範囲

- 数値のカバレッジ下限は設けない（team.md）。その代わり、上の表の境界ごとに少なくとも 1 件の結合テストがあり、ライブラリ側の公開関数には単体テストがあること。
- 既存のテストはすべて通った状態を保つ（Testing Contract の scope floor）。

## テストデータ

- ログのイベント・ストリーム・ロググループはテストの中で組み立てる。ファイルには置かない。
- 認証情報に見える値は、テストの中で組み立てたダミー（`AKIA` + 英大文字と数字 16 文字など）だけを使う。
- 100 万件のデータはテストの中で生成する（速さの計測は `performance-test-instructions.md`）。
