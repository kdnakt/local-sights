# Entities — U3 あきらめの決まり（u3-give-up-rule）

上流：`inception/domain-design/components.md`（StreamSchedule・RetryPolicy・EventFetcher）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）、`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4）、U1 の `construction/u1-concurrent-skeleton/functional-design/entities.md`（StreamSlot・ScheduleOutcome）。まとめの確認は `functional-design-questions.md`。

U3 で持ち主が変わるもの、または意味がはっきりするものだけを書く。StreamSlot・PageStep・StopReason・JobStatus は U1 のまま。

## エンティティ（機械可読）

```yaml
entities:
  - name: ExhaustionStreak
    owner: StreamSchedule
    description: ストリームが終わった順に数えた、続けて再試行を使い切って失敗した本数（U1 では RetryPolicy が持っていた。ADR-007 で StreamSchedule に移す）
    attributes:
      - name: consecutive
        type: integer
        required: true
        min: 0
        constraints: 使い切りで 1 増え、成功か使い切り以外の失敗で 0 に戻る。閉じて止めたストリーム（Aborted）では変わらない
      - name: limit
        type: integer
        required: true
        min: 1
        default: 3
        constraints: あきらめるまでの本数。StreamSchedule の設定として持つ（RetryPolicy の max_consecutive_exhausted をなくす）
    relationships:
      - target: ScheduleOutcome
        cardinality: one-to-one
        direction: ExhaustionStreak → ScheduleOutcome（exhaustionStreak の値）

  - name: StreamEnding
    owner: StreamSchedule
    description: あきらめの数えから見たストリームの終わり方。StreamSchedule が EventFetcher の終わり方（StreamFetchEnd）から分ける（U1 では retry.rs と fetcher.rs の `streak_ending` が持っていた）
    attributes:
      - name: value
        type: enum
        required: true
        allowed_values: [Succeeded, Exhausted, FailedWithoutExhaustion, NotCounted]
        constraints: StreamFetchEnd の Completed → Succeeded、Failed（使い切り）→ Exhausted、Failed（使い切り以外）→ FailedWithoutExhaustion、Aborted → NotCounted
    relationships: []

  - name: GiveUp
    owner: StreamSchedule
    description: あきらめが起きたことの記録。取得 1 回に高々 1 回
    attributes:
      - name: triggeredBy
        type: text
        required: true
        constraints: 3 本目の使い切りが起きたストリームの名前
      - name: failure
        type: record
        required: true
        constraints: そのストリームの失敗の内容（ApiFailure、安全な詳細だけ）。Skipped にした未着手にそのまま写す
      - name: skippedCount
        type: integer
        required: true
        min: 0
        constraints: そのとき未着手だったストリームの本数
    constraints:
      - 一度起きたら取り消さない。あきらめた後に終わった読み途中のストリームは、その結果のまま記録する
    relationships:
      - target: StreamSlot
        cardinality: one-to-many
        direction: GiveUp → StreamSlot（Skipped にした未着手）
```

## まとめ

| エンティティ | 持ち主 | U3 での役割 |
|--------------|--------|-------------|
| ExhaustionStreak | StreamSchedule | 続けて使い切った本数とあきらめるまでの本数（RetryPolicy から移す） |
| StreamEnding | StreamSchedule | EventFetcher の終わり方を、数えから見た終わり方に分ける（閉じて止めたものは数えない） |
| GiveUp | StreamSchedule | あきらめが起きたストリームと写す失敗、Skipped にした本数 |

## Assumptions & Open Questions

None.
