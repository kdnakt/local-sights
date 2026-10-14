# Entities — U5 中断（u5-cancel）

上流：`inception/units-generation/unit-of-work.md`（U5）、`inception/requirements-analysis/requirements.md`（FR5.1〜FR5.11、NFR4、NFR8〜NFR10）、`inception/user-stories/stories.md`（US3.1、US4.1、US4.2）、`inception/domain-design/components.md`・`decisions.md`（ADR-004、ADR-006、ADR-009）、`inception/contract-design/contract-summary.md`（C2〜C7）、`inception/refined-mockups/`（画面 A〜D、CancelNotice、文言の一覧）。質問票は `functional-design-questions.md`（まとめの確認）。

U5 は、U1 で用意した理由つきの止める信号（C7）に「中断」を送る道を足し、終わりの状態に Cancelled を足し、中断の注意を出す。

## エンティティ（機械可読）

```yaml
entities:
  - name: StopReason
    owner: RetryPolicy
    description: 止める信号の理由（C7、U1 で用意済み）。U5 で Cancel を送る道を足す
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Cancel, Close]
        constraints: Close は常に Cancel より強い。Cancel の後の Close は Close に強める。Close の後の Cancel は何もしない（ADR-004）
  - name: JobStatus
    owner: FetchStatus
    description: 取得 1 回の終わりの状態（C3）。U5 で Cancelled を足す
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Running, Completed, Failed, CompletedWithFailures, Aborted, Cancelled]
        constraints: Cancelled は利用者の中断で、取れたログを残す。Aborted は閉じるときの中止で、取れたログを捨てる（今と同じ）。キャッシュに書くのは Completed かつ失敗 0 だけ
  - name: StreamEnding
    owner: StreamSchedule
    description: 1 本のストリームの終わり方（今の SlotEnd）。U5 で意味を足すのは「止めた」だけ
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Finished, Failed, Stopped, Skipped]
        constraints: Stopped は止める信号で読み途中のまま終えたもの（今の Aborted の終わり）。読み終えていない本数に入る。Failed と Skipped は失敗で、入らない。止めた後は新しい Skipped を作らない
  - name: FetchJobCancelFields
    owner: FetchCoordinator
    description: FetchJob に足す項目（C4）
    attributes:
      - name: unfinished_stream_count
        type: integer
        required: true
        min: 0
        default: 0
        constraints: 中断で終えたときの N（止めた＋まだ始めていない）。中断でなければ 0
      - name: cancelled_before_listing
        type: boolean
        required: true
        default: false
        constraints: 一覧を読み終える前に中断で止めたら true
  - name: CancelNoticeView
    owner: AppSession
    description: 中断の注意（C5 の cancelNotice）
    attributes:
      - name: unfinishedStreams
        type: integer
        required: false
        min: 1
        constraints: 一覧の後の中断の N。一覧の前の中断なら無い（null）
    constraints:
      - SessionView.cancelNotice が null なら出さない
      - 次の取得を始めたとき、接続を変えたとき、別のロググループを選んだときに null に戻す
  - name: FetchControlState
    owner: AppSession
    description: 取得のボタンの状態を決める項目（C5）
    attributes:
      - name: cancelling
        type: boolean
        required: true
        default: false
        constraints: 中断を受けてから止まるまで true。段階は Fetching のまま
      - name: fetchCancelAvailable
        type: boolean
        required: true
        constraints: 取得中で、cancelling でなく、キャッシュに保存中でもないときだけ true（キャッシュから表示する道は 1 回の処理で終わるため、別の条件は持たない）
    relationships:
      - target: JobStatus
        cardinality: 1 対 1
        direction: FetchControlState → JobStatus（終わりを受けて cancelling を戻す）
```

## まとめ

| エンティティ | 持ち主 | U5 での変化 |
|--------------|--------|-------------|
| StopReason | RetryPolicy | Cancel を送る道を足す（型は U1 のまま） |
| JobStatus | FetchStatus | Cancelled を足す |
| StreamEnding | StreamSchedule | 止めたストリームを「読み終えていない」に数える |
| FetchJob の項目 | FetchCoordinator | unfinished_stream_count、cancelled_before_listing |
| CancelNoticeView | AppSession | 新規 |
| FetchControlState | AppSession | cancelling、fetchCancelAvailable |

## Assumptions & Open Questions

None.
