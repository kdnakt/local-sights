## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### 前回の指摘の状態

- R-08（型の輪 StreamSchedule と EventFetcher）：解消した。今のコードでは `fetcher.rs` が `retry::StreamEnding` を取り込んでいるが、改訂後は StreamEnding と終わり方の分け方が StreamSchedule に移り、EventFetcher は StreamFetchOutcome（endReason）を返すだけになった。`depends_on` と `dependents` は StreamSchedule → EventFetcher の片方向で対称、EventFetcher の依存先（CloudWatchLogsGateway・CallPacer・RetryPolicy）から StreamSchedule へ戻る道はない。CachePlan ↔ FetchCoordinator の輪も、`cache/plan.rs` の `use crate::coordinator::JobStatus` を FetchStatus に出すことで解ける。部品の依存に輪は見当たらない。
- R-09（終わりの状態を決めるのは StreamSchedule だけ、という主張）：実質的に解消した。ストリームを読む段は StreamSchedule、取得 1 回の確定は一覧の段と合わせて FetchCoordinator、という分担に改め、一覧の失敗（Failed）・一覧が途中まで（CompletedWithFailures）・一覧の前の中断（Cancelled または Aborted）を表にした。今の `decide_job_status` の分岐（Partial かつ 0 ページかつ選択 0 なら Failed、Partial なら CompletedWithFailures）とも合っている。ただし ADR-009 の見出しに古い言い回しが残っている（下の新しい指摘 2）。
- R-10（CallPacer の依存と、許可待ちの間の停止の道）：半分解消した。CallPacer → RetryPolicy の依存は宣言され、RetryPolicy 側の `dependents` とも対称になった。許可待ちを Retrier に渡す試行の先頭に置く、という置き場所も書かれた。ただし、その形が今の Retrier で実現できるかは未解決（下の新しい指摘 1）。
- R-11（存在しない `Partial` の値）：解消した。FetchStatus の説明は実際の値（Running／Completed／CompletedWithFailures／Failed／Aborted）に Cancelled を足す形になっており、JobStatus の値としての `Partial` は残っていない（`Partial` は一覧側の `StreamListingStatus` の値として別に存在する）。

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
| R-08 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > component StreamSchedule（depends_on／behaviour）と component EventFetcher（depends_on） | StreamSchedule は終わり方の分け方（StreamEnding）と streak_ending の対応を引き取るが、依存は FetchStatus だけである。今のコードでは streak_ending は EventFetcher 側の StreamFetchEnd（fetcher.rs）のメソッドで、fetcher.rs は retry.rs の StreamEnding を取り込んでいる。StreamEnding を StreamSchedule に移すと、EventFetcher が StreamSchedule の型を使い（EventFetcher → StreamSchedule）、StreamSchedule も EventFetcher の StreamFetchEnd／StreamFetchOutcome を使う（StreamSchedule → EventFetcher）ことになり、輪ができる。宣言された依存にはどちらの向きも載っていない。 | 輪を作らない型の置き場を決めて書く。例えば、ストリームの終わり方の型（StreamEnding など）を FetchStatus と同じ小さな共有の型に置く、または StreamSchedule は EventFetcher の StreamFetchEnd だけを受け取り StreamEnding を StreamSchedule の内部に閉じる。決めた向きを depends_on／dependents の両側に足し、ADR-007 の Consequences に書く。 | New |
| R-09 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > component StreamSchedule（behaviour：終わりの状態を決める唯一の部品）、component StreamPlanner、decisions.md > ADR-009 | 終わりの状態を StreamSchedule だけが決める、という決まりが一覧の段階で成り立たない。今の decide_job_status は StreamPlan の listing_status・listed_page_count・選ばれたストリームの有無から Failed と CompletedWithFailures を分けるが、StreamSchedule はこの情報を入力として持たない（StreamSlot は PlannedStream を値で指すだけ）。また「一覧の前の中断」は、まだ StreamSchedule が作られていない時点で起きる。StreamPlanner が中断を FetchCoordinator に返すだけなので、Cancelled を決めるのは実際には FetchCoordinator になる。 | 一覧の結果（一覧の状態、一覧のページ数、一覧の失敗）と一覧の前の中断を StreamSchedule の入力に含めるか、これらの場合は FetchCoordinator が決める、と持ち主の分担を書き直す。ADR-009 と ScheduleOutcome の属性（一覧の状態を表す属性、または一覧の前を表す印）と、StreamSchedule の依存（StreamPlanner の StreamPlan の型を使うなら宣言）をそろえる。 | New |
| R-10 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > component CallPacer（behaviour：中断・中止の信号が来たらすぐ戻る、depends_on: []）、component EventFetcher | CallPacer は待ちの間に止める信号を見る、と書かれているが依存は空である。止める信号の型は RetryPolicy の持ち物なので、CallPacer → RetryPolicy の型の依存がある。また今の Retrier.call は、呼び出しの手順が Result<T, ApiFailure> だけを返すため、許可待ちの最中の中断を手順の中から返す口がない。 | 許可待ちを止める信号と合わせて待つ責任を CallPacer が持つなら CallPacer → RetryPolicy を宣言する。EventFetcher が持つなら CallPacer の記述から信号の話を外す。どちらでも、許可待ちの中断を Retrier の外側で扱うか内側で扱うかを機能設計の宿題に明記する。 | New |
| R-11 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > component FetchStatus（behaviour） | 「Running／Completed／Failed／Partial／Aborted など」と書かれているが、コード（coordinator.rs）の値は Running／Completed／Failed／CompletedWithFailures／Aborted で、Partial はない。切り出す型の説明が実際の値と違う。 | 値の名前を CompletedWithFailures に直す（または今の値に Cancelled を足す、とだけ書く）。 | New |
| R-12 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > CallPacer behaviour（許可待ちは…試行の手順の先頭）、EventFetcher behaviour、RetryPolicy behaviour | 今の `Retrier::call`（`retry.rs`）は、渡された処理が `Result<T, ApiFailure>` を返す形で、「止めた」を返す道がない。許可待ちを試行の中に置いて「止めた」を伝えるには `Retrier::call` の形を変える必要があるが、RetryPolicy の説明は「止める信号に理由を足す」までで、この変更に触れていない。許可待ちの中で ApiFailure を返す案では、`decide` が再試行か使い切りとして扱ってしまう。許可待ちを Retrier の外に出す案では、再試行を数える（FR2.3）が崩れる。実装者が選ぶ形で結果が変わる。 | RetryPolicy の説明に、試行の処理が「成功・失敗・止めた」を返せるよう Retrier の呼び出しの形を変えることを書き、EventFetcher と CallPacer の説明からそこを参照する。細かい型は機能設計でよいが、「止めた」を使い切りとは別の結果として返すことは今決める。 | New |
| R-13 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/decisions.md > ADR-009 見出し | 見出しは「決めるのは StreamSchedule だけにする」のままだが、本文は FetchCoordinator が一覧の段の結果と合わせて取得 1 回の状態を確定するとしており、見出しと食い違う。R-09 の直しが見出しに届いていない。 | 見出しを本文に合わせる（例：「ストリームを読む段は StreamSchedule、取得 1 回の確定は FetchCoordinator が決める」）。 | New |
| R-14 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > StreamSchedule（StreamSlot.state、中断時の本数 N、FR5.9）、FetchCoordinator | EventFetcher が「止めた」で終えたストリームの StreamSlot の状態が決まっていない。状態は未着手・読み途中・成功・失敗の 4 つで「止めた」に当たるものがなく、N（読み途中＋未着手）に数えるか、FR5.9（最後のストリームが終わった後の中断は終わった扱い）の「すべて終わった」にどう効くかが読み取れない。また、取得 1 回の状態を確定する表（一覧の失敗・一覧が途中・一覧の前の中断・閉じる）は、今は純粋な関数 `decide_job_status` にあたるが、FetchCoordinator は「非同期のつなぎに専念」と書かれ、この表を純粋なロジックとしてテスト先行で書く置き場所と、その入力に必要な一覧の状態（FetchJob の listingStatus・listingFailure。今のコードにはある）が FetchJob の属性に載っていない。 | 「止めた」で終えたストリームは読み途中のまま残し N に数える、と StreamSchedule に書く。FR5.9 の判定は「未着手・読み途中がもう無い」と明記する。取得 1 回の状態の確定は `decide_job_status` の後継の純粋な関数として置き、入力の listingStatus・listingFailure を FetchJob の属性に加える。 | New |
| R-15 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/domain-design/components.md > FetchCheck / RecordingSink の depends_on と Component Summary | FetchCheck（`examples/fetch_check.rs`）と RecordingSink（`tests/support/recording_sink.rs`）は今、`coordinator::JobStatus` を取り込んでいる。説明には「JobStatus の場所が変わるのに合わせて直す」とあるのに、`depends_on`・FetchStatus の `dependents`・図には載っていない。同じ型を使う CachePlan・AppSession は載せているため、一覧の書き方がそろっていない。 | FetchCheck と RecordingSink の `depends_on` に FetchStatus を足し、FetchStatus の `dependents` と図・要約表を合わせる。再公開（FetchCoordinator が JobStatus を再エクスポートする）で済ませるなら、その旨を FetchStatus の説明に書く。 | New |

> R-01 Not re-checked this round

> R-02 Not re-checked this round

> R-03 Not re-checked this round

> R-04 Not re-checked this round

> R-05 Not re-checked this round

> R-06 Not re-checked this round

> R-07 Not re-checked this round

> R-08 Not re-checked this round

> R-09 Not re-checked this round

> R-10 Not re-checked this round

> R-11 Not re-checked this round

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（検証ツールは定義されていない） | 該当なし | 作者の YAML 点検（25 部品、依存の対称、参照の持ち主、輪なし）を、コードの `use` 文（`fetcher.rs`、`cache/plan.rs`、`session.rs`、`coordinator.rs`）と突き合わせた。輪は見つからなかった。 |

### Summary

前回の R-08〜R-11 は、R-10 の許可待ちの停止の道を除いてほぼ解消し、改訂が新しい輪や破れた参照を持ち込んだ形跡はなく、依存は輪なしである。残るのは Retrier の呼び出しの形の明記など、実装の迷いを減らすための Minor の直しで、承認の妨げにはならない。
