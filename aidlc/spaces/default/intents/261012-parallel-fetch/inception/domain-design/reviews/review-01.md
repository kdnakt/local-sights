## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-12T14:41:23Z
**Iteration:** 1

この審査は助言型（ADVISORY）の 1 回きりで、指摘は承認の場で人間が判断する材料です。部品の分け方（CallPacer・StreamSchedule の純粋部品化、受け口を 1 列に集める形、理由つきの止める信号）は、現行コードの構造（直列の `for`、`&mut self` の `FetchSink`、`AbortHandle`/`AbortSignal`、`ExhaustionStreak`）と整合しており、実装者が部品の責務に迷う箇所は少ないと判断しました。Critical はなく、Major は 2 件です。承認前に次の 2 点（Major）をご検討ください。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > AwsCloudWatchLogsGateway の behaviour、CallPacer の behaviour、ADR-001 Consequences（decisions.md） | SDK の中の再試行を「止めるか数に入れるかは機能設計で決める」と先送りしているが、この選択は部品の依存関係そのものを変える。CallPacer は `CloudWatchLogsGateway` の trait の手前（EventFetcher 側）に置かれており、SDK の内部再試行はこの境界の内側で起きるため、上からは数えられない。「数に入れる」を選ぶと、AwsCloudWatchLogsGateway が CallPacer に依存する（または CallPacer をゲートウェイの中に入れる）必要があり、ADR-001 が退けた案 B（ゲートウェイを包む）と同じ構造になる。現行の `gateway/aws.rs` には再試行の設定がなく、SDK の既定の再試行が有効なので、スロットリング時は 1 回の `get_log_events` が裏で複数回 HTTP を発行し、NFR3 とアプリ側の再試行回数・使い切りの数え（FR3）の両方がずれる。 | 依存の形を左右する判断なので、ここで「SDK の既定の再試行は無効にし、再試行はアプリの Retrier に一本化する」（推奨）か「数えるためにゲートウェイが CallPacer を受け取る」のどちらかを決め、AwsCloudWatchLogsGateway の depends_on と ADR-001 の Consequences に反映する。機能設計に残すのは無効化の具体的な設定方法だけにする。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > 部品カタログの depends_on（CachePlan、AppSession、CallPacer、StreamSchedule、RetryPolicy）と Entity Ownership の References | 宣言された依存が実際の依存と食い違っており、「循環なし」の確認が宣言だけに頼っている。(1) CachePlan は `depends_on: []` だが、`cache/plan.rs` が `crate::coordinator::JobStatus` を取り込み（29 行目）、FetchCoordinator も CachePlan に依存するため、実際は相互依存である。今回「中断」の終わり方を足すと `JobStatus` と `decide_write` の両方が変わる。(2) AppSession は `FetchJob`/`JobStatus` を使い、CancelNoticeView は ScheduleOutcome を参照する（FetchSessionView も FetchJob を参照）が、depends_on は CachePlan だけである。(3) CallPacer（PacingState→FetchJob、→ConnectionTarget）、StreamSchedule（ScheduleOutcome→FetchJob）、RetryPolicy（StopSignal→FetchJob）は depends_on が `[]` なのに、FetchCoordinator が持つ FetchJob を参照しており、FetchCoordinator がこれら 3 つに依存しているため参照の向きが逆の輪になっている。 | 参照が型の依存か、`fetchJobId` という値だけの参照かを区別して書く。値だけなら「ID でのゆるい参照であり、型の依存ではない」と明記する。型の依存なら depends_on に足す。`JobStatus`（と終わりの状態）は FetchCoordinator に置いたままにするか、別の小さな共有型に出すかを決め、CachePlan↔FetchCoordinator と AppSession→FetchCoordinator の依存を一覧に載せる。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > RetryPolicy の behaviour（止める理由の優先） | 同じ段落が「一度送った理由は変えない（先に届いた理由が勝つ）」と「中断の後に閉じるが来たら『閉じる』に強める扱いは機能設計で決める」を並べていて、互いに矛盾する。「中断中…」の間にウィンドウを閉じる操作は現実に起こりうる（現行は `Destroyed` で安全網の中止をかける）。 | 優先の決まりを 1 つに決めて書く（推奨：「閉じる」は常に「中断」を上書きできる。逆はしない）。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > FetchCoordinator の entity FetchJob、AppSession、ScheduleOutcome | 取得 1 回の終わりの状態の持ち主が重複している。FetchJob.status、ScheduleOutcome.endState、FetchSessionView.phase が同じ事実を別々に持ち、読み終えた本数・失敗の本数も FetchJob と ScheduleOutcome の両方にある。現行の `FetchJob` が持つ `failed_streams`・`outcomes`・キャッシュ関連の項目は属性に載っていない。また現行の `JobStatus` には「中断（残す）」に当たる値がなく、`Aborted` は「捨てる」で `finish_fetch` が Idle に戻す動きになっている。 | 終わりの状態の唯一の持ち主を決める（推奨：StreamSchedule が決め、FetchJob.status に写す）。「中断（残す）」を新しい `JobStatus` の値にするかを明記し、`Aborted`（閉じる）との区別と AppSession の `finish_fetch` の扱いを書く。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > StreamSchedule と RetryPolicy の entity（RetryRule） | 連続使い切りの上限（現行は `RetryPolicy.max_consecutive_exhausted`、既定 3）と、終わり方の分類（`StreamEnding`／`streak_ending()`）の行き先が書かれていない。ADR-007 は数えの持ち主を移すが、上限の値と分類の型がどちらの部品にあるかが決まっておらず、RetryRule の属性（failureKind, delays, jitter, maxAttempts）には上限がない。 | 上限と終わり方の分類を StreamSchedule に移すか、設定は RetryPolicy に残して StreamSchedule が受け取るかを 1 行で決める。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > CallPacer の PacingState、FetchCoordinator の behaviour（リージョンの出どころ） | CallPacer が上限を引くリージョンの出どころが決まっていない。`FetchRequest.region()` は `None`（プロファイルの既定のリージョン）になりうるが、その解決は `AwsCloudWatchLogsGateway` の内側（`resolve_connection_target`）で行われ、trait の 3 操作からは取り出せない。アプリの画面からは常にリージョンが渡される（U2:BR2.8）ので通常は問題ないが、`None` の経路（確認用プログラム）では黙って 1 秒 5 回の表にないリージョン扱いになり、Walking Skeleton の実 AWS 確認で速さが半分に見える。 | `None` のときの扱い（フォールバックを使うこと、またはリージョンを必須にすること）を CallPacer の behaviour に 1 行で書く。 | New |
| R-07 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > この一覧の範囲（FetchCheck、RecordingSink、EventTimeline の欠落） | codekb の component-inventory が「今回：中」とした FetchCheck、RecordingSink、EventTimeline が一覧になく、更新の持ち主がいない。FetchCheck は team.md の Walking Skeleton 確認（実 AWS への取得）の道具で、`run_fetch`／`Retrier`／`AbortSignal::never` を使うため、取り消しの理由や CallPacer の配線が変わると直す必要がある。EventTimeline は NFR5（古い時刻のページが後から届いても 100 万件で滑らか）の性能を左右するが、LogView が「今と同じ」とされ、混ぜ込みの費用の確認先が ADR-003 の「速度測定テスト」だけになっている。 | FetchCheck と RecordingSink を「変更（追従）」として一覧に足し、EventTimeline は「変えないが NFR5 の確認対象」と範囲の節に明記する。あわせて、各ストリームから FetchCoordinator への受け渡しの通り道（有界か無制限か）を機能設計の宿題として書く。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（このステージに検証ツールの指定なし） | 実行せず | YAML 目録を手で再確認した。名前は重複なし、宣言された depends_on と dependents は対称、宣言された辺には循環なし。ただし、宣言されていない実際の辺（上の 2 件目の Major）を含めると CachePlan↔FetchCoordinator が相互依存になる。 |

### Summary

部品の分割と責務の置き方はコードの現状とよく合っており、READY と判断します。ただし、SDK の内部再試行の扱いは部品の依存関係を変えるので承認前に決めておくべきで（Major 1）、宣言された依存と実際の依存・参照の食い違いも整理しておくと、後続の機能設計と作業単位の分割が安定します（Major 2）。
