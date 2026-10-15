# NFR Design 質問票 — U2 呼び出しの速さ（u2-call-pacing）

上流：`construction/u2-call-pacing/nfr-requirements/`（security-requirements.md NFR3.5〜NFR8.8、tech-stack-decisions.md、レビュー 1 回目の R-01〜R-04）、`construction/u2-call-pacing/functional-design/`（functional-spec.md・rules.md BR2.1〜BR2.8）、`inception/contract-design/contract-summary.md`（C1）、U1 の `nfr-design/`。

U2 の仕組みは機能設計（BR2.1〜BR2.8）でほぼ決まっていて、使う技術も U1 のまま（tech-stack-decisions.md）のため、新しく利用者に聞くことはない。NFR 要件のレビューの軽微な指摘は、この設計の中で次のように片付ける。

## 前提として決めたこと（質問にしない技術的な決め）

- **R-01（間隔の言い方）**：「下げた後に予約する枠の間隔は、予約したときの速さの間隔以上。予約済みの枠は動かさない」と設計に書き、予約済みの枠のテストを確かめ方に入れる。
- **R-02（上限の言葉）**：この設計の「上限」は U1 の ceilingPerSecond（API の上限の半分。東京 12 回）を指す、と書く。
- **R-03（U1 の要件の引き継ぎ）**：下限まで下げた状態で閉じるとすぐ止まること（U1 の NFR4.1）、再試行が今と同じこと（FR2.5、U1 の NFR1.7）、制限なしなら速さが上限のまま変わらないこと（U1 の NFR1.1・NFR1.3）を、確かめることとして設計に入れる。
- **R-04（親 ID）**：引き下げを画面に出さない決めの由来は FR2.6、`min_rate=` は U1 の NFR8.3 の要約に欄を足すもの、と対応表に書く。

## Consolidated Summary Confirmation

- 下げ・戻し・枠の予約は、U1 の CallPacer の 1 つのロックの中で決める。下げた後に予約する枠は今の速さの間隔、予約済みの枠は動かさない
- 「上限」は U1 の値（API の上限の半分。東京 12 回）。戻してもそれを超えない
- 制限の知らせは EventFetcher の `counted_call` から送る。DescribeLogStreams の制限は送らない
- 要約の 1 行の欄は streams・calls・throttled・elapsed_ms・max_in_flight・min_rate・status。CallPacer の前に終わった取得はリージョンの上限を出す
- 確かめること：下げ方・戻し方の単体テスト（止めた時計）、制限が続くと間隔が広がり止むと戻る結合テスト、下限まで下げた状態で閉じるとすぐ止まる、再試行は今と同じ、制限なしなら上限のまま
- 新しい依存・API・画面・保存先は足さない

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
