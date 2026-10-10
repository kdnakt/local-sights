# Initiative Brief — CloudWatch Logs ローカルビューア

アイデア整理フェーズのまとめ。上流の成果物は `intent-statement`、`stakeholder-map`、`feasibility-assessment`、`constraint-register`、RAID ログ、`scope-document`、`intent-backlog`、`wireframes`、`user-flow`（いずれも `aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/` 配下）。質問票の回答は `approval-handoff-questions.md` の Q1〜Q5 を指す。

**この企画書の範囲とバックログ（下の「統合版」）が、以後の正となる**（Q1）。承認済みの `scope-document` と `intent-backlog` は書き換えず、画面設計で決まった事項はここで統合する。

## 判断

**Go**（Q5）。MVP の完成目標日は **2026-10-10**（Q4）。

## 意図と課題

- マネジメントコンソール（FilterLogEvents）では古いロググループの過去ログが見えないが、CLI の get-log-events では見えた。ログ削除の記録はなく、原因は未特定（intent-statement）。
- 古いロググループの過去ログを確実に閲覧できる、ローカルで動く Rust 製クライアントアプリを作る。取得には GetLogEvents API を使う（intent-statement）。
- 利用者は開発者本人で、OSS として公開する。スコープの決定者は開発者本人だけ（stakeholder-map）。

## 市場・チーム

- 市場調査とチーム編成は実施していない。個人の実務の困りごとから始めたツールで、開発者は 1 人のため（stakeholder-map）。

## 実現性とリスク

- **条件付きで実現可能**。GetLogEvents はストリーム単位・毎秒 25 回まで（一部リージョンは 10 回）で、引き上げはできない。対象はストリーム数千以上・数百万件以上なので、MVP は選んだ範囲だけを取得する（feasibility-assessment、constraint-register）。
- 次のリスクは、対策を前提に受け入れた（Q2）。

| リスク | 対策 | 出典 |
|--------|------|------|
| 他の古いロググループでは GetLogEvents でも過去ログが見えない可能性 | 観測できた事象を前提に進め、見えない例が出たら原因調査を別途検討する | feasibility の R-01 |
| 時間範囲が広いと、取得中に何も表示されない時間が数分以上続く | 「取得中」の表示と注意書きを出す。MVP 後の最優先項目（横断取得の高速化）で改善する | scope-document、wireframes |
| 「1 週間で作れる」は、GUI・macOS のみ・見た目最小限を前提とした仮説 | 収まらない場合の削減順を決めてある（ディスクキャッシュ → 設定ダイアログ → タイムゾーン切替） | wireframes |
| API 料金を公式価格表でまだ確認していない | 要件分析までに確認する | feasibility の I-01 |

## 統合版の範囲（以後の正）

### MVP に含めるもの

| # | できること | 出典 |
|---|------------|------|
| 1 | デスクトップ GUI アプリとして動く。MVP の対応 OS は macOS のみ。見た目は標準部品だけの最小限 | wireframes（Q1、Q7、Q8） |
| 2 | AWS のプロファイルとリージョンを切り替える | scope-document |
| 3 | 選んだ権限で取得できるロググループを一覧から 1 つ選ぶ | scope-document |
| 4 | 時間範囲を絶対日時（年月日・時分秒）で指定する。表示と入力のタイムゾーンを切り替えられる（既定はローカル）。切り替えても、入力済みの日時は同じ瞬間を指したまま | scope-document、wireframes |
| 5 | 指定した時間範囲について、ストリームを跨いだ結果を時刻順に表示する。上限は設けない。取得中は条件を変更できず、取得中にウィンドウを閉じるときは確認を出す | scope-document、wireframes |
| 6 | 表示中のログを文字列の部分一致で絞り込む | scope-document |
| 7 | 取得したログのディスクキャッシュ（任意・既定は無効）と、キャッシュの削除 | scope-document、wireframes |

取得では、終了時刻を必ず指定し、ページの終わりを正しく判定し、スロットリング時は待ってから再試行する（feasibility-assessment）。

### MVP の後

