# Unit Test Instructions — U4 時間範囲とタイムゾーン（u4-time-range）

Test Strategy は Standard（部品ごとに 5〜8 件、主要な境界の結合テスト）。数値のカバレッジ下限は設けないが、ライブラリ側の公開関数には必ずテストを書く（team.md の Testing Posture）。

## テストの枠組みと設定

U1〜U3 と同じ。ライブラリ側は標準の `cargo test`（単体テストは各モジュールの `#[cfg(test)] mod tests`）、画面側は Vitest + React Testing Library（`vitest.config.ts`、環境は jsdom）。`src-tauri` には自動テストを置かず、`cargo build -p local-sights` で組み立てを確かめる。

## この単位のテストの実行方法

前提：リポジトリのルートで `npm ci` を済ませておく。どのコマンドも実際の AWS には接続せず、OS のタイムゾーンの設定にも依存しない。

ライブラリ側（U4 のモジュールだけ）：

```bash
cargo test -p local-sights-core --lib -- time_range:: time_zone:: date_input:: request:: session::
```

画面側（U4 のテストファイルだけ）：

```bash
npx vitest run src/components/TimeZoneToggle.test.tsx src/components/LogTable.test.tsx src/components/FetchForm.test.tsx src/App.test.tsx src/i18n/messages.test.ts
```

テスト先行の手順（plan の Step 3）では、`time_range::`・`date_input::`・`request::`・`session::` のテストを書いたら上のライブラリ側のコマンドを実行して失敗の出力を記録してから実装に進む。

## 部品ごとのテストの目安

| 部品 | テストの場所 | 件数の目安 | 主な観点 |
|------|--------------|------------|----------|
| TimeRangeModel（解釈と文字列化） | `time_range.rs` | 7〜8 | BR1.2、BR3.1、夏時間（存在しない・2 回現れる）、表せない瞬間 |
| TimeRangeModel（入力欄） | `date_input.rs` | 7〜8 | BR1.3〜BR1.5、R-02、R-05、R-06、往復で瞬間が保たれる |
| TimeRangeModel（OS のタイムゾーン） | `time_zone.rs` | 3〜4 | BR1.1、BR3.4、知らない名前は UTC |
| AppSession（検証） | `request.rs` | 4〜5 | BR1.6、BR2.1、BR2.2 |
| AppSession（つなぎ） | `session.rs` | 6〜7 | BR1.4、BR1.5、BR2.1（確認待ち）、R-08 の displayTime |
| DesktopUi | `src/components/*.test.tsx`、`src/App.test.tsx`、`src/i18n/messages.test.ts` | 合計 12 前後 | BR2.2 の文言、BR3.1〜BR3.3、切替で取り寄せ直す |

## モックとスタブ

- ローカルのタイムゾーンは `TimeZoneContext` を決めた名前（`America/New_York`、`Asia/Tokyo`、`UTC`）で作って渡す。環境変数 `TZ` や OS の設定は変えない。
- 画面側のテストでは `src/api.ts` を `vi.mock` で差し替え、Tauri を起動しない。`get_rows` の偽物は displayTime を付けた行を返す（画面は変換をしないため、受け取った文字列をそのまま出すことを確かめる）。

## テストデータ

- 夏時間の境界：America/New_York の 2024-03-10 02:00〜03:00（存在しない）と 2024-11-03 01:00〜02:00（2 回現れる）。
- 受け入れ条件：Asia/Tokyo（UTC+9）で `2024-03-01 10:00:00` ⇄ UTC の `2024-03-01 01:00:00`。
- 4 桁の年に表せない瞬間：UTC の `9999-12-31 23:00:00` を Asia/Tokyo に切り替える。
