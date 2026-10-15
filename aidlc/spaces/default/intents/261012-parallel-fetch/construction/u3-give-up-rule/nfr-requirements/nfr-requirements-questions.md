# NFR Requirements 質問票 — U3 あきらめの決まり（u3-give-up-rule）

上流：`construction/u3-give-up-rule/functional-design/`（rules.md BR3.1〜BR3.7、functional-spec.md、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR3.1〜FR3.4、NFR6・NFR7・NFR8）、`inception/contract-design/contract-summary.md`（C2）、`codekb/local-sights/technology-stack.md`、U1・U2 の `nfr-requirements/`。

U3 は作業単位の種類が library で、StreamSchedule の中の数えの決まりと持ち主を整える変更である。新しい画面・API・依存・保存先を持たず、非機能の要件は U1・U2 の枠の中に収まるため、新しく利用者に聞くことはない。機能設計のレビューの軽微な指摘は、次のように扱う。

## 前提として決めたこと（質問にしない技術的な決め）

- **R-02（limit の渡し元）**：あきらめるまでの本数は StreamSchedule の定数（3）とし、FetchCoordinator は引数なしでそれを使う。`run_fetch` の引数・呼び出し側（確認用プログラム・Tauri アプリ）は変えない。テストは StreamSchedule を直接、別の本数で作れる。1 より小さい値は 1 に切り上げる（今の ExhaustionStreak と同じ）。
- **R-03（あきらめた後の数え）**：あきらめた後は数えを動かさない（BR3.3）。今のコードは後からも数えを動かすため、ここは今からの変更として扱い、テストで確かめる。失敗の一覧・終わった本数・終わりの状態には影響しない。
- **R-04（観測の口）**：あきらめの記録（あきらめたストリーム・写した失敗・Skipped の本数）は ScheduleOutcome に載せ、テストで読めるようにする。使い切りの失敗の内容が無い場合（今は起きない）は、写すものが無いので Skipped にしない今の動きを残す。未着手が 0 本のときは、あきらめの記録だけ残り Skipped は 0 本。
- **R-05（終わり方の型）**：EventFetcher の StreamFetchEnd から StreamSchedule の中の終わり方（StreamEnding）に分けるのは StreamSchedule だけが行う。retry.rs の StreamEnding と ExhaustionStreak、fetcher.rs の `streak_ending` はなくす。閉じて止めたものを数えないことは StreamSchedule の単体テストで確かめる。
- **R-01（例の数値）・R-06（対応表）**：コード生成の計画とテストで正しい例（あきらめの時点の未着手は s14 以降）を使い、対応表に FR3.1〜FR3.4 を載せる。

## Consolidated Summary Confirmation

- 新しい画面・API・権限・保存先・依存は足さない。失敗の詳細は今の許可リストを通した安全な詳細だけ（Skipped に写すものも同じ）
- 呼び出しの量：あきらめた後は未着手を呼ばないので、制限が続くときの呼び出しはむしろ減る。U1・U2 の上限はそのまま
- テスト：StreamSchedule の数えの決まりは純粋な部品としてテストを先に書き、終わる順を入れ替えた場合を確かめる。偽物のゲートウェイで同時に読む流れも確かめる。実際の AWS には接続しない。行カバレッジ 80% 以上
- 既存のテスト：RetryPolicy から数えの項目がなくなるので、それを使うテストを同じ変更で直す。ほかは変えずに通す
- あきらめる本数は StreamSchedule の定数（3）とし、呼び出し側は変えない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
