# Decision Log — アイデア整理フェーズ

アイデア整理フェーズで決まったことの記録。各行の出典は、各ステージの質問票の回答番号（例：`intent-capture Q1`）と、その結果を書いた成果物を指す。上流の成果物は `intent-statement`、`stakeholder-map`、`feasibility-assessment`、`constraint-register`、`scope-document`、`intent-backlog`、`wireframes`（いずれも `aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/` 配下）。

## Decisions

| # | 決定 | ステージ | 出典 |
|---|------|----------|------|
| D-01 | 一番の課題は、マネコンで見えない古いロググループの過去ログを確実に閲覧できること | Intent Capture | intent-capture Q1、intent-statement |
| D-02 | 利用者は開発者本人で、OSS として公開する。スコープの決定者は本人だけ。進捗共有は PR・README 程度 | Intent Capture | intent-capture Q2、Q5〜Q7、stakeholder-map |
| D-03 | ローカルで動くクライアントアプリで、AWS へのデプロイはしない | Intent Capture | intent-capture Q8、intent-statement |
| D-04 | 観測した事象（マネコンでは見えず、CLI の get-log-events では見えた）を確認済みの事実として扱う。原因は未特定 | Intent Capture | intent-capture Q9、intent-statement |
| D-05 | 「実用的な時間」の数値目標は要件分析で決める（前提として受け入れ） | Intent Capture | intent-capture の Assumption Confirmation |
| D-06 | 対象は複数アカウント・複数リージョン。認証は SSO、アクセスキー、AssumeRole、環境変数のすべて | Feasibility | feasibility Q1、Q2、constraint-register |
| D-07 | ログに機密情報が含まれうるため、ディスク保存は利用者が選んだ場合のみ | Feasibility | feasibility Q3、constraint-register |
| D-08 | 原因調査は開発に含めない | Feasibility | feasibility Q8、constraint-register |
| D-09 | MVP では、選んだロググループ・時間範囲だけを取得する。ロググループ全体の横断取得は MVP 後 | Feasibility | feasibility Q9、feasibility-assessment |
| D-10 | MVP の流れ：プロファイル・リージョン切替 → ロググループを 1 つ選ぶ → 時間範囲を指定 → ストリームを跨いで表示 | Scope Definition | scope-definition Q1、scope-document |
| D-11 | MVP のフィルターは部分一致のみ。任意機能はディスクキャッシュだけ | Scope Definition | scope-definition Q2、Q3、scope-document |
| D-12 | MVP の後は、横断取得の高速化 → Logs Insights 互換クエリの順で進める | Scope Definition | scope-definition Q4、Q5、intent-backlog |
| D-13 | 対象外：tail、変更操作、Insights／FilterLogEvents API の利用、Web 版・サーバー版 | Scope Definition | scope-definition Q6、scope-document |
| D-14 | 時間範囲が広くても上限は設けず、すべて取得する | Scope Definition | scope-definition Q7、scope-document |
| D-15 | アプリの形はデスクトップ GUI。見た目は標準部品だけの最小限 | Rough Mockups | rough-mockups Q1、Q7、wireframes |
| D-16 | MVP の対応 OS は macOS のみ。Windows は MVP 後（Q2 の「macOS と Windows」を置き換え） | Rough Mockups | rough-mockups Q8、wireframes |
| D-17 | 1 画面構成。時間範囲は絶対日時（年月日・時分秒）。タイムゾーンは切替可能（既定はローカル）で、切り替えても同じ瞬間を保つ | Rough Mockups | rough-mockups Q3〜Q5、Q9、Q10、wireframes |
| D-18 | 取得中は条件を変更できない。取得中にウィンドウを閉じるときは確認を出す | Rough Mockups | rough-mockups Q11、Q12、wireframes |
| D-19 | 設定ダイアログはキャッシュの有効・無効とキャッシュ削除だけ | Rough Mockups | rough-mockups Q13、wireframes |
| D-20 | アクセシビリティ注記の不足（レビュー R-01）は、直さずに受け入れる。要件分析で要件を決める | Rough Mockups | rough-mockups のレビュー（人間の判断） |
| D-21 | 画面設計で決まった事項は企画書に統合し、以後は企画書を正とする。承認済みの文書は書き換えない | Approval & Handoff | approval-handoff Q1、initiative-brief |
| D-22 | 主なリスク 4 つを、対策を前提に受け入れる | Approval & Handoff | approval-handoff Q2、initiative-brief |
| D-23 | MVP の完成目標日は 2026-10-10。判断は Go | Approval & Handoff | approval-handoff Q4、Q5、initiative-brief |

## Deferred Decisions

| 事項 | 先送り先 | 出典 |
|------|----------|------|
| 「実用的な時間」と「CLI より少ない操作」の基準 | 要件分析 | intent-statement |
| API 料金の確認 | 要件分析まで | feasibility-assessment |
| アクセシビリティ、ウィンドウサイズ、ダークモード、表示言語 | 要件分析 | wireframes |
| 技術選定（GUI フレームワークなど） | 開発ルール確認以降 | wireframes |
