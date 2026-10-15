# Security Design — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/nfr-requirements/security-requirements.md`（NFR3.5・NFR3.6・NFR6.1・NFR7.1・NFR7.2・NFR8.1・NFR8.7・NFR8.8、レビュー 1 回目の R-01〜R-04）、`tech-stack-decisions.md`、`construction/u2-call-pacing/functional-design/`（rules.md BR2.1〜BR2.8）、`inception/contract-design/contract-summary.md`（C1）。まとめの確認は `nfr-design-questions.md`。

この文書の「上限」は、U1 の ceilingPerSecond（AWS の GetLogEvents の上限の半分。東京 12 回、10 回のリージョンは 5 回、表にない・指定なしは 5 回）を指す（NFR 要件のレビュー R-02）。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR3.5（制限で引き下がる） | EventFetcher の `counted_call` が、GetLogEvents の試行が Throttled で終わるたびに `CallPacer::report_throttled()` を呼ぶ。CallPacer は U1 のロックの中で、たまった戻しを当てはめてから、下げてよい時刻を過ぎていれば今の速さを半分にする（下限 1）。下げた後に予約する枠の間隔は、予約したときの速さの間隔以上。下げたときに予約済みの枠は動かさない（NFR 要件のレビュー R-01） |
| NFR3.6（戻しても上限を超えない） | 戻しは枠の予約と制限の知らせのときに、5 秒ごとに 1 ずつ、上限で止める。どの 1 秒の区間でも許可は上限以下のまま（U1 の NFR3.1） |
| NFR8.1（新しい API を呼ばない） | CloudWatchLogsGateway の trait は変えない。DescribeLogStreams の制限は CallPacer に知らせない（`call_unpaced` のまま） |
| NFR8.7（要約の欄は整数だけ） | U1 の要約（NFR8.3）に `min_rate` の欄を足す。FetchCoordinator が取得の終わりに `CallPacer::lowest_rate_per_second()` を読んで FetchStats に渡す。CallPacer を作る前に終わった取得は `RegionCallLimit::for_region(..).ceiling_per_second()`。組み立ての関数は数値と JobStatus だけを受け取る |
| NFR8.8（画面にも外にも知らせない、出典 FR2.6） | JobStatus・FetchSink の知らせ・Tauri の命令の形を変えない。出力は標準エラー出力の要約の 1 行だけ |
| NFR6.1（実際の AWS に接続しない） | テストは止めた時計と FakeGateway（台本で Throttled を返す） |
| NFR7.1・NFR7.2（既存のテストとカバレッジ） | U1 の要約の書式のテストを U2 の欄の順に直す。それ以外の既存のテストは変えずに通す。行カバレッジ 80% を CI で確かめる |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 制限を受けても同じ速さで呼び続け、同じアカウントの他の利用者を待たせる | 半分に下げる（NFR3.5、BR2.2） |
| 8 本の制限や、予約済みの古い速さの呼び出しの制限がまとめて届き、新しい速さを試す前に下限まで落ちる | 下げてよい時刻 = max(下げた時刻 + 1 秒, そのときの nextPermitAt) まで重ねて下げない（BR2.3） |
| 戻しすぎて上限を超える | 上限で止める（NFR3.6） |
| 速さを下げた状態で閉じても、長い許可待ちで止まらない（下限 1 回／秒で 8 本が待つと約 8 秒） | 許可待ちは U1 のまま止める信号と `biased` で並べて待つので、閉じるとすぐ Stopped を返す。下限まで下げた状態で閉じるテストで確かめる（U1 の NFR4.1 の引き継ぎ、NFR 要件のレビュー R-03） |
| 要約の 1 行に秘密や接続の情報が混ざる | 足す欄は整数だけ（NFR8.7） |

## 確かめること（引き継ぎを含む）

| 確かめること | 由来 |
|--------------|------|
| 半分に下げる、まとめて届いた知らせ、予約済みの枠の後から届いた知らせ（速さ 3 で枠が 2.7 秒先まで予約されているとき、1 に下げる）、戻す、端数の持ち越し、戻しの数え直し、知らせの前にたまった戻し | 機能設計 4 章（レビュー 2 回目の R-07 の言い直しを含む） |
| 下げた後に予約する枠の間隔は予約したときの速さの間隔以上、予約済みの枠は動かない | NFR3.5、BR2.5 |
| 制限が続くと間隔が広がり、止むと上限の間隔まで戻る。どの 1 秒も上限以下 | NFR3.5、NFR3.6 |
| 下限まで下げた状態で閉じると、許可待ちがすぐ止まり、新しい呼び出しを始めない | U1 の NFR4.1 |
| 再試行の待ちと回数は今と同じ（1・2・4・8・16 秒、最大 5 回） | FR2.5、U1 の NFR1.7 |
| 制限なしなら速さは上限のまま変わらず、U1 の同時取得のテストがそのまま通る | U1 の NFR1.1・NFR1.3 |
| 要約の欄の順（streams・calls・throttled・elapsed_ms・max_in_flight・min_rate・status）と、CallPacer の前に終わった取得の `min_rate` | NFR8.7、BR2.7 |

## Assumptions & Open Questions

None.
