# Logical Components — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`inception/domain-design/components.md`、`construction/u1-concurrent-skeleton/functional-design/`、`construction/u1-concurrent-skeleton/nfr-requirements/`。質問票の回答は `nfr-design-questions.md` の Q1。

ローカルのデスクトップアプリ 1 つの中の部品で、配置の単位（サーバー・コンテナ）はない。失敗の範囲は「取得 1 回」で、取得 1 回が失敗・中止してもアプリと保持先の今の表示の決まり（閉じるときは捨てる）は変わらない。

| 部品 | U1 で入れる NFR の仕組み | 失敗の範囲 | 共有するもの |
|------|---------------------------|------------|--------------|
| CallPacer（新規） | 速さの上限、枠の予約、止める信号での中断 | 取得 1 回（取得ごとに作る） | 取得 1 回の全ストリーム |
| FetchCoordinator | 同時 8 本の並べ方、渡し待ち 8、要約の数え（FetchStats） | 取得 1 回 | 保持先（LogView）、受け口（FetchSink） |
| EventFetcher | 次の 1 ページの読み取り（PageStep）、試行の前の許可待ち | 1 本のストリーム | CallPacer |
| RetryPolicy（Retrier・StopHandle・StopSignal） | `call` の試行の前の待ち処理、`call_unpaced`、理由つきの止める信号 | 1 回の要求 | 止める信号は取得 1 回の全部品 |
| StreamSchedule（新規） | 次に読むもの、ストリームごとの状態、Skipped | 取得 1 回 | — |
| FetchStatus（新規） | JobStatus の型 | — | CachePlan、AppSession、TauriApp、FetchCheck、テスト |
| AwsCloudWatchLogsGateway | GetLogEvents だけ SDK の再試行を 1 回に | 1 回の呼び出し | SDK のクライアント（接続先ごとに再利用。最初の同時呼び出しで重複して作りうる点は今の作りのままで、機能設計のレビュー 2 の指摘としてコード生成で扱う） |
| FakeGateway（テスト） | 呼び出しの時間、時刻の記録、同時の最大の記録 | テストだけ | — |
| FetchCheck（確認用） | リージョンの引数、要約の 1 行と件数の表示 | 手元の確認だけ | — |

## Assumptions & Open Questions

None.
