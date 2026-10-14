# Performance Design — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

設計の正本は `security-design.md` の 2 節。

| 要件 | 設計 |
|------|------|
| NFR4.2 | 止める信号が来たら、ReadStage は新しい読みを積まず、積んだ読みを落とす。Retrier は今も止まっていれば呼び出しの前に戻る |
| NFR4.3 | 読む段は `stop.stopped()` を先に置いた biased の select で `reads.next()` と、一覧の段は一覧の呼び出しと `stop.stopped()` を並べて待つ。再試行の待ち・許可待ちは今の `wait_or_stop` のまま。どの待ちでも信号が来たらすぐ戻る |
| NFR4.4 | 落とした読みのストリームは schedule の上で読み途中のまま残り、N に入る。そのページは一覧に足さない。全ページを足し終えて終わりの処理だけが残っていたストリームも N に入る（取れたログは欠けていないが、読み終えた印が付く前に止めたため）。打ち切った呼び出しは要約の calls に数えない |
| NFR4.5 | 止める信号が来る前に全部読み終えていれば、N = 0 で今の決め方になる |
| NFR5.5 | 一覧の部品（LogView・Timeline）には手を入れない |

## Assumptions & Open Questions

None.
