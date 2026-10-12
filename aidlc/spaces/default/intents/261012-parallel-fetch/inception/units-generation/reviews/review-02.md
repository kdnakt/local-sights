## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### 前回の指摘の状態

- R-01（許可待ちの持ち主の分裂）：解消済み。FR2.3（再試行の前も含むすべての試行の前の許可待ち）と、許可待ちの最中に止める信号が来たら「止めた」を返す道（ドメイン設計 R-12）は、unit-of-work.md の U1 の責務に移っている。U2 の責務は引き下げと戻しだけで、「FR2.3 は U1 で済んでいる」と明記されている。unit-of-work-dependency.md の依存の理由とつなぎ目、story-map の「複数の作業単位にまたがるもの」「作業単位の中での順番」も同じ分け方にそろっている。components.md の CallPacer／EventFetcher の記述（試行の先頭で許可を待ち、「止めた」なら AWS を呼ばずに終える）とも食い違わない。
- R-02（U1 と U3 の間のあきらめの動き）：解消済み。U1 の責務に、今の ExhaustionStreak（3 本）を終わった順に当てはめ、3 本続いたら未着手は呼ばずに失敗とし、読み途中は最後まで読む、という暫定の動きが書かれた。依存の文書の「薄い一本」の節と story-map にも同じ記述がある。retry.rs の `ExhaustionStreak::record` は使い切り以外で 0 に戻す作りで、暫定の動きの説明は今のコードと合う。
- R-03（U5 の部品の抜け）：解消済み。U5 の含む部品に RecordingSink（Cancelled と読み終えていない本数 N の記録）と FetchCheck（止める信号の理由への追従）が入った。U1 にも RecordingSink・FetchCheck・FakeGateway が入っており、components.md の記述と合う。
- R-04（U1 の大きさ）：人間の判断どおり Delivery Planning に回されている。再指摘しない。

依存の形は U1 を根にした非循環で、ストーリー 9 本はすべて割り当て済み、traceability.json とも一致する。下の 2 件は、読む人が迷う程度の軽い指摘である。

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U1 責務（CallPacer の固定の上限）と U2 責務（再試行の前にも許可を待つこと）、およびunit-of-work-story-map.md > US2.2 の行 | 「再試行も呼び出しの数に入れる」(FR2.3) の持ち主が食い違っている。U1 は「試行ごとの許可」を持つと書き、U2 は「再試行の前にも許可を待つこと（FR2.3）」を責務にしている。ストーリー対応表では FR2.3 が U2 側に載っている。一方で U1 は SDK の再試行を止めて NFR3（固定の上限を超えない）を自動テストで確かめるとしている。再試行が許可を通らなければ、制限を受けて再試行が出る場面で U1 の NFR3 の確認が成り立たず、手元の実 AWS での確認も他の利用者に迷惑をかけない保証にならない。あわせて、許可待ちの最中に止める信号(閉じる)を見て抜ける道（R-12：今の `Retrier::call` には許可待ちから「止めた」を返す道がない）が U2 の機能設計に回されているが、U1 の CallPacer が先にこの許可待ちを作るため、U1 でも閉じる動作が許可待ちで止まらないようにする必要がある。 | FR2.3 と許可待ちの「止めた」の道を U1 の責務に移す（Retrier の試行の手順への組み込みと R-12 の決定を U1 の機能設計で行う）。U2 の責務は引き下げと戻し（FR2.4）、画面に出さないこと（FR2.6）に絞り、ストーリー対応表の FR2.3 の持ち主と、含む部品に U1 の RetryPolicy を足すことを合わせて直す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work-dependency.md > 薄い一本（「あきらめは今の動きのまま」）、unit-of-work.md > U1 責務（あきらめの決まりは U3） | U1 が出荷される時点（U3 より前）で、同時に読むときの「使い切りが続いたらあきらめる」の動きが定まっていない。今の ExhaustionStreak と `max_consecutive_exhausted` は 1 本ずつ順に読む前提で RetryPolicy にあり、U3 で終わった順の数えに移す。U1 が同時に読む形に変えた後も「今の動き」を保てるのか、数えを外して無制限に再試行するのかが書かれていない。U1 は薄い一本として実際の AWS で確かめる単位で、制限を受け続ける場面ほど待ちが長くなりうる。 | U1 の説明に、U3 までの暫定の動き（例：今の数えを同時の取得でも終わった順に使う、または U3 までは未着手のあきらめを無効にしてその旨を記す）を 1 行で決めて書く。決められない場合は U1 の機能設計で決めると明記し、U3 の「U1 の StreamSchedule に足す形」との境目を揃える。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U5 含む部品 | components.md では RecordingSink が Cancelled と読み終えていない本数 N を記録できるようにする、FetchCheck が止める信号の理由の変更に追従すると書かれているが、U5 の含む部品にどちらも載っていない（U1 には載っているが、Cancelled を足すのは U5）。U5 の自動テストの確かめ方（N の数え方、止め方）が RecordingSink に依存するため、担当の抜けになる。 | U5 の含む部品に RecordingSink と FetchCheck を足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U1（大きさ L、含む部品 12 個） | 薄い一本が 12 部品・L で、「最小の動く一本」としてはやや大きい。Q1 で人が選んだ内容（固定の上限と SDK の再試行停止を含める）に沿っており、指摘の 1 つ目を U1 に寄せるとさらに増えるため、問題とまでは言えない。 | 現状のまま承認してよい。U1 の機能設計で、FetchStatus の切り出しや FakeGateway の追従などを先に小さな Bolt に分けられるか Delivery Planning で確かめる。 | New |
| R-05 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U1 薄い一本・同時取得 > 含む部品 | U1 の責務は、今の `Retrier::call` の形を変えて「止めた」を返す道を作ることと、RetryPolicy の ExhaustionStreak を暫定のあきらめに使うことを挙げている。どちらも components.md では RetryPolicy 部品の持ち物である。ところが U1 の含む部品に RetryPolicy が無い（U5 と U3 には載っている）。R-03 と同じ種類の載せ忘れで、U1 の手を入れる範囲が部品の一覧から読み取れない。 | U1 の含む部品に RetryPolicy を足す。あわせて、story-map の U1 の順番にある「Retrier の試行の手順への組み込み」が RetryPolicy 部品の変更だと分かるようにする。 | New |
| R-06 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U3 あきらめの決まり > 実装の注意 | 「今の動きとの違いは、使い切り以外の失敗で数え直すこと（FR3.4）を含めて」と書かれている。しかし FR3.4 は requirements で「今と同じく数を 0 に戻す」とされ、retry.rs の `ExhaustionStreak::record` も今そうしている。U1 の暫定の動きも今の動きなので、U3 に残る本当の差は、数える場所の移動と終わった順での数え方だけになる。この書き方では、FR3.4 が U1 の暫定では抜けていて U3 で足されるように読める。 | 「違い」の書き方を直す。FR3.4 は暫定の動きにも元からある振る舞いであり、U3 では StreamSchedule に移した後も保たれることをテストで確かめる、という趣旨に改める。 | New |

> R-01 Not re-checked this round

> R-02 Not re-checked this round

> R-03 Not re-checked this round

> R-04 Not re-checked this round
