# 業務の概要（Local Sights）

走査したコミット：`2760b13b8bc5b6daba36a6c283b3958c974a5a2d`（2026-10-17）。intent 261017-insights-query（IB-11「Logs Insights 互換のクエリ」）のための全面走査で書き直した。

## 目的と利用者

Local Sights は、Amazon CloudWatch Logs のログを手元の PC に取ってきて、時刻順に一覧し、手元で絞り込むためのデスクトップアプリである（`README.md`）。会社の開発者が、マネジメントコンソールより速く・気軽にログを読むことを目的にしている。配る先は macOS（Apple Silicon）と Windows 11（x64）の会社の PC で、GitHub Releases の `.dmg` と NSIS のインストーラで入れる（team.md Deployment、`docs/install.md`）。

利用者自身が決めた前提（project.md Corrections）：

- Rust 製であること、ログの取得は `GetLogEvents` で行うこと。
- 呼んでよい AWS API は CloudWatch Logs の読み取り 3 つ（`DescribeLogGroups`・`DescribeLogStreams`・`GetLogEvents`）だけ。Logs Insights の API（`StartQuery` など）や `FilterLogEvents` は呼べない。
- 秘密の認証情報とアクセスキー ID は、リポジトリ・ログ・画面・キャッシュに書かない。テレメトリやクラッシュレポートは送らない。
- 自動テストと CI からは AWS につながない。

## 主な機能（今のコードにあるもの）

| 機能 | 内容 | 主な場所 |
|------|------|----------|
| 接続の選択 | AWS の設定ファイルからプロファイル一覧、内蔵のリージョン一覧。変えると表示中のログを捨てる（確認ダイアログあり） | `catalog/`、`connection.rs`、`TopBar.tsx` |
| ロググループの一覧 | `DescribeLogGroups` を全ページ読み、名前で手元の絞り込み。1 つだけ選ぶ | `log_groups/`、`LogGroupPane.tsx` |
| 取得 | 選んだロググループの、開始〜終了（ローカル時刻か UTC、秒単位、終了は 999 ms まで含む）に関わるストリームを `DescribeLogStreams` で選び、最大 8 本を同時に `GetLogEvents` で読む。スロットリングと通信失敗は再試行し、失敗したストリームは一覧に出して残りを表示する | `coordinator.rs`、`fetcher.rs`、`streams/`、`schedule.rs`、`pacing.rs`、`retry.rs` |
| 一覧の表示 | 「時刻／ストリーム／メッセージ」の 3 列。見えている行だけを中核から読み、100 万件でも滑らかに動く。行を開くとメッセージ全体を出す（JSON は整形しない） | `timeline.rs`、`LogTable.tsx`、`ExpandedMessage.tsx` |
| 絞り込み（FR6） | メッセージに文字列を含むか（大文字小文字を区別しない）で、取得済みのイベントを手元で絞る。AWS は呼ばない。取得中でも使え、次の取得でも文字列は残る | `filter.rs`、`log_view.rs`、`LogFilterInput.tsx` |
| ディスクキャッシュ | 既定は切。全ストリームが成功した取得を（プロファイル, リージョン, ロググループ）ごとに保存し、要求範囲を 1 つの記録範囲が含むときは AWS を呼ばずに出す | `cache/` |
| SSO ログイン | 認証エラーのとき、利用者の操作で AWS CLI の `aws sso login` を起動する。アプリ自身は SSO の API を呼ばずトークンも扱わない | `sso_login.rs`、`src-tauri/src/sso_cli/` |
| 取り消し・経過時間 | 取得中の [Cancel]、取得にかかった時間の表示 | `session.rs`、`ElapsedTime.tsx`、`CancelNotice.tsx` |
| 閉じる確認 | 取得中やキャッシュの書き込み中に閉じるときは確認する | `session.rs`、`CloseConfirmDialog.tsx` |

## 業務上の言葉

- **ロググループ／ストリーム／イベント**：CloudWatch Logs の言葉どおり。イベントは時刻・取り込み時刻・メッセージ・ストリーム名を持ち、アプリがストリーム内の順番（`sequence`、0 から）を振る。
- **タイムライン**：今の取得で集めた全イベントの、時刻順の並び。並びキーは `(timestamp, logStreamName, sequence)`。
- **絞り込み（ログフィルタ）**：タイムラインに対する文字列の部分一致。ロググループの一覧の名前の絞り込みとは別物。
- **ジョブ**：1 回の取得。状態は Running・Completed・CompletedWithFailures・Failed・Aborted・Cancelled。

## IB-11 から見た今の姿

- 「クエリ」に当たるものは、メッセージの部分一致 1 つだけである。条件の組み合わせ・項目の選択・並べ替え・件数の上限・JSON の項目の取り出しは無い。
- 期間とロググループは画面の条件欄で決め、ロググループは 1 つに限られる。取得のたびに保持中のログを捨ててロググループ全体を取り直すので、「足りない分だけ取り足す」流れはまだ無い。
- 速さの基準「10 万件 10 秒・100 万件 100 秒」は、今の絞り込み（U5 の NFR1・NFR2）でも同じ数字が使われており、測るテストがある。

## 前回（2026-10-12、コミット `9d08cbf`）からの変化

- 取得の高速化（PR #22）：同時取得（最大 8 本）、呼び出しの速さの上限と自動の引き下げ（`pacing.rs`）、あきらめの決まりの `StreamSchedule` への移動、経過時間、[Cancel]。
- Windows 11（x64）対応（PR #23・#24）：AWS CLI の起動と停止（ジョブオブジェクト）、既定のブラウザ（`ShellExecuteW`）、NSIS のインストーラ、両 OS を 1 つの Release に載せる流れ、版 `0.2.0`。
- SSO ログインの結果ダイアログからの再ログイン（PR #21）、行カバレッジ 80% の CI の止め（PR #20）。
