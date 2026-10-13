# Functional Spec — U3 あきらめの決まり（u3-give-up-rule）

上流：`inception/units-generation/unit-of-work.md`（U3）・`unit-of-work-story-map.md`（US2.3）、`inception/user-stories/stories.md`（AC2.3.1〜AC2.3.4）、`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4）、`inception/domain-design/components.md`・`decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）、`entities.md`、`rules.md`、U1 の `functional-spec.md`。まとめの確認は `functional-design-questions.md`。

この文書は U3 の流れの正本である。データの形は `entities.md`、判断の決まりは `rules.md` が正本。

## 1. 流れ

1. FetchCoordinator は一覧の後、選ばれたストリームと limit（既定 3）で StreamSchedule を作る（BR3.6）。U1 と違い、limit は RetryPolicy から読まない。
2. 読む段は U1 のまま（最大 8 本、選ばれた順）。
3. あるストリームの読み取りが Finished で終わったら、FetchCoordinator はその終わり方（StreamFetchEnd）と結果を StreamSchedule に渡す。StreamSchedule は次の順に決める。
   1. 終わり方を StreamEnding に分ける（Completed → Succeeded、Failed（使い切り）→ Exhausted、Failed（使い切り以外）→ FailedWithoutExhaustion、Aborted → NotCounted）
   2. まだあきらめていなければ、数えを当てはめる（BR3.1、BR3.4、BR3.5）
   3. 数えが limit に達したら、未着手をすべて Skipped にし、このストリームの失敗の内容を写して返す（BR3.2）
4. FetchCoordinator は、終わったストリーム、続いて Skipped にしたストリームを、その順で FetchJob に記録し、それぞれ `on_stream_finished` を呼ぶ（U1 のまま、BR3.7）。
5. あきらめた後、読み途中のストリームは最後まで読み、結果をそのまま記録する（BR3.3）。新しく読み始めるものはない。
6. 取得の終わりの状態は U1 のまま決める（BR3.7）。

```
finish(slot, fetch_end, outcome) -> Vec<Skipped>:
  ending = classify(fetch_end)              # StreamSchedule が分ける
  slot.state = Succeeded or Failed
  if give_up is none and ending != NotCounted:
    streak.record(ending)                   # Exhausted +1、Succeeded/FailedWithoutExhaustion で 0
    if streak.consecutive >= limit:
      give_up = (slot.name, outcome.failure)
      return 未着手をすべて Skipped にした並び（選ばれた順）
  return []
```

## 2. 例（limit 3、同時に読む）

| 終わった順 | 終わり方 | 数え | 起きること |
|------------|----------|------|------------|
| s02 | 使い切り | 1 | |
| s05 | 使い切り | 2 | |
| s01 | 成功 | 0 | （AC2.3.1：この後の使い切りで 1） |
| s03 | 使い切り | 1 | |
| s04 | 権限エラー | 0 | （AC2.3.4） |
| s06・s07・s08 | 使い切り | 1・2・3 | 3 本目（s08）であきらめ。未着手 s10〜s20 を Skipped にし、s08 の失敗の内容を写す（AC2.3.2） |
| s09 | 成功 | — | 読み途中だったので最後まで読み、成功として記録（AC2.3.3）。数えは変えない |

## 3. 確かめ方（自動テストの条件）

StreamSchedule の決まりは純粋な部品としてテストを先に書く（team.md）。取得の流れは偽物のゲートウェイで確かめる。実際の AWS には接続しない。

| 確かめること | 条件 | 合格の基準 | 関わる AC |
|--------------|------|------------|-----------|
| 成功で数え直す | 使い切り 2 本 → 成功 1 本 → 使い切り 1 本（終わった順） | 数は 1、あきらめない | AC2.3.1 |
| 3 本で未着手を失敗に | 使い切りが 3 本たまる、未着手が残る | 未着手は呼ばれず Skipped、3 本目の失敗の内容が写る、選ばれた順に失敗の一覧に入る | AC2.3.2 |
| 読み途中は最後まで | あきらめたとき読み途中が 2 本 | その 2 本は最後まで読み、結果がそのまま記録される。あきらめは 1 回だけ | AC2.3.3 |
| 使い切り以外で 0 | 使い切り 2 本 → 権限エラー | 数は 0 | AC2.3.4 |
| 終わる順の入れ替え | 読み始めた順と終わった順が違う台本（偽物のゲートウェイで時間を変える） | 終わった順で数える | AC2.3.1、AC2.3.2 |
| 閉じて止めたものは数えない | 使い切り 2 本の後に閉じる | 数は 2 のまま、取得は Aborted（U1 のまま） | （BR3.5） |
| 持ち主の移動 | RetryPolicy・EventFetcher | RetryPolicy に max_consecutive_exhausted が無く、EventFetcher が StreamEnding を使わない（コードの確認とビルド） | （BR3.6） |
| 既存のテスト | U1・U2 の結合テスト、今の流れのテスト | 呼び出しの変更（limit の渡し方）以外は変えずに通る | （BR3.7） |

## 4. 規則のまとめ

| 分類 | ルール |
|------|--------|
| 数え方 | BR3.1、BR3.4、BR3.5 |
| あきらめ | BR3.2、BR3.3 |
| 持ち主と既存の動き | BR3.6、BR3.7 |

## 5. 持ち越しの扱い

| 指摘・決め | U3 での扱い |
|------------|-------------|
| U1 の BR1.13（暫定のあきらめ） | U3 で正式な決まり（BR3.1〜BR3.5）に置き換える。数え方の結果は同じ |
| ADR-007（数えの移動） | BR3.6 で行う |
| 約束 C2 の U5 の分（中断のときの本数 N と「最後が終わった後の中断は終わった扱い」） | U5 で足す。U3 は閉じるときだけを扱う |

## Assumptions & Open Questions

None.
