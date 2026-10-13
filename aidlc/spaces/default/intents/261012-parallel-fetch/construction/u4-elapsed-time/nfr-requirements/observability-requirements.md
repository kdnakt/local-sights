# Observability Requirements — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/functional-design/`（functional-spec.md・rules.md BR4.1〜BR4.10、レビュー 1 回目の R-01〜R-06）、`inception/requirements-analysis/requirements.md`（FR4.1〜FR4.6、NFR1〜NFR10）、`inception/contract-design/contract-summary.md`（C4、C5）、`codekb/local-sights/technology-stack.md`。質問票の回答は `nfr-requirements-questions.md` の Q1 とまとめの確認。

U4 の経過は利用者に見せる表示で、運用のための監視は足さない（デスクトップアプリで、外への送信はしない）。

| ID | 要件 | 確かめ方 | 出典 |
|----|------|----------|------|
| NFR2.3 | 利用者は状態行の経過で、取得にかかった時間を見られる（取得中は進み、終わると残る）。手元の確認（NFR2）はこの表示で行う | GUI で目で確かめる（unit-of-work の確かめ方） | NFR2、FR4.1〜FR4.3 |
| NFR2.4 | 取得 1 回の要約の診断ログ（U1 の 1 行、`elapsed_ms=`）は変えない。要約の elapsed_ms は取得の流れの中で測った値で、状態行の経過（［取得］を受け入れた時点から）とは数十ミリ秒ほど違いうる | 既存の要約の書式のテストが変わらずに通る | U1 の NFR8.3 |
| NFR9.1 | 経過の文言は英語と日本語を用意し、OS の言語で出し分ける（「Elapsed {time}」／「経過 {time}」） | 画面のテストで両方の言語の文言を確かめる。文言の一覧のテスト（両言語のキーがそろうこと）が通る | NFR9、AC2.1.6 |
| NFR9.2 | 読み上げ：取得中は経過の部分に `aria-hidden="true"` を付け、秒の進みを読み上げない。終わったら外す。最終の経過が読み上げられるのは、属性を外すのが終わりの状態行の変化（取得中の表示から件数などの表示への置き換え）と同じ描き直しで起き、`role="status"` の領域の変化として一緒に伝わるためである | 画面のテストで、running の間は属性があり、終わった後はないこと、属性を外すのが終わりの表示と同じ描き直しであること。GUI の確認で VoiceOver を使い、終わったときに最終の経過が読まれることを一度確かめる | FR4.6、AC2.1.7、機能設計 R-02・Q1 |

## Assumptions & Open Questions

None.
