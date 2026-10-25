# 業務の概要（Local Sights）

> 2026-10-20、コミット `3208713` の全面の読み直しを土台に、2026-10-25、コミット `793da17` の焦点を絞った読み直し（261025-insights-stats、Focused scan）で更新。根拠はコードと `README.md`・`docs/`。要件や設計の文書は参照していない。今回深く読んだ場所は `reverse-engineering-timestamp.md` の `analyzed.paths` にあり、それ以外の節の記述は前回のものを保っている。

## 目的と利用者

Local Sights は Amazon CloudWatch Logs を手元の PC で読むためのデスクトップアプリである（`README.md:1-7`）。利用者は AWS CLI 形式の設定（`~/.aws/config`）とプロファイルを持つ開発者・運用者で、マネジメントコンソールを開かずに、ロググループの一定期間のログを取り出して眺め、絞り込み、Logs Insights 風のクエリで調べる。会社の PC に配る前提で、macOS（Apple Silicon）と Windows 11（x64）向けのインストーラを GitHub Releases で配る（`docs/install.md`、`.github/workflows/release.yml`）。

アプリが AWS に対して行うのは読み取りの 3 操作（`DescribeLogGroups`、`DescribeLogStreams`、`GetLogEvents`）だけで、テレメトリやクラッシュレポートは送らない（`crates/local-sights-core/src/lib.rs:5-8`、`README.md:5-7`）。Logs Insights のクエリも、AWS 側の `StartQuery` を呼ぶのではなく、取得したログを手元で評価する（`README.md:178-182`）。

## 主な機能（今のコードにあるもの）

1. **接続の選択**：プロファイル（SDK 既定か名前つき）とリージョンを上部のバーで選ぶ。起動時は何も選ばず、選ぶとロググループの一覧を読む（`session.rs` の `select_profile` / `select_region` / `startup_listing`）。表示中のログがあるときは確認ダイアログを出す。リージョンは 2026-10-21 の持ち越し修正（コミット `5ced3bf`）からコードと名前（日英、例：`ap-northeast-1` アジアパシフィック (東京)）を並べ、文字で絞れる選択欄になった（`src/i18n/regions.ts`、`TopBar`）。
2. **ロググループの一覧と検索**：左のペインに一覧を出し、名前の一部で絞る（`log_groups/`、`LogGroupPane`）。
3. **期間を決めた取得**：開始と終了を日付（`yyyy/mm/dd`）と時刻（`hh:mm:ss`）で入れ、ローカルか UTC で解釈する。期間に関わるストリームをすべて列挙し、最大 8 本を同時に読み、ページを時刻順に合流させて表示する（`coordinator.rs`、`schedule.rs`、`timeline.rs`）。
4. **スロットリングと失敗への耐性**：リージョンごとの呼び出し上限（AWS の制限の半分）で速さを抑え、スロットルされたら半分に落として静かな時間で戻す（`pacing.rs`）。要求ごとに最大 5 回再試行し、使い切りが 3 本続いたら残りを諦める（`retry.rs`、`schedule.rs`）。失敗したストリームは一覧にし、残りは表示する。
5. **中断と閉じる**：取得中に［中断］で止める（未完了の本数を注意として出す）。取得中に窓を閉じるときは確認する（`retry.rs` の `StopReason`、`session.rs` の `request_cancel` / `request_close`）。
6. **経過時間**：取得の経過を mm:ss で出す（`session.rs` の `ElapsedView`、`src/elapsed.ts`）。
7. **絞り込み**：取得済みのログを、メッセージに含む文字（大文字小文字を無視）で手元で絞る。取得中に届くページも絞りながら出す（`filter.rs`、`log_view.rs`）。
8. **ディスクキャッシュ（既定は無効）**：成功した取得を利用者のフォルダに JSON Lines で保存し、同じ範囲を次に求めたとき AWS を呼ばずに出す。保存の有無とキャッシュの消去は設定ダイアログで行う（`cache/`、`SettingsDialog`）。
9. **SSO ログイン**：認証エラーのとき、選んだプロファイルが SSO につながるなら［Sign in with SSO］を出し、AWS CLI v2 の `aws sso login` をアプリから起動して、表示された URL を既定ブラウザで開く。トークンはアプリが読まない（`sso_login.rs`、`src-tauri/src/sso_cli/`、`browser.rs`）。
10. **Log Analytics（IB-11）**：左のナビゲーションで切り替える 2 つ目の画面。最大 50 のロググループと期間を選び、`fields` / `filter` / `sort` / `limit` のクエリを CodeEditor（Ace）で書いて実行する。取得は Log Management と別の保持（`QueryStore`）に積み、評価は手元で行う。対応する構文と非対応の構文は `README.md:176-210` の表のとおりで、`stats`・`parse`・`dedup` などの命令と `toMillis(@timestamp)` 以外の関数は「この版では対応していません」として読みの時点で止まる（`query/parser.rs`）。結果は取得と評価がすべて終わってから 1 回で出し、実行中は前回の結果を「前回の結果」の印つきで残す（`insights_session.rs`、`LogAnalyticsPage`）。
11. **大量の行の表示**：行を仮想スクロールで描き、100 万件でも滑らかに動くことを目標にしている（`LogTable`、`rowLayout.ts`）。行を開いてメッセージ全文を見る。2026-10-24（コミット `082dc92`）から、開いた行はメッセージの中の最初の完全な JSON オブジェクトか配列を整形して見せ、「整形／原文」で切り替えられる（`src/jsonPretty.ts`、`ExpandedMessage`）。整形は字句をそのまま保つ（`JSON.parse` を使わない）。
12. **多言語とアプリ内の言語切替**：画面の文言は日英の対訳表で持つ（`src/i18n/messages.ts`）。2026-10-23（コミット `bd65e49`）から、設定ダイアログで「OS の設定に合わせる」「日本語」「English」を選び、`settings.json` に保存して再起動なしに切り替える（`cache/settings.rs` の `LanguagePreference`、`App.tsx` の `resolveLocale`、Cloudscape の `I18nProvider`）。英語の文言は本数が 1 のとき単数形になる（`englishSingular`、`5ced3bf`）。
13. **書き出し（エクスポート）**：両方の画面の［Export］メニューから、表示中の行（Log Management は絞り込み後の行、Log Analytics はクエリの結果）を CSV か Markdown でファイルに保存するか、クリップボードにコピーする（`082dc92`。`export/`、`src-tauri/src/export_boundary.rs`、`ExportMenu`、`ExportNoticeAlert`）。保存は OS のダイアログで場所を選び、既定のファイル名はロググループ名と日時から作る。値はそのまま書く（式に見える値の注意は `README.md:35-39`）。書き出しは同時に 1 つで、結果は通知の Alert で知らせる。

