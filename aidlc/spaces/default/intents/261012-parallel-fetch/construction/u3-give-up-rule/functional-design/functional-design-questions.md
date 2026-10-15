# Functional Design 質問票 — U3 あきらめの決まり（u3-give-up-rule）

上流：`inception/units-generation/unit-of-work.md`（U3）・`unit-of-work-story-map.md`（US2.3）、`inception/user-stories/stories.md`（AC2.3.1〜AC2.3.4）、`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4）、`inception/domain-design/decisions.md`（ADR-007）、`inception/contract-design/contract-summary.md`（C2）、U1 の成果（`construction/u1-concurrent-skeleton/functional-design/rules.md` の BR1.13・BR1.19、`crates/local-sights-core/src/schedule.rs`）。

U3 は、U1 で暫定にした「使い切りが続いたらあきらめる」を正式な決まりにする作業単位である。数え方（終わった順、成功と使い切り以外の失敗で 0 に戻す、3 本で未着手を呼ばずに失敗、読み途中は最後まで読む）は要件（FR3.1〜FR3.4）と受け入れ基準（AC2.3.1〜AC2.3.4）で決まっていて、U1 の暫定の動きもこれと同じ結果になる。U3 で変わるのは、数えの持ち主を RetryPolicy から StreamSchedule に移すこと（ADR-007）と、終わる順を入れ替えた場合をテストで確かめることで、新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **持ち主の移動（ADR-007）**：ExhaustionStreak・あきらめるまでの本数（既定 3）・終わり方の分け方（今の StreamEnding と `streak_ending`）を、RetryPolicy と EventFetcher から StreamSchedule に移す。RetryPolicy は 1 回の要求の再試行と止める信号だけを持つ。EventFetcher は自分の終わり方（StreamFetchEnd）だけを返し、StreamSchedule がそれを分ける。依存は StreamSchedule → EventFetcher の片方向。
- **閉じたときに止めたストリーム（Aborted）**：数えに入れない（今と同じ。閉じた取得は Aborted で終わり、あきらめの判断は使われない）。中断（Cancel）の扱いは U5 で決める。
- **あきらめた後に終わった読み途中のストリーム**：今までどおり結果を記録する（成功なら成功、失敗なら失敗の一覧に入る）。あきらめは一度だけ起き、未着手を Skipped にした後は数えに関係なく取り消さない。
- **Skipped に写す失敗**：3 本目の使い切りが起きたストリームの失敗の内容（安全な詳細だけ、今と同じ）。
- **本数の設定**：あきらめるまでの本数（3）は StreamSchedule の設定として持ち、画面からは変えない。RetryPolicy から項目が 1 つ減るので、使う側（FetchCoordinator・確認用プログラム・テスト）を同じ変更で直す。

## Consolidated Summary Confirmation

- 数え方は今と同じ：ストリームが終わった順に、再試行を使い切って失敗したものを数え、成功か使い切り以外の失敗（権限がないなど）で 0 に戻す（FR3.1、FR3.4）
- 3 本たまったら、未着手のストリームは呼ばずに失敗とし、3 本目の失敗の内容を写す。読み途中のストリームは最後まで読む（FR3.2、FR3.3）
- 閉じて止めたストリームは数えない。あきらめは一度だけで、後から取り消さない
- 数えの持ち主を RetryPolicy から StreamSchedule に移す（ADR-007）。RetryPolicy は再試行と止める信号だけを持つ
- 確かめは StreamSchedule の純粋なテストを先に書き、終わる順を入れ替えた場合（AC2.3.1〜AC2.3.4）を見る。偽物のゲートウェイで、同時に読んでいるときの順でも確かめる
- 画面・要約の 1 行・失敗の一覧の見え方は変えない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
