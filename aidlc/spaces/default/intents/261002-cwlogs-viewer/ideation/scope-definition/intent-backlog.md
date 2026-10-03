# Intent Backlog — CloudWatch Logs ローカルビューア

上流の成果物：`intent-statement`、`feasibility-assessment`、`constraint-register`（いずれも `aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/` 配下）。質問票の回答は `scope-definition-questions.md` の Q1〜Q7 を指す。

## 優先順位づけの方法

- **MoSCoW** で MVP の内外を分ける。Must は MVP に必須、Should は MVP に含めるが削っても成立するもの、Could は MVP 後の候補、Won't は今回のワークフローでは作らないもの。
- **並べ方は「価値優先」**。開発者 1 人・期限 1 週間（constraint-register C-O1、C-O2）なので、利用者が実際に過去ログを見られる流れを最初に通し、そこに機能を足していく（delivery lead の視点）。

## バックログ

| ID | 項目 | MoSCoW | 依存 | 出典 |
|----|------|--------|------|------|
| IB-01 | AWS のプロファイルとリージョンを切り替える | Must | なし | Q1、constraint-register C-E1、C-E2 |
| IB-02 | 選んだ権限で取得できるロググループを一覧から 1 つ選ぶ | Must | IB-01 | Q1 |
| IB-03 | 時間範囲（開始・終了）を指定する | Must | なし | Q1 |
| IB-04 | 指定した時間範囲について、ストリームを跨いだ結果を時刻順に表示する（上限なし、待機・再試行あり） | Must | IB-02、IB-03 | Q1、Q7、feasibility-assessment |
| IB-05 | 表示中のログを文字列の部分一致で絞り込む | Must | IB-04 | Q2 |
| IB-06 | 取得済みログのディスクキャッシュ（任意・既定は無効） | Should | IB-04 | Q3、constraint-register C-R1 |
| IB-07 | ロググループ全体の横断取得の高速化（並列化、キャッシュ活用など） | Could（MVP 後の最優先） | IB-04、IB-06 | Q4、feasibility-assessment |
| IB-08 | 取得の進捗表示と中断 | Could | IB-04 | Q3（MVP に含めず） |
| IB-09 | 表示中の結果をファイルに書き出す | Could | IB-04 | Q3（MVP に含めず） |
| IB-10 | JSON 形式のログメッセージを整形して表示する | Could | IB-04 | Q3（MVP に含めず） |
| IB-11 | Logs Insights 互換のクエリ | Could（IB-07 の次） | IB-04 | Q5、intent-statement |
| IB-12 | リアルタイムの追従表示（tail） | Won't | — | Q6 |
| IB-13 | ログの変更操作（書き込み・削除・保持期間変更） | Won't | — | Q6 |
| IB-14 | Logs Insights／FilterLogEvents API の利用 | Won't | — | Q6 |
| IB-15 | Web 版・サーバー版 | Won't | — | Q6 |

## MVP の進め方（delivery lead の視点）

1. **最初に通す流れ**：IB-01 → IB-02 → IB-03 → IB-04。これで「古いロググループの過去ログが見られる」という一番の価値（intent-statement）が成立する。
2. **次に足すもの**：IB-05（部分一致フィルタ）。
3. **最後に足すもの**：IB-06（任意のディスクキャッシュ）。期限が厳しい場合、削っても MVP として成立する唯一の項目。

## MVP 後の進め方

IB-07（横断取得の高速化）→ IB-11（Logs Insights 互換クエリ）の順（Q4）。IB-08〜IB-10 は、IB-07 の作業中に必要性が高いものから取り込む。特に IB-08（進捗表示と中断）は、広い時間範囲の取得で待ち時間が長くなる問題（scope-document の「受け入れたこと」）と関係が深い。

## 未解決事項

- 「CLI より少ない操作」の比較基準と「実用的な時間」の数値は、要件分析で決める（intent-statement）。
- CloudWatch Logs の API 料金は、要件分析までに公式価格表で確認する（feasibility-assessment）。