## 業務上の言葉

| 言葉 | 意味（コードでの対応） |
|---|---|
| 接続（connection） | プロファイルとリージョンの組。変えると取得済みのログとクエリの結果を捨てる（`connection.rs`、`ConnectionState`） |
| 取得（fetch） | 1 つのロググループの 1 つの期間を読む 1 回の仕事。`fetch_number` で番号が付く（`session.rs`） |
| ストリーム（stream） | CloudWatch Logs のログストリーム。期間に関わるものだけを選ぶ（`streams/`） |
| タイムライン（timeline） | 取得したイベントの時刻順の列（`timeline.rs`） |
| 絞り込み（log filter） | 取得済みのログを手元で狭める条件（`filter.rs`） |
| キャッシュの鍵（cache key） | 接続・ロググループ・期間から決まる SHA-256 のファイル名（`cache/plan.rs`） |
| 保持（holdings） | クエリの画面が持つロググループごとのイベント。committed と staging に分かれ、`epoch` で世代を数える。実行の始めに staging と、選ばれなくなったロググループの保持を捨てる（`query/store.rs:296-310`） |
| 実行（run） | クエリの 1 回の実行。`run_id` で番号が付き、結果の行は `run_id` だけで引く（`insights_session.rs`、`5ced3bf` で `holdings_epoch` を廃止） |
| 結果の行（ResultRow） | 評価が終わったときに保持から複製した 1 イベントとそのロググループ名。セルの文字は表示のたびに項目から作る（`query/evaluator.rs:761-786`） |
| 言語の設定（LanguagePreference） | `system` / `ja` / `en`。言語そのものではなく「決め方」で、`system` は OS の言語に従う（`cache/settings.rs:28-40`） |
| 書き出し（export）／書き出しの仕事（ExportJob） | 画面・形式（CSV／Markdown）・行き先（ファイル／クリップボード）の組で頼む 1 回の書き出し。アプリに 1 つだけあり、`export_number` で番号が付く（`export/exporter.rs`） |
| 中断（Cancel）／閉じる（Close） | 取得を止める 2 つの理由。閉じるが常に強い（`retry.rs`） |

## この作業（insights-stats）から見た今の姿

v0.2.0 の公開（2026-10-20）のあと `main` に入ったのは、持ち越し Minor の一括修正（PR #36、`5ced3bf`）、アプリ内の言語切替（`bd65e49`）、CSV/Markdown の書き出しと JSON 整形（`082dc92`）、依存チェックの修正と文書（`a2dd083`）で、まだ配っていない（`docs/releasing.md:46-51`：次の版の番号はタグの直前に決める）。本作業は Log Analytics のクエリに `stats` による集計（`count(*)`、`sum`、`avg`、`min`、`max`、`bin()` と `by`）と、実行中の途中結果の表示（取得が進むにつれて表の行が増える）を足すもので、AWS の新しい API は呼ばない。今のコードでは `stats` はパーサで拒まれ、結果の行は「1 行 = 1 イベント」で固定され、評価は取得がすべて終わってから 1 回だけ走る。この 3 点が設計の起点になる（詳しくは `architecture.md` の「Improvement Opportunities」と `code-quality-assessment.md` の「この作業から見た構造上の制約」）。

## 前回（2026-10-20、コミット `3208713`）からの変化

構成の骨格（中核／アプリ／画面の 3 層、`CloudWatchLogsGateway` の境界、`AppSession` と `InsightsSession` の分担）は変わっていない。増えたのは、中核の `export` モジュール（28 番目の `pub mod`）、アプリの `export_boundary` モジュールと 2 つのコマンド（計 34）、2 つの Tauri プラグイン（ダイアログ、クリップボード）、画面の `ExportMenu`・`ExportNoticeAlert`・`jsonPretty.ts`・`i18n/regions.ts`、設定ファイルの `language` 項目である。前回の「技術的な負債」の 5 項目（不安定な結合テスト、QueryStore の 3 点、記述の古さ、画面の小物、リージョンの選択欄）は `5ced3bf` で直った。
