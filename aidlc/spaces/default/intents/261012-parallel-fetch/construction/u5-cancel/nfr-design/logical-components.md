# Logical Components — U5 中断（u5-cancel）

上流：`construction/u5-cancel/nfr-requirements/`（NFR4.2〜NFR10.1、追加の質問 Q1）、`construction/u5-cancel/functional-design/`（BR5.1〜BR5.14。BR5.3・BR5.4 の応答待ちと 1.2・1.4 は非機能要件で置き換え済み）、`inception/contract-design/contract-summary.md`（C3〜C7）、今のコード。質問票は `nfr-design-questions.md`。

| 部品 | 場所 | 変更 | 持つ要件 |
|------|------|------|----------|
| FetchStatus | `fetch_status.rs` | Cancelled を足す | C3 |
| StreamSchedule | `schedule.rs` | 変更なし（outcome の unfinished_streams を N に使う） | BR5.4、NFR4.6 |
| FetchCoordinator | `coordinator.rs` | 打ち切り、decide_job_status の入力、FetchJob の 2 項目、serve_from_cache の理由の分岐 | BR5.3、BR5.5、NFR4.2〜NFR4.5 |
| CachePlan | `cache/plan.rs` | 網羅の検査の追従だけ（書く決まりは変えない） | BR5.7 |
| FetchStats | `fetch_stats.rs` | 変更なし（status の語が増えるだけ） | NFR2.5 |
| AppSession | `session.rs` | cancelling、cancel_notice、request_cancel、SessionView の 3 項目 | BR5.1、BR5.2、BR5.8、BR5.9 |
| RecordingSink（テスト） | `tests/support/` | Cancelled・N・一覧の前の印を記録して確かめる | BR5.14 |
| FetchCheck | `examples/fetch_check.rs` | 終わりの状態の分岐に Cancelled | BR5.14 |
| TauriApp | `src-tauri/src/lib.rs`、`build.rs`、`capabilities/default.json` | cancel_fetch、持ち手の登録の順 | NFR6.17、NFR8.15 |
| ApiBridge | `src/api.ts` | 型と cancelFetch | C5、C6 |
| ConditionArea | `src/components/ConditionArea.tsx` | ボタンの 4 つの見た目、R-06 のフォーカスの effect の条件 | BR5.10、NFR10.1 |
| CancelNotice | `src/components/CancelNotice.tsx`（新規） | 中断の注意 | BR5.11、NFR9.4 |
| AppShell | `src/App.tsx` | 注意の置き場所 | BR5.11 |
| StatusLine | `src/components/StatusLine.tsx` | 「中断」の表示 | BR5.12 |
| MessageCatalog | `src/i18n/messages.ts` | 文言 | BR5.13、NFR9.3 |

## 境目と影響の範囲

- 打ち切りの影響は取得 1 回のタスクの中だけ。取得 1 回ごとに止める信号を作るので、次の取得に持ち越さない
- 共有するもの：AppSession（Mutex）と AppState.abort（Mutex）。ロックの順は session → abort に固定
- インフラの部品：なし（ローカルのデスクトップアプリ）

## Assumptions & Open Questions

None.
