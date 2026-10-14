# Scalability Requirements — U5 中断（u5-cancel）

上流：`construction/u5-cancel/functional-design/`（functional-spec.md・rules.md BR5.1〜BR5.14、レビュー 2 回目の新しい指摘 2 件）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C3〜C7）、`codekb/local-sights/technology-stack.md`。質問票は `nfr-requirements-questions.md`（まとめの確認）。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR4.6 | ストリームが多くても（数百〜数千本）、N の数え方と止め方は変わらない。まだ始めていないストリームは呼ばずに数えるだけ | StreamSchedule の単体テストで 1,000 本の例を数える | FR5.4、BR5.4 |
| NFR6.14 | 中断を何度繰り返しても、注意・中断中の状態は取得 1 回分だけ持つ（次の取得で消え、増えていかない） | session.rs のテストで、中断 → 次の取得 → 中断の順に、注意が 1 つだけであること | BR5.8、BR5.9 |

## Assumptions & Open Questions

None.
