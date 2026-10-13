# NFR Requirements 質問票 — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/functional-design/`（functional-spec.md・rules.md BR2.1〜BR2.8、レビュー 2 回目の R-06・R-07）、`inception/requirements-analysis/requirements.md`（FR2.4〜FR2.6、NFR3・NFR6・NFR7・NFR8）、`inception/contract-design/contract-summary.md`（C1）、`codekb/local-sights/technology-stack.md`、U1 の `nfr-requirements/`。

U2 は作業単位の種類が library で、CallPacer の中の決まり（下げ方と戻し方）を足すだけの変更である。新しい画面・API・依存・保存先を持たない。非機能の要件は、U1 で決めた枠（速さの上限、秘密を出さない、実際の AWS に接続しない、カバレッジ 80%）の中に収まるため、新しく利用者に聞くことはない。

## 前提として決めたこと（質問にしない技術的な決め）

- **呼び出しの量**：下げても戻しても、許可の間隔は今の速さの間隔以上で、今の速さはリージョンの上限を超えないので、U1 の「どの 1 秒も上限以下」（U1 の NFR3.1）はそのまま守られる。
- **要約の 1 行**：足す `min_rate=` は整数だけで、秘密や接続の情報を含まない（U1 の NFR8.3・NFR8.6 の決まりの中）。U1 の要約の書式のテストは U2 の書式に直す。
- **依存**：新しい依存を足さない（tokio の時計と std の Mutex のまま）。
- **テスト**：CallPacer の決まりは純粋な部品としてテストを先に書き、止めた時計で確かめる。行カバレッジ 80% を CI で確かめる（PR #20 の仕組み）。
- **機能設計のレビュー 2 回目の R-06・R-07**（文書の古い欄の名前とテストの条件の言い回し）は、コード生成の計画とテストで正しい形（holdDecreaseUntil、速さ 3 から 1 に下げる場合）を使う。

## Consolidated Summary Confirmation

- 呼び出しの量：下げても戻しても、どの 1 秒もリージョンの上限（東京 12 回）を超えない。下げている間はそれより少ない
- 秘密：要約の 1 行に足すのは整数の `min_rate=` だけ。新しい API・権限・保存先・依存は足さない
- 画面：何も足さない（FR2.6）
- テスト：CallPacer の下げ方・戻し方は止めた時計でテストを先に書く。偽物のゲートウェイで、制限が続くと間隔が広がり、止むと戻ることを確かめる。実際の AWS には接続しない。行カバレッジ 80% 以上
- 既存のテストは通ったまま。ただし U1 の要約の書式のテストは、欄が 1 つ増えるので U2 の書式に直す

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
