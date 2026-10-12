## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-10-12T16:12:00Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U1 責務（CallPacer の固定の上限）と U2 責務（再試行の前にも許可を待つこと）、およびunit-of-work-story-map.md > US2.2 の行 | 「再試行も呼び出しの数に入れる」(FR2.3) の持ち主が食い違っている。U1 は「試行ごとの許可」を持つと書き、U2 は「再試行の前にも許可を待つこと（FR2.3）」を責務にしている。ストーリー対応表では FR2.3 が U2 側に載っている。一方で U1 は SDK の再試行を止めて NFR3（固定の上限を超えない）を自動テストで確かめるとしている。再試行が許可を通らなければ、制限を受けて再試行が出る場面で U1 の NFR3 の確認が成り立たず、手元の実 AWS での確認も他の利用者に迷惑をかけない保証にならない。あわせて、許可待ちの最中に止める信号(閉じる)を見て抜ける道（R-12：今の `Retrier::call` には許可待ちから「止めた」を返す道がない）が U2 の機能設計に回されているが、U1 の CallPacer が先にこの許可待ちを作るため、U1 でも閉じる動作が許可待ちで止まらないようにする必要がある。 | FR2.3 と許可待ちの「止めた」の道を U1 の責務に移す（Retrier の試行の手順への組み込みと R-12 の決定を U1 の機能設計で行う）。U2 の責務は引き下げと戻し（FR2.4）、画面に出さないこと（FR2.6）に絞り、ストーリー対応表の FR2.3 の持ち主と、含む部品に U1 の RetryPolicy を足すことを合わせて直す。 | New |
| R-02 | Major | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work-dependency.md > 薄い一本（「あきらめは今の動きのまま」）、unit-of-work.md > U1 責務（あきらめの決まりは U3） | U1 が出荷される時点（U3 より前）で、同時に読むときの「使い切りが続いたらあきらめる」の動きが定まっていない。今の ExhaustionStreak と `max_consecutive_exhausted` は 1 本ずつ順に読む前提で RetryPolicy にあり、U3 で終わった順の数えに移す。U1 が同時に読む形に変えた後も「今の動き」を保てるのか、数えを外して無制限に再試行するのかが書かれていない。U1 は薄い一本として実際の AWS で確かめる単位で、制限を受け続ける場面ほど待ちが長くなりうる。 | U1 の説明に、U3 までの暫定の動き（例：今の数えを同時の取得でも終わった順に使う、または U3 までは未着手のあきらめを無効にしてその旨を記す）を 1 行で決めて書く。決められない場合は U1 の機能設計で決めると明記し、U3 の「U1 の StreamSchedule に足す形」との境目を揃える。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U5 含む部品 | components.md では RecordingSink が Cancelled と読み終えていない本数 N を記録できるようにする、FetchCheck が止める信号の理由の変更に追従すると書かれているが、U5 の含む部品にどちらも載っていない（U1 には載っているが、Cancelled を足すのは U5）。U5 の自動テストの確かめ方（N の数え方、止め方）が RecordingSink に依存するため、担当の抜けになる。 | U5 の含む部品に RecordingSink と FetchCheck を足す。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/inception/units-generation/unit-of-work.md > U1（大きさ L、含む部品 12 個） | 薄い一本が 12 部品・L で、「最小の動く一本」としてはやや大きい。Q1 で人が選んだ内容（固定の上限と SDK の再試行停止を含める）に沿っており、指摘の 1 つ目を U1 に寄せるとさらに増えるため、問題とまでは言えない。 | 現状のまま承認してよい。U1 の機能設計で、FetchStatus の切り出しや FakeGateway の追従などを先に小さな Bolt に分けられるか Delivery Planning で確かめる。 | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| なし（このステージに検証ツールの指定なし） | — | 依存の向きは U2〜U5 が U1 のみに依存する非循環で、ストーリー 9 本はすべて対応済み（traceability.json と一致）。 |

### Summary

作業単位の分け方、依存の非循環、ストーリーの対応は整っていて、薄い一本は DAG の先頭にあり、他の単位は U1 のみに依存する。承認前に見てほしいのは、再試行も数える許可 (FR2.3) と許可待ちの停止の道が U1 と U2 のどちらの責務か曖昧な点と、U1 から U3 までの間のあきらめの動きが未定義な点である。
