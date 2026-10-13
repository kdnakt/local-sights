# Reliability Design — U1 薄い一本・同時取得（u1-concurrent-skeleton）

上流：`construction/u1-concurrent-skeleton/nfr-requirements/reliability-requirements.md`（NFR1.5〜NFR1.9、NFR4.1、NFR7.1）、`construction/u1-concurrent-skeleton/functional-design/`（BR1.5、BR1.13〜BR1.16、BR1.20）。質問票の回答は `nfr-design-questions.md` の Q1。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR1.5（一部の失敗でも続ける） | 1 本の読み取りが失敗で終わっても、FetchCoordinator は失敗の一覧に足して、ほかの読み取りと未着手の起動を続ける |
| NFR1.6（必ず終わる） | 選ばれたストリームが 0 本なら読む段を飛ばす。読む段は「入れ物が空で、未着手を始められない」ときに抜ける。待つのは入れ物に何か入っているときだけ |
| NFR4.1（閉じるときは速く止まる） | 止める信号（理由つき、StopHandle・StopSignal）を、許可待ち・再試行の待ちと並べて待つ。許可と止める信号が同時に用意できたときは、止める信号を先に見る（`tokio::select!` の `biased` で止める側を上にする）。止めた後は未着手も次の 1 ページも始めず、返ったページは渡さない |
| NFR1.7（再試行は今と同じ） | Retrier の待ちと回数の決まり（RetryPolicy）は変えない。`call` に試行の前の待ち処理を足すだけで、待ち処理なしの `call_unpaced` は今の動きと同じ |
| NFR1.8（あきらめの暫定） | 今の ExhaustionStreak を、ストリームが終わった順に記録する。3 本続いたら StreamSchedule の未着手をすべて Skipped にし、その分の on_stream_finished を呼ぶ |
| NFR7.1（既存のテストを保つ） | JobStatus の移動、止める信号の置き換え、fetch_stream の分割は、使う側をすべて同じ変更で直し、既存のテストを書き換えずに通す（取り込み先の変更は除く） |
| NFR1.9（panic） | 読み取りは取得 1 回のタスクの中で動くので、panic はタスクごと終わり、今の「異常終了した取得の回復」（TauriApp）が拾う |

## 止める理由の強さ

| 今 | 来た理由 | 次 |
|----|----------|----|
| なし | Cancel | Cancel |
| なし | Close | Close |
| Cancel | Close | Close |
| Close | Cancel | Close |
| 同じ理由 | 同じ理由 | 変わらない |

U1 では Close だけを送る。表の全行を単体テストで確かめる（BR1.20）。

## Assumptions & Open Questions

None.
