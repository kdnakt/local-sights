# Security Design — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/nfr-requirements/`（security-requirements.md NFR3.7・NFR6.1・NFR7.1・NFR7.2・NFR8.1・NFR8.9、tech-stack-decisions.md、レビュー 1 回目の R-01〜R-04）、`construction/u3-give-up-rule/functional-design/`（rules.md BR3.1〜BR3.7、functional-spec.md）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）。まとめの確認は `nfr-design-questions.md`。

この文書は、機能設計のレビュー（R-01〜R-06）と NFR 要件のレビュー（R-01〜R-04）で残った点の正本を兼ねる。機能設計の文書はレビューの記録の後なので書き換えず、食い違うところはこの文書が優先する。

## 設計

| 要件 | 仕組み |
|------|--------|
| NFR3.7（あきらめたら未着手を呼ばない） | `StreamSchedule::finish` が 3 本目の使い切りを受けたら、その場で未着手をすべて Skipped にして返し、`start_next` は以後何も返さない。使い切りの終わり方が失敗の内容を必ず持つことは型で保証する：fetcher.rs の `StreamFetchEnd::Failed` を `Failed { exhausted: bool, failure: ApiFailure }` に変え（`Copy` をやめて `Clone` にする）、EventFetcher は `CallResult::Failed { failure, exhausted }` の値をそのまま入れる。`finish` はこの `failure` を写すので、「内容が無いので Skipped にしない」分岐はなくなる（NFR 要件のレビュー R-04、NFR 設計のレビュー R-01） |
| NFR8.9（写す失敗は安全な詳細だけ） | 3 本目の使い切りの `ApiFailure` を複製して写す。新しい文字列は作らない |
| NFR8.1（新しい API を呼ばない） | CloudWatchLogsGateway の trait を変えない |
| NFR6.1（実際の AWS に接続しない） | StreamSchedule の単体テストと FakeGateway の結合テストだけ |
| NFR7.1（既存のテスト） | 型と引数の変更に合わせて直すテストは次だけで、確かめる中身は弱めない：retry.rs の数えのテスト 2 件は schedule.rs に移す。schedule.rs のテストは `finish` の新しい引数と `with_limit` に合わせて呼び出しを直す。fetcher.rs のテストで `StreamFetchEnd::Failed { exhausted: true }` と比べているものは、`matches!` で exhausted と失敗の種類を確かめる形にする。`tests/` の結合テスト（u1〜u6、u1_concurrent、u2_call_pacing）は変えずに通す |
| NFR7.2（テスト先行とカバレッジ） | StreamSchedule の数えの決まりは純粋な部品としてテストを先に書く。行カバレッジ 80% を CI で確かめる |

## 決まりの正本（レビューの指摘の片付け）

| 決まり | 内容 | 由来 |
|--------|------|------|
| 数えの持ち主（BR3.6） | 数え（今の `ExhaustionStreak` を schedule.rs の非公開の構造体として移す）・本数・終わり方の分け方は schedule.rs の中だけに置く。`StreamEnding` という型は作らず、今の `SlotEnd` に数えのための分け方（非公開の関数）を持たせる。retry.rs の `StreamEnding`・`ExhaustionStreak`・`RetryPolicy::max_consecutive_exhausted`、fetcher.rs の `StreamFetchEnd::streak_ending` はなくす | ADR-007、NFR 要件のレビュー R-02、NFR 設計のレビュー R-02 |
| 終わり方の受け取り（R-02） | `StreamSchedule::finish(index, end: StreamFetchEnd)` が EventFetcher の終わり方をそのまま受け取り、中で `SlotEnd`（Completed・FailedExhausted・FailedNotExhausted・Aborted・Skipped）を決める。FetchCoordinator の中の変換はなくす | NFR 要件のレビュー R-02 |
| 閉じて止めたもの（BR3.5） | `StreamFetchEnd::Aborted` は数えを変えない | 機能設計 BR3.5 |
| 本数（BR3.6） | schedule.rs の公開の定数 `GIVE_UP_LIMIT: u32 = 3`。`StreamSchedule::new(names)` はそれを使い、本数を渡す公開の `StreamSchedule::with_limit(names, limit)` を別に持つ（単体テストと将来の使い方のため）。1 未満は 1 に切り上げる | NFR 要件の前提 R-02、NFR 設計のレビュー R-03 |
| あきらめた後の数え（BR3.3） | あきらめたら数えを動かさない（今のコードからの変更）。読み途中の結果は今までどおり記録する | 機能設計のレビュー R-03 |
| 観測の口（R-04） | 公開の構造体 `GiveUp { triggered_by: String, failure: ApiFailure, skipped_count: usize }` を作り、`ScheduleOutcome` に `give_up: Option<GiveUp>`（あきらめなかったら None）を足す。`ScheduleOutcome` は `Copy` をやめて `Clone` にし、`outcome()` は値を複製して返す。未着手が 0 本のときもあきらめの記録は残る（skipped_count 0） | 機能設計のレビュー R-04、NFR 設計のレビュー R-03 |
| 例の数値（R-01） | 8 本の枠は 1 本終わるごとに埋まるので、functional-spec.md 2 章の例では、あきらめの時点の未着手は s14〜s20（s09〜s13 は読み途中）。テストはこの数え方で書く | 機能設計のレビュー R-01 |

## 確かめること

| 確かめること | 由来 |
|--------------|------|
| 成功で数え直す（使い切り 2 → 成功 → 使い切りで 1） | AC2.3.1、FR3.1 |
| 3 本で未着手を Skipped、3 本目の失敗を写す、選ばれた順、未着手は呼ばれない | AC2.3.2、FR3.2、NFR3.7、NFR8.9 |
| 読み途中は最後まで読み、結果がそのまま記録される。あきらめは 1 回、後から数えが動かない | AC2.3.3、FR3.3、BR3.3 |
| 使い切り以外の失敗で 0 | AC2.3.4、FR3.4 |
| 終わる順の入れ替え（偽物のゲートウェイの呼び出し時間で、読み始めと違う順に終わる） | FR3.1 |
| 閉じて止めたものは数えない（StreamSchedule の単体テスト） | BR3.5 |
| 未着手が 0 本のときもあきらめの記録が残る | 機能設計のレビュー R-04 |
| 本数 1 未満は 1 | 本数の決まり |

## 脅威と対策

| 脅威 | 対策 |
|------|------|
| 制限が続くのに残りを呼び続ける | あきらめたら必ず未着手を呼ばない（型で保証） |
| 分け方が 2 か所に残り食い違う | schedule.rs の中だけに置く |
| Skipped の詳細に秘密が混ざる | 元の ApiFailure を複製するだけ |

## Assumptions & Open Questions

None.
