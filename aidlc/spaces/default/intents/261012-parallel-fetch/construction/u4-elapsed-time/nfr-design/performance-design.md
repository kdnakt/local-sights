# Performance Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

設計の正本は `security-design.md` の 2 節。ここでは速さに関わる点だけを書く。

| 要件 | 設計 |
|------|------|
| NFR5.3 | 毎秒の描き直しは `ElapsedTime` 部品の中の状態（`useElapsed`）だけで起こす。StatusLine・App・LogTable の状態は毎秒変わらないので描き直されない。描き直しの予約は表示の秒が変わる時刻に 1 回だけ（`setTimeout`、`setInterval` は使わない） |
| NFR5.4 | `fetch-progress` の項目と `session-changed` を送る契機は変えない。経過はもともと送られる SessionView に乗るだけ |
| NFR2.2 | 最終の値はライブラリの戻らない時計の差（ミリ秒）で、取得の流れの処理を待たせない（記録は数十ナノ秒の操作） |

予算：取得中の追加の描画は、経過の span 1 つを 1 秒に 1 回。ログの表の描画回数は変わらない。

## Assumptions & Open Questions

None.
