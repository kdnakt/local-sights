# NFR Design 質問票 — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/nfr-requirements/`（security-requirements.md NFR3.7〜NFR7.2、tech-stack-decisions.md、レビュー 1 回目の R-01〜R-04）、`construction/u3-give-up-rule/functional-design/`（rules.md BR3.1〜BR3.7、functional-spec.md、レビュー 1 回目の R-01〜R-06）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）、今のコード（`schedule.rs`・`retry.rs`・`fetcher.rs`・`coordinator.rs`）。

U3 の仕組みは機能設計と NFR 要件でほぼ決まっていて、技術も U1 のままのため、新しく利用者に聞くことはない。レビューで残った軽微な指摘は、この設計を正本として次のように決める（機能設計の文書はレビューを記録した後なので書き換えず、ここで上書きする。project.md Corrections）。

## 前提として決めたこと（質問にしない技術的な決め）

- **機能設計の例と持ち越しの直し（NFR 要件のレビュー R-01）**：functional-spec.md 2 章の例は、8 本の枠が 1 本終わるごとに埋まるため、あきらめの時点の未着手は s14 以降（s09〜s13 は読み途中）が正しい。5 章の「数え方の結果は同じ」は、あきらめた後に数えを動かさない点だけ今のコードから変わる。テストと対応表はこの設計に従う。
- **終わり方の型（NFR 要件のレビュー R-02）**：`StreamSchedule::finish` は EventFetcher の `StreamFetchEnd` をそのまま受け取り、中で今の `SlotEnd`（スロットの終わり方）と数えのための分け方を決める。FetchCoordinator の中の変換はなくす。retry.rs の `StreamEnding`・`ExhaustionStreak` と fetcher.rs の `streak_ending` はなくし、schedule.rs の中だけで使う型にする。
- **規範に入れる決まり（NFR 要件のレビュー R-03）**：閉じて止めたものは数えない、数えの持ち主は StreamSchedule、本数は定数 3（1 未満は 1）、あきらめの記録は `ScheduleOutcome` に載せてテストで読む、を security-design の要件として書き、対応表に FR3.1〜FR3.4 と BR3.5・BR3.6 を載せる。
- **失敗の内容が無い使い切り（NFR 要件のレビュー R-04）**：`StreamFetchEnd::Failed { exhausted: true }` のとき、EventFetcher は必ず失敗の内容を持つ（`CallResult::Failed` が `failure` を持つため）。これを型で保証するため、使い切りの終わり方が失敗の内容を持つ形にし、「内容が無いので Skipped にしない」分岐はなくす。あきらめたら必ず未着手を呼ばない（NFR3.7）。

## Consolidated Summary Confirmation

- 数えの持ち主は StreamSchedule。終わり方の分け方も StreamSchedule の中だけで行い、RetryPolicy と EventFetcher から数えの型をなくす
- 使い切りの終わり方は必ず失敗の内容を持つ形にし、あきらめたら必ず未着手を呼ばない
- あきらめの記録（あきらめたストリーム・写した失敗・Skipped の本数）は ScheduleOutcome に載せ、テストで読む。あきらめた後は数えを動かさない
- あきらめる本数は定数 3（1 未満は 1）。呼び出し側は変えない
- 機能設計の例の数値の誤り（未着手は s14 以降）と「結果は同じ」の言い過ぎは、この設計を正本として直す
- 新しい依存・API・画面・保存先は足さない。テストは止めた時計と偽物のゲートウェイで、実際の AWS に接続しない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
