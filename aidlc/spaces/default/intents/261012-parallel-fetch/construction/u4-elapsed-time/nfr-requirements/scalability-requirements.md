# Scalability Requirements — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR6.3 | 長い取得でも表記が崩れない：1 時間以上は h:mm:ss、時の桁は必要なだけ伸ばす（10 時間以上も表せる） | 表記の関数のテスト（3,600,000 ミリ秒 → 1:00:00、10 時間 → 10:00:00、100 時間 → 100:00:00） | FR4.5、BR4.7 |
| NFR6.4 | 取得を何度繰り返しても、経過の記録は 1 回分だけ（取得ごとに置き換え、増えていかない） | ライブラリのテストで、2 回目の取得を始めると 1 回目の最終の値が消え、記録が 1 つであること | BR4.1 |

同時に読む本数・ストリームの数・イベント数は、経過の扱いに影響しない（経過は取得 1 回に 1 つ）。

## Assumptions & Open Questions

None.
