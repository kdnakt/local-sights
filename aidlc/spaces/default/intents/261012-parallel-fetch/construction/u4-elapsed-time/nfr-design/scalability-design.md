# Scalability Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

| 要件 | 設計 |
|------|------|
| NFR6.3 | `formatElapsed` は時を桁埋めせず整数のまま出すので、10 時間・100 時間でも崩れない。ミリ秒は Rust で `u64`、画面で `number`（2^53 ミリ秒は 28 万年を超えるので足りる） |
| NFR6.4 | AppSession の記録は `Option<ElapsedRecord>` の 1 つだけで、取得を始めるたびに置き換える。画面の起点（Anchor）も 1 つだけ |

ストリームの本数・イベント数・同時に読む本数は、経過の設計に関わらない。

## Assumptions & Open Questions

None.
