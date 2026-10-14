# Reliability Design — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

設計の正本は `security-design.md` の 2 節。

| 要件 | 設計 |
|------|------|
| NFR6.5 | 画面の時計は `performance.now()` と自分の予約だけで進み、ライブラリから何も届かない間（再試行の待ち）も止まらない |
| NFR6.6 | `nextAnchor` の保持は「previous が running かつ incoming が running」のときだけ。新しい取得の始まり・running false・null では受け取った値に置き換える |
| NFR6.7 | 合わせ直しのたびに起点をライブラリの値（保持したときは表示の値）にする。保持した値は次の合わせ直しまでにライブラリの値に追い越されるので、ずれは送受信の遅れ（1 秒未満）に収まる。PC のスリープをまたいだ取得は対象外で、終わりの値はライブラリのもの |
| NFR6.8 | ライブラリは `saturating_duration_since` で差を取り負にならない。画面は `max(0, now − receivedAt)` |
| NFR6.9 | 合わせ直しは `elapsed` のオブジェクトの入れ替わりで見分ける（合成では入れ替わらない） |
| NFR6.10 | `elapsed` を None にする場所は、Aborted の終わりと `apply_connection_change` だけ。`select_log_group` は触らない |
| NFR6.11 | `abort_fetch_with_failure` でも final_ms を決める |

## 失敗の扱い

経過の計算に失敗する道はない（時計の読み取りと引き算だけで、エラーを返さない）。`unwrap`・`expect` は使わない。

## Assumptions & Open Questions

None.
