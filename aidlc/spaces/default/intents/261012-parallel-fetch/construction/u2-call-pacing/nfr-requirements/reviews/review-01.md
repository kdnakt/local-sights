## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/nfr-requirements/security-requirements.md > NFR3.5 の要件欄 | 「下げている間の許可の間隔は今の速さの間隔以上」は BR2.5 と合わない。BR2.5 は、下げたときにすでに予約済みの枠は古い速さの時刻のまま動かさず、許可の間隔は「そのとき使った速さの minInterval 以上」と定めている。書かれたとおりに止めた時計のテストを書くと、下げた直後の予約済みの枠の間隔（古い速さの間隔）で不合格になる。 | 「下げた後に予約する枠の間隔は、予約したときの速さの間隔以上。予約済みの枠は動かさない」と直し、確かめ方に予約済みの枠のテスト（機能設計 4 章の「予約済みの枠」）を明記する。 | New |
| R-02 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/nfr-requirements/security-requirements.md > NFR3.6 の要件欄 | 「リージョンの上限」の意味が U1 とずれている。U1 の NFR3.1 は「リージョンの上限の半分以下」と書き、U2 の NFR3.6 と質問票は「リージョンの上限（東京 12 回）」と書く。U2 の語は U1 の ceilingPerSecond（API の上限の半分）を指すが、そう書かれていないため、テストを書く人が API の実際の上限（東京 25 回）と読むおそれがある。 | NFR3.6 に「ここでの上限は U1 の NFR3.2 の値（東京 12 回など。API の上限の半分）」と一言加える。 | New |
| R-03 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/nfr-requirements/security-requirements.md > 要件の表全体、traceability.json > upstream_ids | U1 の要件のうち、下げた速さで効きが変わるものが引き継がれていない。(a) U1 の NFR4.1（閉じるとき、許可待ち・再試行の待ちの間ならすぐ止まる）は、下限 1 回／秒で最大 8 件が待つと待ちが約 8 秒になる状況で新しく試される。NFR7.1 の「既存のテストは通ったまま」は、速さを下げた状態で閉じるテストを含まない。(b) 再試行の待ちと回数が今と同じこと（FR2.5、BR2.8、U1 の NFR1.7）と、制限を受けない取得の動きが U1 と変わらないこと（NFR1.1、NFR1.3）も、U2 の要件として立っていない。upstream_ids も NFR3・NFR6・NFR7・NFR8 だけで、NFR4 と FR2.4〜FR2.6 が載っていない。 | 速さを下限まで下げた状態で閉じると、許可待ちがすぐ止まり新しい呼び出しを始めないことを確かめる要件を足す。再試行が今と同じこと、制限なしなら速さが上限のまま変わらないことも 1 行ずつ足し、traceability.json に NFR4・FR2.4〜FR2.6 を載せる。 | New |
| R-04 | Minor | aidlc/spaces/default/intents/261012-parallel-fetch/construction/u2-call-pacing/nfr-requirements/security-requirements.md > NFR8.7、NFR8.8 の出典欄 | 親の付け方が合っていない。NFR8 は「進み具合・中断・失敗の表示に秘密を出さない」だが、NFR8.8（引き下げを画面に出さない）の由来は FR2.6 で、秘密の話ではない。NFR8.7 は U1 の NFR8.3・NFR8.6 を言い直したものである。U1 の NFR レビュー R-05 と同じ型の指摘で、traceability.json でも NFR8.8 が NFR8 に載っている。 | NFR8.8 の出典の主を FR2.6 とし、traceability.json にも FR2.6 の行を足す（またはその要件の由来を明記する）。NFR8.7 は U1 の NFR8.3 に `min_rate` の欄を足す形だと書く。 | New |

### Summary

U2 は新しい API・権限・保存先・依存を足さない library の変更で、NFR3・NFR6・NFR7・NFR8 は testable な要件として一通りそろい、project.md の Forbidden とも矛盾せず、CI の `cargo llvm-cov -p local-sights-core --fail-under-lines 80` と FakeGateway の場所も実在する。Critical と Major はなく、NFR3.5 の文言と BR2.5 のずれ、上限の言葉、U1 の引き継ぎ漏れ（閉じる動作・再試行・制限なしの動き）、親 ID の付け方の 4 件が軽微に残るだけなので、判定は READY とする。
