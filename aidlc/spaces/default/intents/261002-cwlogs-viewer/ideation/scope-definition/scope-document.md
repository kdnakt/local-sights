# Scope Document — CloudWatch Logs ローカルビューア

上流の成果物：
- `intent-statement`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/intent-capture/intent-statement.md`）
- `feasibility-assessment`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/feasibility/feasibility-assessment.md`）
- `constraint-register`（`aidlc/spaces/default/intents/261002-cwlogs-viewer/ideation/feasibility/constraint-register.md`）

質問票の回答は `scope-definition-questions.md` の Q1〜Q7 を指す。

## 目的

マネジメントコンソールでは見えない古いロググループの過去ログを、確実に閲覧できるようにする（intent-statement）。今回のワークフローでは、1 週間以内に使える MVP を作る（feasibility-assessment、constraint-register の C-O2）。

## MVP の範囲（In Scope）

| # | できること | 出典 |
|---|------------|------|
| 1 | AWS のプロファイルとリージョンを切り替える | Q1 |
| 2 | 切り替えた権限で取得できるロググループを一覧から 1 つ選ぶ | Q1 |
| 3 | 時間範囲（開始・終了）を指定する | Q1 |
| 4 | 指定した時間範囲について、ログストリームを跨いだ結果を時刻順に閲覧する。時間範囲が広く対象ストリームが多くても、上限を設けずすべて取得する（時間がかかってもよい） | Q1、Q7 |
| 5 | 表示中のログを文字列の部分一致で絞り込む | Q2 |
| 6 | 取得済みログをディスクにキャッシュする。利用者が選んだ場合のみ有効で、既定は無効 | Q3、constraint-register C-R1 |

MVP の取得には、実現性評価の推奨どおり、終了時刻の指定・ページ終端の正しい判定・スロットリング時の待機と再試行を含める（feasibility-assessment の推奨事項 2）。

## MVP 後の範囲（Next）

優先順は次のとおり（Q4）。

1. **ロググループ全体の横断取得の高速化**：大量ストリーム・広い時間範囲の取得を速くする（並列化、キャッシュの活用など）。実現性評価で MVP 後に回した項目（feasibility-assessment）。
2. **Logs Insights 互換のクエリ**：Logs Insights のクエリ構文との互換を目指す（Q5）。取得済みのログに対してローカルで実行する（feasibility-assessment）。
3. **その他の候補**（MVP に含めなかったもの、Q3）：取得の進捗表示と中断、結果のファイル書き出し、JSON ログの整形表示。

## 対象外（Out of Scope）

| 項目 | 出典 |
|------|------|
| リアルタイムの追従表示（tail） | Q6 |
| ログの書き込み・削除・保持期間変更などの変更操作 | Q6 |
| Logs Insights や FilterLogEvents API の利用（取得は GetLogEvents を使う） | Q6、constraint-register C-T1 |
| Web ブラウザ版・サーバー版 | Q6、constraint-register C-T8 |
| 過去ログが見えなかった原因の調査 | constraint-register C-X1 |

## 範囲の判断で受け入れたこと

- **広い時間範囲は遅くなる。** 上限を設けずすべて取得する（Q7）一方、進捗表示と中断は MVP に含めない（Q3）。そのため、時間範囲が広いと、取得中に何も表示されない時間が数分以上続く可能性がある。これは MVP の範囲内で受け入れたトレードオフであり、MVP 後の最優先項目（横断取得の高速化、Q4）で改善する。
- **フィルターは部分一致のみ。** 正規表現や複数条件は MVP に含めない（Q2）。より高度な絞り込みは Logs Insights 互換のクエリでまとめて扱う（Q5）。

## 成功の判定（意図整理の成功指標との対応）

| 意図整理の成功指標（intent-statement） | MVP での扱い |
|----------------------------------------|--------------|
| マネコンで見えなかった古いロググループの過去ログが表示できる | 範囲 1〜4 で達成する |
| 指定した期間・ストリームのログを CLI より少ない操作で表示・フィルタできる | 範囲 1〜5 で達成する。比較基準は要件分析で決める |
| 数万〜数十万件でも実用的な時間でフィルタ結果が返る | 範囲 5（取得済みログの絞り込み）で達成する。「実用的な時間」の数値は要件分析で決める |

## 価値の流れ（Value Stream）

```mermaid
flowchart LR
  A["プロファイル・リージョンを選ぶ"] --> B["ロググループを一覧から選ぶ"]
  B --> C["時間範囲を指定する"]
  C --> D["ストリームを跨いで取得し、時刻順に表示する"]
  D --> E["部分一致で絞り込む"]
  D -. "任意" .-> F["ディスクにキャッシュする"]
```

<!-- Text fallback: プロファイル・リージョンを選ぶ → ロググループを一覧から選ぶ → 時間範囲を指定する → ストリームを跨いで取得し時刻順に表示する → 部分一致で絞り込む。取得結果は任意でディスクにキャッシュできる。 -->