1. ロググループ全体の横断取得の高速化（最優先）
2. Logs Insights のクエリ構文と互換のクエリ
3. その他：取得の進捗表示と中断、ファイル書き出し、JSON 整形、**Windows 対応**（Linux は未定）

### 対象外

リアルタイムの追従表示（tail）、ログの変更操作、Logs Insights／FilterLogEvents API の利用、Web 版・サーバー版、過去ログが見えなかった原因の調査（scope-document、constraint-register）。

## 統合版のバックログ（以後の正）

| ID | 項目 | MoSCoW | 変更点 | 出典 |
|----|------|--------|--------|------|
| IB-00 | デスクトップ GUI の土台（macOS のみ、標準部品だけ） | Must | 新規 | wireframes |
| IB-01 | プロファイルとリージョンの切り替え | Must | なし | intent-backlog |
| IB-02 | ロググループを一覧から 1 つ選ぶ | Must | なし | intent-backlog |
| IB-03 | 時間範囲の指定（年月日・時分秒、タイムゾーン切替、切替時も同じ瞬間を保つ） | Must | 入力形式とタイムゾーンを明確化 | intent-backlog、wireframes |
| IB-04 | ストリームを跨いだ時刻順表示（上限なし、待機と再試行、取得中は条件を変更不可、取得中の終了は確認） | Must | 操作ロックと終了確認を追加 | intent-backlog、wireframes |
| IB-05 | 部分一致フィルター | Must | なし | intent-backlog |
| IB-06 | ディスクキャッシュ（任意・既定は無効）とキャッシュ削除 | Should | キャッシュ削除を追加 | intent-backlog、wireframes |
| IB-07 | ロググループ全体の横断取得の高速化 | Could（MVP 後の最優先） | なし | intent-backlog |
| IB-08 | 取得の進捗表示と中断 | Could | なし | intent-backlog |
| IB-09 | ファイル書き出し | Could | なし | intent-backlog |
| IB-10 | JSON 整形表示 | Could | なし | intent-backlog |
| IB-11 | Logs Insights 互換クエリ | Could（IB-07 の次） | なし | intent-backlog |
| IB-16 | Windows 対応 | Could | 新規（MVP では見送り） | wireframes（Q8） |
| IB-12〜IB-15 | tail、変更操作、Insights／FilterLogEvents API の利用、Web 版・サーバー版 | Won't | なし | intent-backlog |

**作る順番（MVP）**：IB-00 → IB-01 → IB-02 → IB-03 → IB-04 → IB-05 → IB-06。期限が厳しいときは IB-06 から外す（wireframes の「期限と削減順」）。

## 画面の概要

1 画面構成。上部バーにプロファイル・リージョン・タイムゾーン・設定、左にロググループ一覧、右に条件エリア・ログ一覧・ステータス行を置く。画面は 7 つ：起動直後、取得中、取得完了、絞り込み中、該当なし・エラー・入力の誤り、設定、取得中の終了確認。主な流れは 6 回の操作で結果が見られる（wireframes、user-flow）。ラフな画面設計は、思い描く形と合っていると確認済み（Q3）。

## 次のフェーズに引き継ぐ未解決事項

| 事項 | 決めるステージ | 出典 |
|------|----------------|------|
| 「実用的な時間」の数値目標 | 要件分析 | intent-statement |
| 「CLI より少ない操作」の比較基準 | 要件分析 | intent-statement |
| API 料金の確認 | 要件分析まで | feasibility の I-01 |
| アクセシビリティ要件（キーボード操作が必要かどうかなど） | 要件分析 | wireframes のレビュー（R-01 は受け入れ済みのリスク） |
| 最小ウィンドウサイズ、列幅、ダークモード、画面の表示言語 | 要件分析 | wireframes |
| 長いメッセージの見せ方、再取得時にフィルターを引き継ぐか、キャッシュで「同じ条件」とみなす判定方法 | 要件分析 | wireframes、user-flow |
| GUI フレームワークなどの技術選定 | 開発ルール確認以降 | wireframes（Q8） |
