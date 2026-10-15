# Monitoring Design — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-design/observability-design.md`、`security-design.md`。質問票のまとめの確認。

ローカルのアプリで、監視の仕組み・アラート・ダッシュボードは持たず、外へは何も送らない（project.md Forbidden：テレメトリやクラッシュレポートを外部に送らない）。見えるのは、取得 1 回ごとの要約の 1 行（標準エラー出力）と、手元の確認用プログラムの表示だけである。

## Metrics & KPIs

| Metric | Source | Threshold | Why it matters |
|--------|--------|-----------|----------------|
| elapsed_ms（読む段の時間） | 要約の 1 行 | 合否は手元の比較で決める：同じ条件で 3 回ずつ、U1 の版の中央値が main の版より短く、件数が同じ（NFR2.1） | U1 のねらい（速くなること）を確かめる |
| calls（GetLogEvents の試行の数） | 要約の 1 行 | なし（呼び出しの量の目安として見る） | 速さの上限が効いているかを読み取る |
| throttled（Throttled の失敗の数） | 要約の 1 行 | なし。0 が続くのが望ましい | 他の利用者の枠を奪っていないか（NFR3.1）の目安 |
| max_in_flight（読み途中の数の最大） | 要約の 1 行 | 8 を超えない | 同時 8 本の上限（NFR5.3） |
| streams・status | 要約の 1 行 | なし | 取得 1 回の結果を読み取る |

## Alerts

| Alert | Condition | Severity | Routes to |
|-------|-----------|----------|-----------|
| なし | — | — | 外への送り先を持たない。失敗は今のまま画面の失敗の一覧に出す |

## SLIs / SLOs

| SLI | SLO target | Measurement window |
|-----|------------|--------------------|
| なし（サービスを提供しないため SLO を置かない） | — | 速さの確かめは NFR2.1 の手元の比較で、U1 の確認のときに 1 度行う |

## Logs & Tracing

| 項目 | 決まり |
|------|--------|
| 出し先 | 標準エラー出力だけ。ファイルに書かず、外にも送らない（NFR8.4） |
| 書式 | `local-sights: fetch summary streams= calls= throttled= elapsed_ms= max_in_flight= status=`（observability-design） |
| 出してよいもの | 数値と JobStatus の語だけ。ロググループ名・ストリーム名・ログの中身・秘密の値は出さない（NFR8.3、NFR8.6） |
| 見方 | 開発者が手元でアプリを端末から起動したときと、確認用プログラム（FetchCheck）の表示 |
| トレース・ダッシュボード | なし |

## Assumptions & Open Questions

None.
