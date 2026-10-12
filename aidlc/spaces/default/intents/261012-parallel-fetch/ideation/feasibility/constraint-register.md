# Constraint Register — ロググループ全体の取得の高速化

上流：`ideation/intent-capture/intent-statement.md`

## Technical Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-T1 | 呼んでよいのは読み取りの 3 つ（DescribeLogGroups / DescribeLogStreams / GetLogEvents）だけ | 他の取得手段（まとめて検索する呼び出しなど）で速くすることはできない | project.md Forbidden、intent-statement |
| C-T2 | 東京リージョンのログを読む呼び出しの上限は、アカウント全体で 1 秒あたり 25 回。引き上げは申請できない | 速さの天井になる | AWS 公開資料「CloudWatch Logs quotas」、Q1 |
| C-T3 | ストリームの一覧の呼び出しの上限は、アカウント全体で 1 秒あたり 25 回 | 一覧は 1 ページずつ順に読むため、今回の作業で上限に近づくことはない見込み | AWS 公開資料「CloudWatch Logs quotas」 |
| C-T4 | ストリームの一覧は 1 ページずつ順にしか読めない | 一覧の部分は同時に読んで速くすることができない | feasibility-assessment |
| C-T5 | 表示は時刻順を保つ（今の動き） | 同時に読んでも、表示の順は変えない | intent-statement の「保つ」の考え方、今のアプリ |
| C-T6 | 利用者から見たキャッシュの動きは今と同じ（すべて成功した取得だけを残す） | 中断した取得はキャッシュに残さない | intent-statement、Q3 |

## Organizational Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-O1 | アプリが使う呼び出しは、上限の半分程度まで | 速さの天井は 1 秒あたり 12 回前後になる。他の利用者に余裕を残す | Q2 |
| C-O2 | 期限は 1 週間以内 | 今ある仕組みを生かし、作り込みは避ける | Q6 |
| C-O3 | 範囲と優先順位は開発者本人が決める。共有は PR と README 程度 | 承認待ちで止まることはない | intent-capture Q6 |
| C-O4 | 利用者が速さの設定を変えられるようにはしない | 同時に読む本数などはアプリが決める | intent-statement |
| C-O5 | 自動テストと CI からは実際の AWS に接続しない。速さの比較は開発者本人の手元で行う | 速さの確認は手作業になる。計測用のロググループはある | project.md Forbidden、team.md Walking Skeleton、Q5 |

## Regulatory Constraints

| ID | 制約 | 影響 | 出典 |
|----|------|------|------|
| C-R1 | 秘密の認証情報を、ログ・画面のエラー表示・キャッシュに書かない | 進み具合や中断の表示にも、認証情報を出さない | project.md Forbidden |
| C-R2 | テレメトリやクラッシュレポートを外部に送らない | 速さの計測結果を自動で外に送らない | project.md Forbidden |

## Out of Scope Constraints

| 項目 | 出典 |
|------|------|
| Logs Insights 互換のクエリ | intent-statement |
| 利用者が速さの設定を変えられるようにすること | intent-statement |
| 呼び出しの上限の引き上げ申請（ログを読む呼び出しは引き上げできない） | AWS 公開資料「CloudWatch Logs quotas」 |

## Assumptions & Open Questions

None.
