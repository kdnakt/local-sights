# Phase Check — Ideation → Inception

対象：`aidlc/spaces/default/intents/261017-insights-query/ideation/` 配下の成果物（intent-statement、stakeholder-map、scope-document、intent-backlog、initiative-brief、decision-log）。

## 1. 意図 → 範囲 → バックログの一貫性

| 意図（intent-statement） | 範囲（scope-document） | バックログ（intent-backlog） | 結果 |
|--------------------------|------------------------|------------------------------|------|
| 条件を組み合わせて絞り込みたい | B1（Must） | B1 | OK |
| 命令は `fields`・`filter`・`sort`・`limit` | B1・B2・B4（Must） | B1・B2・B4 | OK |
| クエリの中で期間とロググループを指定し、足りない分だけ追加で取得する | B5（Must） | B5 | OK |
| 今の部分一致の絞り込みは残す | 「変えないもの」 | —（変更しない項目） | OK |
| 集計（`stats`）は次の版以降 | 「範囲外」 | Won't | OK |
| クエリの保存・履歴、結果の書き出しは作らない | 「範囲外」 | Won't | OK |
| 成功指標：コンソールとの互換 | 「成功の判断」（B6 の範囲で同じ結果） | B6 | OK（確かめ方は initiative-brief で決定。D17〜D19） |
| 成功指標：速さの基準 | 「変えないもの」 | — | 注意：未測定（受け入れたリスク D15。要件で条件を決め、作る段階で測る） |
| 成功指標：macOS と Windows の両方で動く | 「変えないもの」 | — | OK |

範囲で足した項目（B3 の項目の種類、B6 の一覧、B7 のエラー、B8〜B10 の書く手助け）は、範囲の定義の回答（scope-definition-questions の Q1・Q7・Q8）に出典がある。

## 2. 範囲 → 実現性の裏付け

実現性の検討（feasibility）は、この計画では行っていない。各項目の裏付けは次のとおり。

| 範囲の項目 | 裏付け | 結果 |
|------------|--------|------|
| B1〜B4（手元でのクエリ実行） | 取得済みのログに対して手元で実行する。絞り込みは保持と別の部品にしてあり差し替えられる（project-description の前提メモ、261002 の domain-design） | OK（今のコードは Reverse Engineering で確かめる） |
| B5（追加の取得） | 読み取りの 3 つの呼び出しだけで行う（project.md の Forbidden） | OK |
| B6・B7 | 文書と画面の表示で、外部への依存はない | OK |
| B8〜B10 | 画面側の機能。時間が足りなければ次の版 | OK |
| 速さの基準 | 未測定 | 注意：受け入れたリスク（D15） |

## 3. 承認と記録

| 確認項目 | 結果 |
|----------|------|
| 意図を整理した | OK（intent-capture 承認済み） |
| 範囲を決めた | OK（scope-definition 承認済み） |
| リスクを確かめた | OK（approval-handoff Q1・Q1a。互換の確かめ方は Q4〜Q6 で決定） |
| 企画を進める判断 | Go（approval-handoff Q3） |
| 決定の記録 | decision-log（D1〜D21） |

## 結果

食い違いや出典のない項目はない。速さの基準が未測定である点だけを、受け入れたリスクとして次のフェーズに引き継ぐ。
