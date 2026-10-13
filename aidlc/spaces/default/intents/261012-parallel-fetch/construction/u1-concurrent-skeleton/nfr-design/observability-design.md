# Observability Design — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-requirements/observability-requirements.md`（NFR2.2〜NFR2.4、NFR8.6）、`security-requirements.md`（NFR8.3）。質問票の回答は `nfr-design-questions.md` の Q1。

監視の仕組み・アラート・ダッシュボード・外への送信は持たない（ローカルのアプリ、テレメトリを送らない）。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR2.2（取得 1 回の要約） | FetchCoordinator が取得 1 回ごとに数える入れ物（FetchStats）を作る。項目は streams（読んだ本数）、calls（GetLogEvents の試行の数）、throttled（Throttled の失敗の数）、elapsed_ms（読む段の始まりから終わりまで）、max_in_flight（読み途中の数の最大）。終わりに JobStatus と合わせて 1 行を組み立て、標準エラー出力へ出す。キャッシュから表示した取得では出さない |
| NFR2.3（確認用プログラム） | FetchCheck も同じ組み立ての関数で同じ 1 行を表示し、件数も表示する |
| NFR2.4（数え方） | calls は Retrier の試行ごと（許可が出て GetLogEvents を呼んだとき）に 1、throttled は試行の結果が Throttled のときに 1、max_in_flight は入れ物に入れたときの数の最大。入れ物は取得 1 回で捨てる |
| NFR8.6（出してよいもの） | 組み立ての関数は数値と JobStatus だけを受け取る（security-design） |

## 書式

```
local-sights: fetch summary streams=180 calls=212 throttled=3 elapsed_ms=18450 max_in_flight=8 status=Completed
```

項目の順と名前は固定する。値はすべて整数か JobStatus の語。

## Assumptions & Open Questions

None.
