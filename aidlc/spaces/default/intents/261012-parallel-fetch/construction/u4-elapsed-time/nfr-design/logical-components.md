# Logical Components — U4 かかった時間（u4-elapsed-time）

上流：`construction/u4-elapsed-time/nfr-requirements/`（NFR2.2〜NFR9.2、レビュー 2 回目の新しい指摘）、`construction/u4-elapsed-time/functional-design/functional-spec.md`（BR4.1〜BR4.10）、`inception/contract-design/contract-summary.md`（C5）、今のコード（`crates/local-sights-core/src/session.rs`、`src/api.ts`、`src/App.tsx`、`src/components/StatusLine.tsx`、`src/i18n/messages.ts`）。質問票は `nfr-design-questions.md`。

| 部品 | 場所 | 変更 | 持つ要件 |
|------|------|------|----------|
| SessionClock | `crates/local-sights-core/src/session.rs`（または同じクレートの小さなモジュール） | 新規。「今」を返す差し替えられる時計 | NFR6.2、NFR6.8 |
| AppSession | `session.rs` | 変更。`elapsed` の記録と、begin_fetch・finish_fetch・abort_fetch_with_failure・apply_connection_change での書き換え、`with_clock` | BR4.1〜BR4.5、NFR6.4、NFR6.10、NFR6.11 |
| SessionView / ElapsedView | `session.rs` | 変更。`elapsed` を足す | NFR8.10 |
| TauriApp | `src-tauri/src/lib.rs` | 変更なし（SessionView をそのまま送る） | NFR5.4 |
| ApiBridge | `src/api.ts` | 変更。`ElapsedView` 型と `SessionView.elapsed` | C5 |
| 経過の純粋な関数 | `src/elapsed.ts` | 新規。`formatElapsed`・`nextAnchor`・`shownMs`・`msUntilNextSecond` | NFR6.3、NFR6.6、NFR6.12 |
| ScreenHooks（useElapsed） | `src/hooks/useElapsed.ts` | 新規 | NFR6.5、NFR6.9 |
| ElapsedTime | `src/components/ElapsedTime.tsx` | 新規。経過だけを描く | NFR5.3、NFR9.2 |
| StatusLine | `src/components/StatusLine.tsx` | 変更。ElapsedTime を置く | BR4.8 |
| MessageCatalog | `src/i18n/messages.ts` | 変更。`status.elapsed` | NFR9.1 |

## 境目と影響の範囲

- 失敗が広がる範囲：経過の部品だけ。経過の計算はエラーを返さず、取得の流れ（FetchCoordinator 以下）には触れない
- 共有するもの：AppSession の中の記録（今と同じ Mutex で守られる）
- インフラの部品：なし（ローカルのデスクトップアプリ）

## Assumptions & Open Questions

None.
