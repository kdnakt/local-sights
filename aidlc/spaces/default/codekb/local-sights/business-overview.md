# 業務の概要（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直し。根拠はコードと `README.md`・`docs/`。要件や設計の文書は参照していない。

## 目的と利用者

Local Sights は Amazon CloudWatch Logs を手元の PC で読むためのデスクトップアプリである（`README.md:1-7`）。利用者は AWS CLI 形式の設定（`~/.aws/config`）とプロファイルを持つ開発者・運用者で、マネジメントコンソールを開かずに、ロググループの一定期間のログを取り出して眺め、絞り込み、Logs Insights 風のクエリで調べる。会社の PC に配る前提で、macOS（Apple Silicon）と Windows 11（x64）向けのインストーラを GitHub Releases で配る（`docs/install.md`、`.github/workflows/release.yml`）。

アプリが AWS に対して行うのは読み取りの 3 操作（`DescribeLogGroups`、`DescribeLogStreams`、`GetLogEvents`）だけで、テレメトリやクラッシュレポートは送らない（`crates/local-sights-core/src/gateway/mod.rs:22-26`、`README.md:5-7`）。Logs Insights のクエリも、AWS 側の `StartQuery` を呼ぶのではなく、取得したログを手元で評価する（`README.md:170-173`）。

## 主な機能（今のコードにあるもの）

1. **接続の選択**：プロファイル（SDK 既定か名前つき）とリージョンを上部のバーで選ぶ。起動時は何も選ばず、選ぶとロググループの一覧を読む（`session.rs` の `select_profile` / `select_region` / `startup_listing`）。表示中のログがあるときは確認ダイアログを出す。
2. **ロググループの一覧と検索**：左のペインに一覧を出し、名前の一部で絞る（`log_groups/`、`LogGroupPane`）。
3. **期間を決めた取得**：開始と終了を日付（`yyyy/mm/dd`）と時刻（`hh:mm:ss`）で入れ、ローカルか UTC で解釈する。期間に関わるストリームをすべて列挙し、最大 8 本を同時に読み、ページを時刻順に合流させて表示する（`coordinator.rs`、`schedule.rs`、`timeline.rs`）。
4. **スロットリングと失敗への耐性**：リージョンごとの呼び出し上限（AWS の制限の半分）で速さを抑え、スロットルされたら半分に落として静かな時間で戻す（`pacing.rs`）。要求ごとに最大 5 回再試行し、使い切りが 3 本続いたら残りを諦める（`retry.rs`、`schedule.rs`）。失敗したストリームは一覧にし、残りは表示する。
5. **中断と閉じる**：取得中に［中断］で止める（未完了の本数を注意として出す）。取得中に窓を閉じるときは確認する（`retry.rs` の `StopReason`、`session.rs` の `request_cancel` / `request_close`）。
6. **経過時間**：取得の経過を mm:ss で出す（`session.rs` の `ElapsedView`、`src/elapsed.ts`）。
7. **絞り込み**：取得済みのログを、メッセージに含む文字（大文字小文字を無視）で手元で絞る。取得中に届くページも絞りながら出す（`filter.rs`、`log_view.rs`）。
8. **ディスクキャッシュ（既定は無効）**：成功した取得を利用者のフォルダに JSON Lines で保存し、同じ範囲を次に求めたとき AWS を呼ばずに出す。保存の有無とキャッシュの消去は設定ダイアログで行う（`cache/`、`SettingsDialog`）。
9. **SSO ログイン**：認証エラーのとき、選んだプロファイルが SSO につながるなら［Sign in with SSO］を出し、AWS CLI v2 の `aws sso login` をアプリから起動して、表示された URL を既定ブラウザで開く。トークンはアプリが読まない（`sso_login.rs`、`src-tauri/src/sso_cli/`、`browser.rs`）。
10. **Log Analytics（IB-11）**：左のナビゲーションで切り替える 2 つ目の画面。最大 50 のロググループと期間を選び、`fields` / `filter` / `sort` / `limit` のクエリを CodeEditor（Ace）で書いて実行する。取得は Log Management と別の保持（`QueryStore`）に積み、評価は手元で行う。対応する構文と非対応の構文は `README.md:168-203` の表のとおり。
11. **大量の行の表示**：行を仮想スクロールで描き、100 万件でも滑らかに動くことを目標にしている（`LogTable`、`rowLayout.ts`）。行を開いてメッセージ全文を見る。
12. **多言語**：画面の文言は日英の対訳表で持ち、ブラウザの言語で選ぶ（`src/i18n/messages.ts`）。

## 業務上の言葉

| 言葉 | 意味（コードでの対応） |
|---|---|
| 接続（connection） | プロファイルとリージョンの組。変えると取得済みのログとクエリの結果を捨てる（`connection.rs`、`ConnectionState`） |
| 取得（fetch） | 1 つのロググループの 1 つの期間を読む 1 回の仕事。`fetch_number` で番号が付く（`session.rs:1557`） |
| ストリーム（stream） | CloudWatch Logs のログストリーム。期間に関わるものだけを選ぶ（`streams/`） |
| タイムライン（timeline） | 取得したイベントの時刻順の列（`timeline.rs`） |
| 絞り込み（log filter） | 取得済みのログを手元で狭める条件（`filter.rs`） |
| キャッシュの鍵（cache key） | 接続・ロググループ・期間から決まる SHA-256 のファイル名（`cache/plan.rs`） |
| 保持（holdings） | クエリの画面が持つロググループごとのイベント。committed と staging に分かれ、`holdings_epoch` で世代を数える（`query/store.rs`） |
| 実行（run） | クエリの 1 回の実行。`run_id` で番号が付く（`insights_session.rs`） |
| 中断（Cancel）／閉じる（Close） | 取得を止める 2 つの理由。閉じるが常に強い（`retry.rs:192-214`） |

## この作業（carryover-fixes）から見た今の姿

v0.2.0（取得の高速化・Windows 版・IB-11 を含む）は 2026-10-20 に公開済みである（`git tag`、依頼文）。本作業は新しい振る舞いを足すのではなく、過去の作業単位で持ち越した小さな指摘を直すものなので、業務上の機能は変わらない。関わる場所は `code-quality-assessment.md` の「技術的な負債」に、依頼の 5 項目として証拠つきで列挙した。

## 前回（2026-10-17、コミット `2760b13`）からの変化

前回の codekb のあとに `main` に入ったのは、IB-11 の U3（複数ロググループと保持）・U4（互換の確かめ）の実装と、互換の確かめの記録（PR #35、コミット `3208713`）、Dependabot の更新（PR #30、#32）である。構成の骨格（中核／アプリ／画面の 3 層、`CloudWatchLogsGateway` の境界、`AppSession` と `InsightsSession` の分担）は変わっていない。今回は要件文書ではなくコードだけから書き直したため、前回の文書とは記述の切り方が違う。
