# Scalability Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

| 要件 | 設計 |
|------|------|
| NFR4.6 | N は schedule が持つ数（読み途中の数とまだ始めていない数）から出し、まだ始めていないストリームは呼ばずに数えるだけ。本数に比例する処理は増えない |
| NFR6.14 | AppSession の cancel_notice は Option で 1 つだけ。取得を始めるたびに None に戻す |

## Assumptions & Open Questions

None.
