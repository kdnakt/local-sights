# Observability Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

U4 は運用のための計測を足さない（デスクトップアプリで、外へ送らない）。

| 要件 | 設計 |
|------|------|
| NFR2.3 | 状態行の経過が利用者にとっての計測。取得中は進み、終わると残る |
| NFR2.4 | U1 の要約の 1 行（`elapsed_ms=`）は変えない。これは FetchCoordinator の中で測った値で、状態行の経過（`begin_fetch` から）とは数十ミリ秒ほど違いうる |
| NFR9.1 | `messages.ts` に `status.elapsed`（英「Elapsed {time}」、日「経過 {time}」）。文言の一覧のテストが両言語のキーをそろえる |
| NFR9.2 | `ElapsedTime` は running の間 `aria-hidden="true"`。終わりの SessionView を受け取った描き直しで、StatusLine の取得中の表示が件数などの表示に置き換わるのと同時に属性が外れ、`role="status"` の領域の変化として最終の経過も伝わる。VoiceOver での確認は GUI の確認で一度行う |

## Assumptions & Open Questions

None.
