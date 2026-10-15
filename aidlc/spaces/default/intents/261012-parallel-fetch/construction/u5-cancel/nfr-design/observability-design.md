# Observability Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

| 要件 | 設計 |
|------|------|
| NFR2.5 | 要約の 1 行の status に `JobStatus::Cancelled.as_str()` の "Cancelled" が入る。ほかの欄は今と同じ |
| NFR9.3 | messages.ts に新しいキーを英日で足し、文言の一覧のテストで両言語をそろえる |
| NFR9.4 | CancelNotice は role="status" の囲みの中の Alert（warning）。出たときに一度だけ読み上げられ、アイコンと文で伝わる。VoiceOver での確認は GUI の確認で一度行う |
| NFR10.1 | ConditionArea の取得のボタンは 1 つの要素のまま文言と状態だけを変える |

## Assumptions & Open Questions

None.
