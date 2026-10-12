# Intent Backlog — ロググループ全体の取得の高速化

上流：`ideation/intent-capture/intent-statement.md`、`ideation/feasibility/feasibility-assessment.md`、`ideation/feasibility/constraint-register.md`

優先度は MoSCoW で分ける。Must は今回の版に欠かせないもの、Won't は今回は作らないもの。（Q1、Q4）

| ID | 項目 | MoSCoW | 依存 | 外す順（期限が厳しいとき） | 出典 |
|----|------|--------|------|----------------------------|------|
| B1 | 複数のストリームを同時に読んで、取得を速くする | Must | なし | 外さない | intent-statement、Q1 |
| B2 | 制限を受けたら引き下がる（使う量は上限の半分程度まで） | Must | B1 | 外さない | feasibility Q2、Q1 |
| B3 | 「3 本続いたらあきらめる」決まりの決め直し | Must | B1 | 外さない | feasibility Q4、Q1 |
| B4 | 進み具合の表示（取得にかかった時間を含む） | Must | B1 | 2 番目に外す | intent-statement、Q1、Q2、Q5 |
| B5 | 中断（取れた分は残し、中断と取れていない範囲を示す） | Must | B1 | 最初に外す | intent-statement、feasibility Q3、Q1、Q2 |
| B6 | 変更前の版との速さの比較（計測） | Won't | — | — | Q4 |
| — | Logs Insights 互換のクエリ | Won't | — | — | intent-statement |
| — | 利用者による速さの設定 | Won't | — | — | intent-statement |

## 依存関係

- B2・B3・B4・B5 は、同時に読む形（B1）を前提にする。（feasibility-assessment）
- B4 の「かかった時間」は、速くなったかどうかの判断にも使う。（Q5）

## 進める順

B1 → B2・B3 → B4 → B5（価値が先。速くする部分を先に作り、そのあと表示に進む）。（Q3）

## Assumptions & Open Questions

None.
