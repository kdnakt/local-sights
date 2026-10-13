# Scalability Requirements — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

| ID | 要件 | 目標・測り方 | 出典 |
|----|------|--------------|------|
| NFR3.2 | 呼び出しの速さの上限 | GetLogEvents は再試行を含めて、どの 1 秒の区間 [t, t+1 秒) でも、東京 12 回・上限 10 回のリージョン 5 回・eu-west-3 15 回・表にない／指定なし 5 回以下。偽物で呼んだ時刻を記録して確かめる | NFR3、BR1.1〜BR1.3 |
| NFR3.3 | 8 本が同時に許可を求めたとき | 許可の時刻がすべて最小の間隔（東京なら 83,333,334 ナノ秒）以上あく | BR1.3 |
| NFR3.4 | SDK の中の再試行が数に入らないこと | GetLogEvents の呼び出しでは SDK の再試行の回数が 1 に上書きされている（設定の単体テスト）。DescribeLogGroups・DescribeLogStreams は上書きしない | NFR3、BR1.6 |
| NFR1.4 | ストリームの本数による伸び方 | 数十〜数百本（困りごとの場面）で、かかる時間は「本数 × 1 本あたりの呼び出し数 ÷ 速さの上限」程度になる（呼び出しの待ち時間ではなく速さの上限で決まる）。数千本でも動くが、速さの目安にはしない | NFR1、requirements §6 |
| NFR5.3 | 同時に読む本数 | 最大 8 本で固定。利用者は変えられない | BR1.7、requirements §6 |

## Assumptions & Open Questions

None.
