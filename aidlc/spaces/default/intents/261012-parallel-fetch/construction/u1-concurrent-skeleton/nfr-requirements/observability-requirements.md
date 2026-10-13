# Observability Requirements — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/functional-design/`（functional-spec.md・rules.md）、`inception/requirements-analysis/requirements.md`（NFR1〜NFR10）、`inception/contract-design/contract-summary.md`、`codekb/local-sights/`。質問票の回答は `nfr-requirements-questions.md` の Q1・Q2。

ローカルのアプリで、監視の仕組み・アラート・ダッシュボードは持たない（テレメトリは外に送らない）。見えるようにするのは、開発者が手元で確かめるための診断ログだけ。

| ID | 要件 | 中身 | 出典 |
|----|------|------|------|
| NFR2.2 | 取得 1 回の終わりの要約 | 取得 1 回の終わりに、標準エラー出力に 1 行を出す。項目は「読んだストリームの本数、GetLogEvents の呼び出しの数（再試行を含む）、制限を受けた回数、かかった時間（ミリ秒）、同時に読んだ本数の最大、終わりの状態（JobStatus）」。キャッシュから表示した取得では出さない | Q1 |
| NFR2.3 | 確認用プログラムの表示 | FetchCheck も同じ要約（同じ組み立ての関数）を表示する。手元の確認（NFR2.1）でかかった時間と件数を比べるのに使う | Q1、Q2 |
| NFR8.6 | 出してよいもの | 数値と終わりの状態だけ。名前・接続の情報・失敗の詳細は出さない（security-requirements NFR8.3） | NFR8 |
| NFR2.4 | 数え方 | 呼び出しの数は試行ごと（再試行を含む）に 1、制限を受けた回数は Throttled の失敗ごとに 1、同時の最大は読み途中のストリームの数の最大。数えるのは取得 1 回の中だけで、取得をまたいで残さない | BR1.4 |

## Assumptions & Open Questions

None.
