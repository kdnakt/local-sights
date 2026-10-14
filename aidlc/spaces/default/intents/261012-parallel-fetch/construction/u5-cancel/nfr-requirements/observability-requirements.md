# Observability Requirements — U5 中断（u5-cancel）

上流：`construction/u5-cancel/functional-design/`（functional-spec.md・rules.md BR5.1〜BR5.14、レビュー 2 回目の新しい指摘 2 件）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C3〜C7）、`codekb/local-sights/technology-stack.md`。質問票は `nfr-requirements-questions.md`（まとめの確認）。

| ID | 要件 | 確かめ方 | 出典 |
|----|------|--------------|------|
| NFR2.5 | 取得 1 回の要約の 1 行の status に Cancelled が入る。ほかの欄は今と同じ | 要約の書式のテスト | C3、BR5.14 |
| NFR9.3 | 新しい文言（中断、中断中…、中断、注意 2 種）は英語と日本語を用意し、OS の言語で出し分ける | 文言の一覧のテスト（両言語のキーがそろう）、画面のテスト | NFR9、BR5.13 |
| NFR9.4 | 中断の注意は色だけでなくアイコンと文で伝え、出たときに一度だけ読み上げる（role=status の囲み） | 画面のテストで role と文、閉じるボタンがないこと。GUI の確認で VoiceOver を一度使う | FR5.11、AC4.1.6 |
| NFR10.1 | ［中断］は［取得］と同じ位置・同じタブ順で、キーボードだけで押せる。押した後もフォーカスはボタンに残る | 画面のテストで、同じボタンの要素が文言だけ変わり、フォーカスが動かない | NFR10、AC3.1.5、AC3.1.8 |

## Assumptions & Open Questions

None.
