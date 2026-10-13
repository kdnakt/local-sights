# Performance Design — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-requirements/`（performance・scalability・reliability・observability・security、tech-stack-decisions.md）、`construction/u1-concurrent-skeleton/functional-design/`、`inception/contract-design/contract-summary.md`。質問票の回答は `nfr-design-questions.md` の Q1。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR1.1（100 本が 10 秒以下） | 取得 1 回のタスクの中で、FuturesUnordered に「1 本のストリームの次の 1 ページ」を最大 8 つ並べる。呼び出しの待ち時間が重なり、全体の時間は速さの上限（東京 12 回／秒）で決まる（約 8.5 秒） |
| NFR1.2（最初の 1 件が早い） | 一覧の後すぐに 8 本を並べて始めるので、範囲にログがあるストリームまで順に待たずに最初のページが届く。届いたページはその場で保持先に混ぜ込み on_batch を呼ぶ（まとめない） |
| NFR1.3（同時の最大がちょうど 8） | 並べる数を 8 で止める。テストは 1 回 700 ミリ秒以上の偽物で最大を記録する |
| NFR5.1（100 万件の滑らかさ） | 保持先（EventTimeline・LogView）の混ぜ込みは今のまま。ロックはページごとに短く取る（今と同じ）。既存の速度測定テストで確かめる |
| NFR5.2（渡し待ち最大 8） | 1 本のストリームは前のページを渡し終えてから次の 1 ページを読む。別の通り道（チャネル）は持たない |
| NFR2.1（手元の比較） | 確認用プログラムで、main の版と U1 の版を同じ条件（`AWS_REGION=ap-northeast-1`、同じプロファイル・ロググループ・時間範囲）で 3 回ずつ動かし、`time` の実時間の中央値と件数を比べる（Q1）。手順は U1 の確認の手順書に書く |

## 速さの予算

| 部分 | 予算 |
|------|------|
| 許可の間隔 | 1,000,000,000 ÷ 上限 ナノ秒を切り上げ（東京 83,333,334 ナノ秒） |
| 1 ページの混ぜ込みと on_batch | 今と同じ（変えない） |
| 同時に出ている呼び出し | 最大 8 |

## Assumptions & Open Questions

None.
