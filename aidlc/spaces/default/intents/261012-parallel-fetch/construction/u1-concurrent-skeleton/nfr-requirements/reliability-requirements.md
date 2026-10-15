# Reliability Requirements — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

ローカルのデスクトップアプリなので、可用性の SLO（稼働率）は持たない。U1 の信頼性は「同時に読んでも、取れたものを失わず、止まらず、今の動きを壊さない」ことである。

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR1.5 | 一部のストリームが失敗しても、ほかのストリームは読み続け、取れたログは残る | 偽物で 1 本が権限エラーのテスト | FR1.5、BR1.14 |
| NFR1.6 | 取得は必ず終わる：選ばれたストリームが 0 本でもすぐ終わり、読みかけが無くなれば終わる。永久に待たない | 0 本のテスト、全ストリームが失敗するテスト | BR1.9、functional-spec 2 節 |
| NFR4.1 | ウィンドウを閉じるとき（Close）は、新しい呼び出しを始めず、許可待ち・再試行の待ちの間ならすぐ止まる。出ている呼び出しの応答は待つ | 許可待ちの間に閉じるテスト、応答待ちの間に閉じるテスト | NFR4、BR1.5、BR1.16 |
| NFR1.7 | 再試行の待ちと回数は今と同じ（1・2・4・8・16 秒 ±20%、要求ごとに最大 5 回）。GetLogEvents の 5xx は再試行せず失敗として一覧する（SDK の再試行を止めた結果、受け入れた） | 既存の再試行のテストが通る | FR2.5、BR1.6 |
| NFR1.8 | あきらめの暫定の動き：使い切りが 3 本続いたら（終わった順）、未着手は呼ばずに失敗とし、読み途中は最後まで読む | 偽物で使い切りを続けるテスト | BR1.13 |
| NFR7.1 | 既存のテストはすべて通ったまま（JobStatus の置き場所の移動、止める信号の置き換え、fetch_stream の分割の後も） | `cargo test --workspace` と Vitest | org.md Testing Posture |
| NFR1.9 | 読み取りの中で panic が起きたら、取得 1 回のタスクごと終わり、アプリの「異常終了した取得の回復」が拾う（今と同じ） | 既存の回復の仕組みを変えない | functional-spec 2 節 |

## Assumptions & Open Questions

None.
